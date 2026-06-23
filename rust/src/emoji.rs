/// emoji.rs — Emoji 图文资源管理器
///
/// EmojiManager 管理三种注册模式的表情贴图：
/// a) register_from_rgba — 直接从 RGBA 像素数据注册
/// b) register_from_file — 从本地图片文件加载并解码
/// c) register_from_url — 从网络 URL 异步下载
///
/// 使用 lru::LruCache 做内存缓存，支持文件修改时间自动重载。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use std::time::SystemTime;

use lru::LruCache;

use crate::config::EmojiCacheConfig;
use crate::network;

// ============================================================
// Emoji 位图结构体
// ============================================================

/// Emoji 贴图位图数据
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EmojiBitmap {
    /// 表情唯一标识符
    pub id: String,
    /// 图像宽度（像素）
    pub width: u32,
    /// 图像高度（像素）
    pub height: u32,
    /// RGBA 像素数据（每个像素 4 字节，行优先顺序）
    pub rgba_data: Vec<u8>,
    /// 缓存大小（字节）
    pub cached_size_bytes: u64,
    /// 文件的最后修改时间（用于自动重载）
    file_modified: Option<SystemTime>,
    /// 文件路径（用于自动重载）
    file_path: Option<PathBuf>,
    /// 注册方式
    register_method: RegisterMethod,
}

/// 注册方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegisterMethod {
    Rgba,
    File,
    Url,
}

impl EmojiBitmap {
    /// 创建新的 Emoji 位图
    pub fn new(id: &str, width: u32, height: u32, rgba_data: Vec<u8>) -> Result<Self, String> {
        let expected_len = (width * height * 4) as usize;
        if rgba_data.len() != expected_len {
            return Err(format!(
                "RGBA 数据长度不匹配: 期望 {} 字节, 实际 {} 字节",
                expected_len, rgba_data.len()
            ));
        }

        let size_bytes = rgba_data.len() as u64;

        Ok(Self {
            id: id.to_string(),
            width,
            height,
            rgba_data,
            cached_size_bytes: size_bytes,
            file_modified: None,
            file_path: None,
            register_method: RegisterMethod::Rgba,
        })
    }

    /// 获取像素数据引用
    pub fn as_bytes(&self) -> &[u8] {
        &self.rgba_data
    }

    /// 计算像素数据大小
    pub fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

// ============================================================
// Emoji 管理器
// ============================================================

/// Emoji 表情管理器
///
/// 使用 LRU 缓存管理内存中的贴图数据，
/// 文件注册支持自动重载（基于修改时间）。
pub struct EmojiManager {
    /// LRU 缓存：id -> EmojiBitmap
    cache: LruCache<String, Arc<EmojiBitmap>>,
    /// 全量注册索引（包含缓存中可能被淘汰的，但保留注册信息用于文件重载）
    registry: HashMap<String, RegistryInfo>,
    /// 最大缓存字节数
    max_total_bytes: u64,
    /// 单个贴图最大字节数
    max_bitmap_bytes: u64,
    /// 当前缓存总字节数
    total_bytes: u64,
    /// 缓存配置
    config: EmojiCacheConfig,
    /// 下一个表情 ID 自增计数器
    next_id: u64,
}

/// 注册索引信息（用于文件重载）
#[derive(Debug, Clone)]
struct RegistryInfo {
    id: String,
    register_method: RegisterMethod,
    file_path: Option<PathBuf>,
    url: Option<String>,
    width: u32,
    height: u32,
}

impl EmojiManager {
    /// 创建新的 Emoji 管理器
    pub fn new(config: EmojiCacheConfig) -> Self {
        let max_entries = config.max_entries.min(1024).max(16) as usize;
        Self {
            cache: LruCache::new(max_entries.try_into().unwrap_or(64)),
            registry: HashMap::new(),
            max_total_bytes: config.max_total_cache_bytes,
            max_bitmap_bytes: config.max_bitmap_bytes,
            total_bytes: 0,
            config,
            next_id: 0,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(EmojiCacheConfig::default())
    }

    // ============================================================
    // 三种注册方式
    // ============================================================

    /// a) 直接从 RGBA 数据注册
    ///
    /// id: 表情唯一标识符
    /// width: 图像宽度
    /// height: 图像高度
    /// rgba_data: RGBA 像素数据
    pub fn register_from_rgba(
        &mut self,
        id: &str,
        width: u32,
        height: u32,
        rgba_data: &[u8],
    ) -> Result<(), String> {
        // 安全性校验
        if id.is_empty() || id.len() > 128 {
            return Err("Emoji ID 为空或过长（超过 128 字符）".to_string());
        }
        if id.contains('\0') {
            return Err("Emoji ID 包含 null 字符".to_string());
        }
        if width == 0 || height == 0 {
            return Err("图像尺寸无效（宽高不能为 0）".to_string());
        }
        if width > 1024 || height > 1024 {
            return Err("图像尺寸过大（最大 1024x1024）".to_string());
        }

        let expected_len = (width * height * 4) as usize;
        if rgba_data.len() != expected_len {
            return Err(format!(
                "RGBA 数据长度不匹配: 期望 {} 字节, 实际 {} 字节",
                expected_len, rgba_data.len()
            ));
        }

        // 检查单个贴图大小
        if rgba_data.len() as u64 > self.max_bitmap_bytes {
            return Err(format!(
                "贴图数据过大: {} 字节, 最大允许 {} 字节",
                rgba_data.len(),
                self.max_bitmap_bytes
            ));
        }

        // 检查总缓存容量
        let new_size = rgba_data.len() as u64;
        if self.total_bytes + new_size > self.max_total_bytes {
            // 尝试淘汰旧数据腾出空间
            self.evict_until(self.max_total_bytes - new_size);
        }

        let bitmap = EmojiBitmap::new(id, width, height, rgba_data.to_vec())?;

        // 更新缓存
        let size = bitmap.cached_size_bytes;
        self.cache.put(id.to_string(), Arc::new(bitmap));
        self.total_bytes += size;

        // 更新注册索引
        self.registry.insert(id.to_string(), RegistryInfo {
            id: id.to_string(),
            register_method: RegisterMethod::Rgba,
            file_path: None,
            url: None,
            width,
            height,
        });

        Ok(())
    }

    /// b) 从本地文件加载注册
    ///
    /// 使用 image crate 解码 PNG/JPEG/GIF 等格式。
    /// 自动追踪文件修改时间以实现重载。
    pub fn register_from_file(&mut self, id: &str, file_path: &str) -> Result<(), String> {
        // 安全性校验
        if id.is_empty() || id.len() > 128 {
            return Err("Emoji ID 无效".to_string());
        }
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("文件不存在: {}", file_path));
        }
        if !path.is_file() {
            return Err(format!("路径不是文件: {}", file_path));
        }
        // 限制文件大小（最大 32MB）
        let metadata = fs::metadata(path).map_err(|e| format!("读取文件元数据失败: {}", e))?;
        if metadata.len() > 32 * 1024 * 1024 {
            return Err("文件过大（超过 32MB）".to_string());
        }

