/// barrage.rs — 弹幕对象与对象池管理
///
/// 定义弹幕核心数据结构 BarrageObject，包含文本内容、特效配置引用、
/// 轨道类型、位置/速度/颜色/字号/透明度/生命周期等字段。
/// 使用 BarrageObjectPool 实现对象预分配和空闲索引复用，
/// 减少运行时内存分配开销。

use crate::config::{TextEffectConfig, TrackMode};
use std::sync::Arc;

// ============================================================
// Emoji 片段：用于图文混排
// ============================================================

/// 图文混排中的单个片段，可以是文字或 Emoji 贴图
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EmojiSegment {
    /// true=表情贴图, false=纯文本
    pub is_emoji: bool,
    /// 文本内容（当 is_emoji=false 时有效）
    pub text: String,
    /// Emoji ID（当 is_emoji=true 时有效）
    pub emoji_id: String,
}

impl EmojiSegment {
    /// 创建纯文本片段
    pub fn text(text: &str) -> Self {
        Self {
            is_emoji: false,
            text: text.to_string(),
            emoji_id: String::new(),
        }
    }

    /// 创建 Emoji 贴图片段
    pub fn emoji(emoji_id: &str) -> Self {
        Self {
            is_emoji: true,
            text: String::new(),
            emoji_id: emoji_id.to_string(),
        }
    }
}

// ============================================================
// 弹幕对象
// ============================================================

/// 弹幕对象结构体，表示屏幕上的单个弹幕
///
/// 包含完整的显示属性和生命周期信息。
/// #[repr(C)] 导出以供 FFI 使用。
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BarrageObject {
    /// 弹幕唯一标识符（自增 ID）
    pub id: u64,

    // ===== 文本内容 =====
    /// 弹幕文本（纯文本模式，仅当 segments 为空时使用）
    pub text: String,
    /// 图文混排片段列表（支持文字+Emoji混合）
    pub segments: Vec<EmojiSegment>,

    // ===== 特效配置 =====
    /// 特效配置引用（None 表示使用全局默认特效）
    pub effect_config: Option<Arc<TextEffectConfig>>,

    // ===== 轨道与位置 =====
    /// 轨道模式
    pub track_mode: TrackMode,
    /// 所在轨道索引
    pub track_index: u32,
    /// 当前 X 坐标（像素）
    pub x: f32,
    /// 当前 Y 坐标（像素）
    pub y: f32,
    /// 初始 X 坐标
    pub start_x: f32,
    /// 初始 Y 坐标
    pub start_y: f32,

    // ===== 运动属性 =====
    /// X 方向速度（像素/秒）
    pub speed_x: f32,
    /// Y 方向速度（像素/秒）
    pub speed_y: f32,

    // ===== 外观属性 =====
    /// 弹幕颜色（RGBA u32 编码）
    pub color_rgba: u32,
    /// 字号（像素）
    pub font_size: u32,
    /// 透明度（0.0~1.0）
    pub alpha: f32,
    /// 弹幕宽度（像素，由渲染器更新）
    pub width: u32,
    /// 弹幕高度（像素，由渲染器更新）
    pub height: u32,

    // ===== 生命周期 =====
    /// 弹幕持续时间（毫秒）
    pub duration_ms: u32,
    /// 弹幕创建时间戳（引擎时间，毫秒）
    pub created_at_ms: u64,
    /// 弹幕已存活时间（毫秒）
    pub elapsed_ms: u64,
    /// 是否存活（false=已过期待回收）
    pub alive: bool,
}

impl BarrageObject {
    /// 创建新的弹幕对象（使用全局默认特效）
    pub fn new(
        id: u64,
        text: &str,
        track_mode: TrackMode,
        track_index: u32,
        color_rgba: u32,
        font_size: u32,
        alpha: f32,
        duration_ms: u32,
        created_at_ms: u64,
    ) -> Self {
        Self {
            id,
            text: text.to_string(),
            segments: Vec::new(),
            effect_config: None, // None = 使用全局默认
            track_mode,
            track_index,
            x: 0.0,
            y: 0.0,
            start_x: 0.0,
            start_y: 0.0,
            speed_x: 0.0,
            speed_y: 0.0,
            color_rgba,
            font_size,
            alpha: alpha.clamp(0.0, 1.0),
            width: 0,
            height: 0,
            duration_ms: duration_ms.max(100),
            created_at_ms,
            elapsed_ms: 0,
            alive: true,
        }
    }

    /// 创建带独立特效配置的弹幕
    pub fn new_with_effect(
        id: u64,
        text: &str,
        segments: Vec<EmojiSegment>,
        track_mode: TrackMode,
        track_index: u32,
        color_rgba: u32,
        font_size: u32,
        alpha: f32,
        duration_ms: u32,
        created_at_ms: u64,
        effect_config: Option<Arc<TextEffectConfig>>,
    ) -> Self {
        Self {
            id,
            text: text.to_string(),
            segments,
            effect_config,
            track_mode,
            track_index,
            x: 0.0,
            y: 0.0,
            start_x: 0.0,
            start_y: 0.0,
            speed_x: 0.0,
            speed_y: 0.0,
            color_rgba,
            font_size,
            alpha: alpha.clamp(0.0, 1.0),
            width: 0,
            height: 0,
            duration_ms: duration_ms.max(100),
            created_at_ms,
            elapsed_ms: 0,
            alive: true,
        }
    }

