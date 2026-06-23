/// network.rs — 异步 HTTP/HTTPS 客户端
///
/// 基于 ureq 同步 HTTP 客户端封装，提供图片下载功能。
/// 支持：
/// - 超时配置
/// - 自动重试（最多 3 次）
/// - 下载回调通知（start/success/fail）
/// - 内存 LRU + 磁盘双层缓存
/// - 缓存目录管理和过期清理

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use lru::LruCache;

// ============================================================
// 下载回调通知
// ============================================================

/// 下载事件
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadEvent {
    /// 开始下载
    Start,
    /// 下载成功
    Success,
    /// 下载失败
    Fail,
}

/// 下载回调函数类型
pub type DownloadCallback = Box<dyn Fn(DownloadEvent, &str) + Send + Sync>;

// ============================================================
// 全局缓存
// ============================================================

/// 网络下载缓存
struct NetworkCache {
    /// LRU 内存缓存：URL -> (下载时间, 数据)
    memory: LruCache<String, (u64, Vec<u8>)>,
    /// 磁盘缓存目录
    disk_cache_dir: Option<PathBuf>,
    /// 缓存有效期（秒）
    ttl_secs: u64,
}

impl NetworkCache {
    fn new(capacity: usize, ttl_secs: u64) -> Self {
        Self {
            memory: LruCache::new(capacity.try_into().unwrap_or(64)),
            disk_cache_dir: None,
            ttl_secs,
        }
    }

    /// 设置磁盘缓存目录
    fn set_disk_cache_dir(&mut self, dir: PathBuf) {
        self.disk_cache_dir = Some(dir);
    }

    /// 从缓存获取数据
    fn get(&mut self, url: &str) -> Option<Vec<u8>> {
        let now = current_time_secs();

        // 尝试内存缓存
        if let Some((time, data)) = self.memory.get(url) {
            if now.saturating_sub(*time) < self.ttl_secs {
                return Some(data.clone());
            }
        }

        // 尝试磁盘缓存
        if let Some(ref cache_dir) = self.disk_cache_dir {
            let cache_path = cache_path_from_url(cache_dir, url);
            if cache_path.exists() {
                if let Ok(metadata) = fs::metadata(&cache_path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            let file_time = duration.as_secs();
                            if now.saturating_sub(file_time) < self.ttl_secs {
                                // 磁盘缓存有效，读取并放入内存
                                if let Ok(data) = fs::read(&cache_path) {
                                    self.memory.put(url.to_string(), (now, data.clone()));
                                    return Some(data);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// 写入缓存
    fn put(&mut self, url: &str, data: Vec<u8>) {
        let now = current_time_secs();
        self.memory.put(url.to_string(), (now, data.clone()));

        // 写入磁盘缓存
        if let Some(ref cache_dir) = self.disk_cache_dir {
            let cache_path = cache_path_from_url(cache_dir, url);
            if let Some(parent) = cache_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&cache_path, &data);
        }
    }

    /// 清空缓存
    fn clear(&mut self) {
        self.memory.clear();

        // 清空磁盘缓存
        if let Some(ref cache_dir) = self.disk_cache_dir {
            let _ = fs::remove_dir_all(cache_dir);
            let _ = fs::create_dir_all(cache_dir);
        }
    }

    /// 获取缓存大小（字节数）
    fn size(&self) -> usize {
        self.memory.iter().map(|(_, (_, data))| data.len()).sum()
    }

    /// 清理过期缓存
    fn clean_expired(&mut self) {
        let now = current_time_secs();
        let expired_urls: Vec<String> = self.memory
            .iter()
            .filter(|(_, (time, _))| now.saturating_sub(*time) >= self.ttl_secs)
            .map(|(url, _)| url.clone())
            .collect();

        for url in expired_urls {
            self.memory.pop(&url);
        }
    }
}

/// 从 URL 生成缓存文件路径
fn cache_path_from_url(cache_dir: &Path, url: &str) -> PathBuf {
    // 使用 URL 的哈希作为文件名
    let hash = simple_hash(url);
    let ext = url_extension(url);
    cache_dir.join(format!("{}.{}", hash, ext))
}

/// 简单的字符串哈希（非加密）
fn simple_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// 从 URL 中提取文件扩展名
fn url_extension(url: &str) -> &str {
    if let Some(last_segment) = url.rsplit('/').next() {
        // 检查是否包含点号（说明有扩展名）
        if last_segment.contains('.') {
            if let Some(ext) = last_segment.rsplit('.').next() {
                if ext.len() <= 5 {
                    return ext;
                }
            }
        }
    }
    "png" // 默认扩展名
}

/// 获取当前时间戳（秒）
fn current_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ============================================================
// 全局网络缓存单例
// ============================================================

static NETWORK_CACHE: Lazy<RwLock<NetworkCache>> = Lazy::new(|| {
    RwLock::new(NetworkCache::new(256, 86400))
});

// ============================================================
// HTTP 客户端
// ============================================================

/// 下载选项
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// 连接超时（秒）
    pub connect_timeout_secs: u64,
    /// 读取超时（秒）
    pub read_timeout_secs: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// User-Agent 字符串
    pub user_agent: String,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 10,
            read_timeout_secs: 30,
            max_retries: 3,
            user_agent: "rs-barrage-engine/0.1".to_string(),
        }
    }
}

/// 下载图片
///
/// 从指定 URL 下载图片数据，自动使用缓存。
/// 支持最多 3 次自动重试。
///
/// 参数:
/// - url: 图片 URL
/// - options: 下载选项
///
/// 返回: Ok(Vec<u8>) 包含图片的 RGBA 或原始字节数据
pub fn download_image(
    url: &str,
    options: &DownloadOptions,
    callback: Option<&DownloadCallback>,
) -> Result<Vec<u8>, String> {
    // URL 安全性校验
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!("不支持的 URL 协议: {}", url));
    }
    if url.len() > 2048 {
        return Err("URL 过长（超过 2048 字符）".to_string());
    }