        // 读取并解码图片
        let img = image::open(path).map_err(|e| format!("图片解码失败: {}: {}", file_path, e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let rgba_data = rgba.into_raw();

        let file_modified = metadata.modified().ok();

        // 检查大小
        if rgba_data.len() as u64 > self.max_bitmap_bytes {
            return Err("解码后的图像数据过大".to_string());
        }

        // 检查总容量
        let new_size = rgba_data.len() as u64;
        if self.total_bytes + new_size > self.max_total_bytes {
            self.evict_until(self.max_total_bytes - new_size);
        }

        let mut bitmap = EmojiBitmap::new(id, width, height, rgba_data)?;
        bitmap.file_modified = file_modified;
        bitmap.file_path = Some(path.to_path_buf());
        bitmap.register_method = RegisterMethod::File;

        let size = bitmap.cached_size_bytes;
        self.cache.put(id.to_string(), Arc::new(bitmap));
        self.total_bytes += size;

        self.registry.insert(id.to_string(), RegistryInfo {
            id: id.to_string(),
            register_method: RegisterMethod::File,
            file_path: Some(path.to_path_buf()),
            url: None,
            width,
            height,
        });

        Ok(())
    }

    /// c) 从网络 URL 注册
    ///
    /// 异步下载图片，解码后缓存。
    pub fn register_from_url(&mut self, id: &str, url: &str) -> Result<(), String> {
        // 安全性校验
        if id.is_empty() || id.len() > 128 {
            return Err("Emoji ID 无效".to_string());
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(format!("不支持的 URL 协议: {}", url));
        }
        if url.len() > 2048 {
            return Err("URL 过长".to_string());
        }

        // 下载图片数据
        let options = network::DownloadOptions::default();
        let image_data = network::download_image(url, &options, None)?;

        // 解码图片
        let img = image::load_from_memory(&image_data)
            .map_err(|e| format!("图片数据解码失败: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let rgba_data = rgba.into_raw();

        // 检查大小
        if rgba_data.len() as u64 > self.max_bitmap_bytes {
            return Err("解码后的图像数据过大".to_string());
        }

        // 检查总容量
        let new_size = rgba_data.len() as u64;
        if self.total_bytes + new_size > self.max_total_bytes {
            self.evict_until(self.max_total_bytes - new_size);
        }

        let bitmap = EmojiBitmap::new(id, width, height, rgba_data)?;

        let size = bitmap.cached_size_bytes;
        self.cache.put(id.to_string(), Arc::new(bitmap));
        self.total_bytes += size;

        self.registry.insert(id.to_string(), RegistryInfo {
            id: id.to_string(),
            register_method: RegisterMethod::Url,
            file_path: None,
            url: Some(url.to_string()),
            width,
            height,
        });

        Ok(())
    }

    // ============================================================
    // 核心查询接口
    // ============================================================

    /// a) 获取 Emoji 信息（宽、高、字节数）
    ///
    /// 返回 Option<(u32 width, u32 height, u64 bytes)>
    /// 对于文件注册的 Emoji，检查文件修改时间并自动重载
    pub fn get_emoji_info(&mut self, id: &str) -> Option<(u32, u32, u64)> {
        // 检查是否需要自动重载
        self.try_reload(id);

        self.cache.get(id).map(|bitmap| {
            (bitmap.width, bitmap.height, bitmap.cached_size_bytes)
        })
    }

    /// b) 拷贝 Emoji 像素数据到外部缓冲区
    ///
    /// 返回 true 表示拷贝成功，false 表示未找到或缓冲区不足
    pub fn copy_emoji_bitmap(&mut self, id: &str, buffer: &mut [u8], buffer_len: usize) -> bool {
        // 检查是否需要自动重载
        self.try_reload(id);

        if let Some(bitmap) = self.cache.get(id) {
            if buffer.len() < bitmap.rgba_data.len() || buffer_len < bitmap.rgba_data.len() {
                return false; // 缓冲区不足
            }
            let len = bitmap.rgba_data.len().min(buffer.len()).min(buffer_len);
            buffer[..len].copy_from_slice(&bitmap.rgba_data[..len]);
            true
        } else {
            false
        }
    }

    /// c) 获取 Emoji RGBA 数据拷贝
    ///
    /// 返回 Option<Vec<u8>>，完整像素数据
    pub fn get_emoji_bitmap_bytes(&mut self, id: &str) -> Option<Vec<u8>> {
        // 检查是否需要自动重载
        self.try_reload(id);

        self.cache.get(id).map(|bitmap| bitmap.rgba_data.clone())
    }

    /// 获取 Emoji 位图不可变引用
    pub fn get_emoji_ref(&mut self, id: &str) -> Option<Arc<EmojiBitmap>> {
        self.try_reload(id);
        self.cache.get(id).cloned()
    }

    // ============================================================
    // 缓存管理
    // ============================================================

    /// 清除所有 Emoji 缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.total_bytes = 0;
        self.registry.clear();
    }

    /// 移除指定 Emoji
    pub fn remove_emoji(&mut self, id: &str) -> bool {
        let removed = self.cache.pop(id);
        self.registry.remove(id);
        if let Some(ref bitmap) = removed {
            self.total_bytes = self.total_bytes.saturating_sub(bitmap.cached_size_bytes);
        }
        removed.is_some()
    }

    /// 获取当前缓存大小（字节）
    pub fn cache_size(&self) -> u64 {
        self.total_bytes
    }

    /// 获取缓存条目数
    pub fn cache_entries(&self) -> usize {
        self.cache.len()
    }

    /// 检查 Emoji 是否存在
    pub fn has_emoji(&mut self, id: &str) -> bool {
        self.cache.contains(id)
    }

    // ============================================================
    // 内部方法
    // ============================================================

    /// 尝试自动重载文件注册的 Emoji
    fn try_reload(&mut self, id: &str) {
        let should_reload = self.registry.get(id).map(|info| {
            if info.register_method == RegisterMethod::File {
                if let Some(ref file_path) = info.file_path {
                    if let Ok(metadata) = fs::metadata(file_path) {
                        if let Ok(modified) = metadata.modified() {
                            // 检查文件是否已被修改
                            if let Some(cached) = self.cache.get(id) {
                                return cached.file_modified.map(|t| modified != t).unwrap_or(true);
                            }
                            return true; // 缓存中不存在
                        }
                    }
                }
            }
            false
        }).unwrap_or(false);

        if should_reload {
            if let Some(info) = self.registry.get(id) {
                if let Some(ref file_path) = info.file_path {
                    // 从缓存中移除旧数据
                    if let Some(old) = self.cache.pop(id) {
                        self.total_bytes = self.total_bytes.saturating_sub(old.cached_size_bytes);
                    }

                    // 重新加载
                    if let Ok(img) = image::open(file_path) {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let data = rgba.into_raw();
                        if let Ok(metadata) = fs::metadata(file_path) {
                            let modified = metadata.modified().ok();
                            if let Ok(mut bitmap) = EmojiBitmap::new(id, w, h, data) {
                                bitmap.file_modified = modified;
                                bitmap.file_path = Some(file_path.clone());
                                bitmap.register_method = RegisterMethod::File;
                                let size = bitmap.cached_size_bytes;
                                self.cache.put(id.to_string(), Arc::new(bitmap));
                                self.total_bytes += size;
                            }
                        }
                    }
                }
            }
        }
    }

    /// 淘汰缓存直到总字节数 <= target
    fn evict_until(&mut self, target: u64) {
        while self.total_bytes > target && self.cache.len() > 1 {
            if let Some((_, bitmap)) = self.cache.pop_lru() {
                self.total_bytes = self.total_bytes.saturating_sub(bitmap.cached_size_bytes);
            } else {
                break;
            }
        }
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_from_rgba() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![255u8; 64 * 64 * 4]; // 64x64 白色像素

        assert!(mgr.register_from_rgba("test_emoji", 64, 64, &rgba).is_ok());
        assert!(mgr.has_emoji("test_emoji"));
    }

    #[test]
    fn test_register_invalid_size() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![255u8; 100]; // 数据长度不对
        let result = mgr.register_from_rgba("bad_emoji", 10, 10, &rgba);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_zero_dimensions() {
        let mut mgr = EmojiManager::with_default_config();
        let result = mgr.register_from_rgba("zero", 0, 0, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_emoji_info() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![128u8; 32 * 32 * 4]; // 32x32
        mgr.register_from_rgba("info_test", 32, 32, &rgba).unwrap();

        let info = mgr.get_emoji_info("info_test");
        assert!(info.is_some());
        let (w, h, bytes) = info.unwrap();
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        assert_eq!(bytes, (32 * 32 * 4) as u64);
    }

    #[test]
    fn test_get_emoji_info_not_found() {
        let mut mgr = EmojiManager::with_default_config();
        assert!(mgr.get_emoji_info("nonexistent").is_none());
    }

    #[test]
    fn test_copy_emoji_bitmap() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![1u8, 2, 3, 4]; // 1x1 像素
        mgr.register_from_rgba("copy_test", 1, 1, &rgba).unwrap();

        let mut buffer = vec![0u8; 4];
        let result = mgr.copy_emoji_bitmap("copy_test", &mut buffer, 4);
        assert!(result);
        assert_eq!(buffer, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_copy_emoji_bitmap_insufficient_buffer() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![255u8; 16 * 16 * 4];
        mgr.register_from_rgba("buf_test", 16, 16, &rgba).unwrap();

        let mut buffer = vec![0u8; 4]; // 太小了
        let result = mgr.copy_emoji_bitmap("buf_test", &mut buffer, 4);
        assert!(!result);
    }

    #[test]
    fn test_get_emoji_bitmap_bytes() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![0xAB; 8 * 8 * 4];
        mgr.register_from_rgba("bytes_test", 8, 8, &rgba).unwrap();

        let result = mgr.get_emoji_bitmap_bytes("bytes_test");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), rgba);
    }