    /// 更新弹幕状态（逐帧调用）
    /// delta_ms: 距上一帧的毫秒数
    /// speed_multiplier: 全局速度倍率
    pub fn update(&mut self, delta_ms: u64, speed_multiplier: f32) {
        if !self.alive {
            return;
        }

        // 更新时间
        self.elapsed_ms += delta_ms;

        // 检查是否过期
        if self.elapsed_ms >= self.duration_ms as u64 {
            self.alive = false;
            return;
        }

        // 更新位置（仅滚动模式需要移动）
        let delta_secs = delta_ms as f32 / 1000.0 * speed_multiplier;
        self.x += self.speed_x * delta_secs;
        self.y += self.speed_y * delta_secs;
    }

    /// 获取弹幕生命进度（0.0~1.0）
    pub fn progress(&self) -> f32 {
        if self.duration_ms == 0 {
            return 1.0;
        }
        (self.elapsed_ms as f32 / self.duration_ms as f32).clamp(0.0, 1.0)
    }

    /// 获取当前 alpha（考虑生命周期的淡出效果）
    pub fn current_alpha(&self, global_alpha: f32) -> f32 {
        let progress = self.progress();
        // 最后 10% 时间淡出
        let fade_alpha = if progress > 0.9 {
            (1.0 - progress) / 0.1
        } else {
            1.0
        };
        (self.alpha * global_alpha * fade_alpha).clamp(0.0, 1.0)
    }

    /// 获取弹幕矩形包围盒 (x, y, w, h)
    pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.width as f32, self.height as f32)
    }

    /// 重置对象状态（对象池回收后复用）
    pub fn reset(&mut self) {
        self.text.clear();
        self.segments.clear();
        self.effect_config = None;
        self.track_mode = TrackMode::Scroll;
        self.track_index = 0;
        self.x = 0.0;
        self.y = 0.0;
        self.start_x = 0.0;
        self.start_y = 0.0;
        self.speed_x = 0.0;
        self.speed_y = 0.0;
        self.color_rgba = 0xFFFFFFFF;
        self.font_size = 28;
        self.alpha = 1.0;
        self.width = 0;
        self.height = 0;
        self.duration_ms = 5000;
        self.created_at_ms = 0;
        self.elapsed_ms = 0;
        self.alive = false;
    }
}

// ============================================================
// 弹幕对象池
// ============================================================

/// 弹幕对象池，预分配 Vec + 空闲索引栈实现高效复用
///
/// 避免频繁的堆分配/释放，提升高并发场景下的性能。
pub struct BarrageObjectPool {
    /// 预分配的弹幕对象存储
    objects: Vec<Option<BarrageObject>>,
    /// 空闲索引栈（从栈顶取，LIFO 方式复用）
    free_indices: Vec<usize>,
    /// 当前已分配的对象数量
    allocated_count: usize,
    /// 扩容步长
    grow_step: u32,
}

impl BarrageObjectPool {
    /// 创建新的对象池，预分配 capacity 个槽位
    pub fn new(capacity: u32, grow_step: u32) -> Self {
        let cap = capacity.max(32) as usize;
        let step = grow_step.max(16).min(512) as usize;
        let mut pool = Self {
            objects: Vec::with_capacity(cap),
            free_indices: Vec::with_capacity(cap),
            allocated_count: 0,
            grow_step: step as u32,
        };

        // 预填入空值
        for i in 0..cap {
            pool.objects.push(None);
            pool.free_indices.push(i);
        }

        pool
    }

    /// 从对象池分配一个弹幕对象
    /// 返回分配的索引和可变引用；如果池满且无法扩容则返回 None
    pub fn allocate(&mut self, obj: BarrageObject) -> Option<(usize, &mut BarrageObject)> {
        // 如果空闲栈为空，尝试扩容
        if self.free_indices.is_empty() {
            self.grow();
        }

        // 从空闲栈顶取一个索引
        if let Some(idx) = self.free_indices.pop() {
            self.objects[idx] = Some(obj);
            self.allocated_count += 1;
            // 返回可变引用（安全性由调用者保证，在同一线程中使用）
            let obj_ref = self.objects[idx].as_mut().unwrap();
            Some((idx, obj_ref))
        } else {
            None // 池满且无法扩容
        }
    }

    /// 释放弹幕对象，将索引回收至空闲栈
    pub fn deallocate(&mut self, idx: usize) {
        if idx >= self.objects.len() {
            return; // 索引越界，安全忽略
        }

        if let Some(ref mut obj) = self.objects[idx] {
            obj.reset(); // 重置状态
        }

        self.objects[idx] = None;
        self.free_indices.push(idx);
        self.allocated_count = self.allocated_count.saturating_sub(1);
    }