    // 尝试从缓存获取
    {
        let mut cache = NETWORK_CACHE.write();
        if let Some(data) = cache.get(url) {
            return Ok(data);
        }
    }

    // 回调：开始下载
    if let Some(cb) = callback {
        cb(DownloadEvent::Start, url);
    }

    // 构建 HTTP 客户端
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(options.connect_timeout_secs))
        .timeout_read(Duration::from_secs(options.read_timeout_secs))
        .user_agent(&options.user_agent)
        .build();

    // 带重试的下载
    let mut last_error = String::new();
    let max_retries = options.max_retries.min(3).max(1);

    for attempt in 0..max_retries {
        match agent.get(url).call() {
            Ok(response) => {
                let mut body = Vec::new();
                match response.into_reader().read_to_end(&mut body) {
                    Ok(_) => {
                        if body.is_empty() {
                            last_error = "下载内容为空".to_string();
                            continue;
                        }

                        // 写入缓存
                        {
                            let mut cache = NETWORK_CACHE.write();
                            cache.put(url, body.clone());
                        }

                        // 回调：成功
                        if let Some(cb) = callback {
                            cb(DownloadEvent::Success, url);
                        }

                        return Ok(body);
                    }
                    Err(e) => {
                        last_error = format!("读取响应体失败: {}", e);
                    }
                }
            }
            Err(e) => {
                last_error = format!("HTTP 请求失败 (尝试 {}/{}): {}", attempt + 1, max_retries, e);
            }
        }

        // 重试前等待（指数退避）
        if attempt < max_retries - 1 {
            std::thread::sleep(Duration::from_millis(500 * (attempt + 1) as u64));
        }
    }

    // 回调：失败
    if let Some(cb) = callback {
        cb(DownloadEvent::Fail, url);
    }

    Err(format!("图片下载失败（已重试 {} 次）: {}", max_retries, last_error))
}

// ============================================================
// 缓存管理接口
// ============================================================

/// 设置磁盘缓存目录
pub fn set_disk_cache_dir(dir: &str) -> Result<(), String> {
    let path = PathBuf::from(dir);

    // 安全性校验：确保是目录路径
    if dir.contains('\0') {
        return Err("缓存目录路径包含 null 字符".to_string());
    }

    // 尝试创建目录
    fs::create_dir_all(&path).map_err(|e| format!("创建缓存目录失败: {}", e))?;

    let mut cache = NETWORK_CACHE.write();
    cache.set_disk_cache_dir(path);
    Ok(())
}

/// 清空全部缓存
pub fn clear_cache() {
    let mut cache = NETWORK_CACHE.write();
    cache.clear();
}

/// 获取缓存大小（字节）
pub fn get_cache_size() -> u64 {
    let cache = NETWORK_CACHE.read();
    cache.size() as u64
}

/// 清理过期缓存
pub fn clean_expired_cache() {
    let mut cache = NETWORK_CACHE.write();
    cache.clean_expired();
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_extension() {
        assert_eq!(url_extension("https://example.com/image.png"), "png");
        assert_eq!(url_extension("https://example.com/photo.jpg"), "jpg");
        assert_eq!(url_extension("https://example.com/image"), "png");
    }

    #[test]
    fn test_cache_path_from_url() {
        let cache_dir = Path::new("/tmp/test_cache");
        let path = cache_path_from_url(cache_dir, "https://example.com/img.png");
        assert!(path.to_string_lossy().ends_with(".png"));
        assert!(path.to_string_lossy().starts_with("/tmp/test_cache"));
    }

    #[test]
    fn test_simple_hash() {
        let h1 = simple_hash("https://example.com/a.png");
        let h2 = simple_hash("https://example.com/b.png");
        assert_ne!(h1, h2); // 不同 URL 应有不同的哈希
        assert_eq!(h1.len(), 16); // 十六进制 64 位哈希 = 16 字符
    }

    #[test]
    fn test_invalid_url() {
        let options = DownloadOptions::default();
        let result = download_image("not-a-url", &options, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_url_too_long() {
        let long_url = "https://example.com/".to_string() + &"a".repeat(2048);
        let options = DownloadOptions::default();
        let result = download_image(&long_url, &options, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_operations() {
        let mut cache = NetworkCache::new(16, 3600);
        assert!(cache.get("https://test.com/img.png").is_none());

        cache.put("https://test.com/img.png", vec![1, 2, 3, 4]);
        assert_eq!(cache.get("https://test.com/img.png"), Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn test_clean_expired_cache() {
        // 使用 ttl = 3600 使缓存保留 1 小时
        let mut cache = NetworkCache::new(16, 3600);
        cache.put("https://test.com/img.png", vec![1, 2, 3]);
        assert!(cache.get("https://test.com/img.png").is_some());

        cache.clean_expired();
        // 1 小时内不会过期
        assert!(cache.get("https://test.com/img.png").is_some());
    }
}