    #[test]
    fn test_remove_emoji() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![255u8; 4 * 4 * 4];
        mgr.register_from_rgba("remove_test", 4, 4, &rgba).unwrap();

        assert!(mgr.has_emoji("remove_test"));
        assert!(mgr.remove_emoji("remove_test"));
        assert!(!mgr.has_emoji("remove_test"));
    }

    #[test]
    fn test_clear_cache() {
        let mut mgr = EmojiManager::with_default_config();
        let rgba = vec![255u8; 4 * 4 * 4];
        mgr.register_from_rgba("a", 4, 4, &rgba).unwrap();
        mgr.register_from_rgba("b", 4, 4, &rgba).unwrap();

        assert_eq!(mgr.cache_entries(), 2);
        mgr.clear_cache();
        assert_eq!(mgr.cache_entries(), 0);
        assert_eq!(mgr.cache_size(), 0);
    }

    #[test]
    fn test_cache_size_limits() {
        let mut mgr = EmojiManager::new(EmojiCacheConfig {
            max_total_cache_bytes: 64, // 非常小的容量
            ..Default::default()
        });

        // 注册第一个 32 字节的贴图（应成功）
        let rgba1 = vec![255u8; 32];
        assert!(mgr.register_from_rgba("a", 1, 8, &rgba1).is_ok());

        // 注册第二个 32 字节的贴图（应成功，总 64 字节）
        let rgba2 = vec![128u8; 32];
        assert!(mgr.register_from_rgba("b", 1, 8, &rgba2).is_ok());

        // 第三个贴图可能触发淘汰
        let rgba3 = vec![64u8; 32];
        let result = mgr.register_from_rgba("c", 1, 8, &rgba3);
        // 应成功（会淘汰旧的）
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_invalid_id() {
        let mut mgr = EmojiManager::with_default_config();
        let result = mgr.register_from_rgba("", 1, 1, &[255u8; 4]);
        assert!(result.is_err());
    }
}