    /// 获取指定索引的可变引用
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut BarrageObject> {
        if idx >= self.objects.len() {
            return None;
        }
        self.objects[idx].as_mut()
    }

    /// 获取指定索引的不可变引用
    pub fn get(&self, idx: usize) -> Option<&BarrageObject> {
        if idx >= self.objects.len() {
            return None;
        }
        self.objects[idx].as_ref()
    }

    /// 获取所有存活对象的索引及其引用
    pub fn iter_alive(&self) -> impl Iterator<Item = (usize, &BarrageObject)> {
        self.objects.iter().enumerate().filter_map(|(idx, obj)| {
            if let Some(ref o) = obj {
                if o.alive {
                    return Some((idx, o));
                }
            }
            None
        })
    }

    /// 获取所有存活对象的可变引用
    pub fn iter_alive_mut(&mut self) -> impl Iterator<Item = (usize, &mut BarrageObject)> {
        self.objects.iter_mut().enumerate().filter_map(|(idx, obj)| {
            if let Some(ref mut o) = obj {
                if o.alive {
                    return Some((idx, o));
                }
            }
            None
        })
    }

    /// 回收所有过期弹幕
    pub fn reap_expired(&mut self) {
        let mut to_remove = Vec::new();
        for (idx, obj) in self.objects.iter().enumerate() {
            if let Some(ref o) = obj {
                if !o.alive {
                    to_remove.push(idx);
                }
            }
        }
        for idx in to_remove {
            self.deallocate(idx);
        }
    }

    /// 获取当前存活对象数量
    pub fn alive_count(&self) -> usize {
        self.objects.iter().filter_map(|o| o.as_ref()).filter(|o| o.alive).count()
    }

    /// 获取总容量
    pub fn capacity(&self) -> usize {
        self.objects.len()
    }

    /// 获取空闲索引数
    pub fn free_count(&self) -> usize {
        self.free_indices.len()
    }

    /// 扩容对象池
    fn grow(&mut self) {
        let current_len = self.objects.len();
        let grow_size = self.grow_step as usize;
        // 扩大容量（上限 8192）
        if current_len >= 8192 {
            return;
        }
        let new_size = (current_len + grow_size).min(8192);
        for i in current_len..new_size {
            self.objects.push(None);
            self.free_indices.push(i);
        }
    }

    /// 清空对象池
    pub fn clear(&mut self) {
        self.free_indices.clear();
        self.allocated_count = 0;
        let cap = self.objects.len();
        for i in 0..cap {
            self.objects[i] = None;
            self.free_indices.push(i);
        }
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TrackMode;

    #[test]
    fn test_barrage_object_creation() {
        let obj = BarrageObject::new(
            1,
            "测试弹幕",
            TrackMode::Scroll,
            0,
            0xFFFFFFFF,
            28,
            1.0,
            5000,
            0,
        );
        assert_eq!(obj.id, 1);
        assert_eq!(obj.text, "测试弹幕");
        assert!(obj.alive);
        assert!(obj.effect_config.is_none());
    }

    #[test]
    fn test_barrage_object_lifetime() {
        let mut obj = BarrageObject::new(
            1, "test", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 1000, 0,
        );
        assert!(obj.alive);
        obj.update(1500, 1.0);
        assert!(!obj.alive); // 超过 1000ms 应过期
    }

    #[test]
    fn test_object_pool_allocate_deallocate() {
        let mut pool = BarrageObjectPool::new(32, 16);
        let initial_free = pool.free_count();

        let obj = BarrageObject::new(1, "test", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
        let (idx, _) = pool.allocate(obj).unwrap();
        assert!(idx < 32);
        assert_eq!(pool.alive_count(), 1);
        assert_eq!(pool.free_count(), initial_free - 1);

        pool.deallocate(idx);
        assert_eq!(pool.alive_count(), 0);
    }

    #[test]
    fn test_object_pool_reuse() {
        let mut pool = BarrageObjectPool::new(32, 16);

        let obj1 = BarrageObject::new(1, "first", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
        let (idx1, _) = pool.allocate(obj1).unwrap();

        pool.deallocate(idx1);

        let obj2 = BarrageObject::new(2, "second", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
        let (idx2, _) = pool.allocate(obj2).unwrap();

        // 应复用 idx1
        assert_eq!(idx1, idx2);
    }

    #[test]
    fn test_barrage_progress() {
        let mut obj = BarrageObject::new(1, "test", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 1000, 0);
        assert!((obj.progress() - 0.0).abs() < 0.01);
        obj.update(500, 1.0);
        assert!((obj.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_emoji_segment() {
        let seg_text = EmojiSegment::text("hello");
        assert!(!seg_text.is_emoji);
        assert_eq!(seg_text.text, "hello");

        let seg_emoji = EmojiSegment::emoji("emoji_001");
        assert!(seg_emoji.is_emoji);
        assert_eq!(seg_emoji.emoji_id, "emoji_001");
    }
}