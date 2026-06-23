/// core.rs — Core 全局调度器
///
/// BarrageCore 整合所有模块的全局调度器，是弹幕引擎的核心入口。
///
/// 功能：
/// - 管理 TrackManager、TimelineController、FilterEngine、EmojiManager、Renderer、ObjectPool
/// - add_barrage：从外部接收弹幕（使用 crossbeam 无锁队列缓冲）
/// - update：每帧更新所有弹幕状态
/// - render：输出 RGBA 像素缓冲区
/// - seek、pause、resume、clear、set_speed、set_filter
/// - parking_lot::RwLock 保护共享状态

use std::sync::Arc;

use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use parking_lot::RwLock;

use crate::barrage::{BarrageObject, BarrageObjectPool, EmojiSegment};
use crate::config::{
    BarrageFilterConfig, BarrageGlobalConfig, BarrageTrackConfig,
    TextEffectConfig, TrackMode,
};
use crate::emoji::EmojiManager;
use crate::filter::{FilterEngine, FilterResult};
use crate::renderer::{RenderOptions, Renderer};
use crate::timeline::TimelineController;
use crate::track::TrackManager;

// ============================================================
// 入站弹幕消息
// ============================================================

/// 从外部传入的弹幕消息
pub struct BarrageMessage {
    /// 弹幕文本
    pub text: String,
    /// 图文混排片段
    pub segments: Vec<EmojiSegment>,
    /// 轨道模式
    pub track_type: TrackMode,
    /// 颜色（RGBA）
    pub color_rgba: u32,
    /// 字号
    pub font_size: u32,
    /// 透明度
    pub alpha: f32,
    /// 持续时间（毫秒）
    pub duration_ms: u32,
    /// 特效配置（None = 使用全局）
    pub effect_config: Option<Arc<TextEffectConfig>>,
}

// ============================================================
// Core 全局调度器
// ============================================================

/// 弹幕引擎核心调度器
///
/// 整合所有子系统，提供统一的调度接口。
/// 使用 RwLock 保护内部状态，支持多线程读取。
pub struct BarrageCore {
    /// 轨道管理器
    pub track_manager: RwLock<TrackManager>,
    /// 时间轴控制器
    pub timeline: RwLock<TimelineController>,
    /// 过滤引擎
    pub filter: RwLock<FilterEngine>,
    /// Emoji 管理器
    pub emoji_manager: RwLock<EmojiManager>,
    /// 渲染器
    pub renderer: RwLock<Renderer>,
    /// 弹幕对象池
    pub pool: RwLock<BarrageObjectPool>,
    /// 入站弹幕无锁队列（发送端）
    barrage_sender: Sender<BarrageMessage>,
    /// 入站弹幕无锁队列（接收端）
    barrage_receiver: Receiver<BarrageMessage>,
    /// 全局配置
    global_config: BarrageGlobalConfig,
    /// 全局特效配置
    global_effect: Option<Arc<TextEffectConfig>>,
    /// 自增 ID 计数器
    next_id: RwLock<u64>,
    /// 渲染选项
    render_options: RwLock<RenderOptions>,
}

impl BarrageCore {
    /// 创建弹幕引擎核心
    ///
    /// 使用全局配置初始化所有子系统。
    pub fn new(config: BarrageGlobalConfig) -> Self {
        let mut validated_config = config;
        validated_config.validate();

        // 创建无锁队列
        let (tx, rx) = channel::unbounded();

        // 创建轨道管理器
        let track_manager = if !validated_config.track_configs.is_null()
            && validated_config.track_configs_len > 0
        {
            let len = validated_config.track_configs_len.min(100) as usize;
            let configs: Vec<BarrageTrackConfig> = unsafe {
                (0..len)
                    .map(|i| {
                        let mut cfg = *validated_config.track_configs.add(i);
                        cfg.validate();
                        cfg
                    })
                    .collect()
            };
            TrackManager::new_with_configs(
                &configs,
                validated_config.canvas_width,
                validated_config.canvas_height,
                validated_config.allow_overlap,
            )
        } else {
            TrackManager::new(
                validated_config.track_count,
                validated_config.canvas_width,
                validated_config.canvas_height,
                validated_config.allow_overlap,
                40,
                10,
                200.0,
            )
        };

        // 创建过滤引擎
        let filter = FilterEngine::with_default_config();

        // 创建 Emoji 管理器
        let emoji_manager = EmojiManager::with_default_config();

        // 创建渲染器
        let renderer = Renderer::new(
            validated_config.canvas_width,
            validated_config.canvas_height,
        );

        // 创建对象池
        let pool = BarrageObjectPool::new(
            validated_config.pool_capacity,
            validated_config.pool_grow_step,
        );

        Self {
            track_manager: RwLock::new(track_manager),
            timeline: RwLock::new(TimelineController::new()),
            filter: RwLock::new(filter),
            emoji_manager: RwLock::new(emoji_manager),
            renderer: RwLock::new(renderer),
            pool: RwLock::new(pool),
            barrage_sender: tx,
            barrage_receiver: rx,
            global_config: validated_config,
            global_effect: None,
            next_id: RwLock::new(1),
            render_options: RwLock::new(RenderOptions::default()),
        }
    }

    /// 使用默认配置创建引擎
    pub fn with_default_config() -> Self {
        Self::new(BarrageGlobalConfig::default())
    }

    // ============================================================
    // 弹幕管理
    // ============================================================

    /// 从外部添加弹幕（通过无锁队列异步接收）
    ///
    /// 此方法将弹幕消息发送到无锁队列，在 update 时统一处理。
    pub fn add_barrage(&self, msg: BarrageMessage) {
        let _ = self.barrage_sender.send(msg);
    }

    /// 批量添加弹幕的便捷方法
    pub fn send_barrage(
        &self,
        text: &str,
        track_type: TrackMode,
        color_rgba: u32,
        font_size: u32,
        alpha: f32,
        duration_ms: u32,
        effect_config: Option<Arc<TextEffectConfig>>,
    ) {
        self.add_barrage(BarrageMessage {
            text: text.to_string(),
            segments: Vec::new(),
            track_type,
            color_rgba,
            font_size,
            alpha: alpha.clamp(0.0, 1.0),
            duration_ms,
            effect_config,
        });
    }

    // ============================================================
    // 每帧更新
    // ============================================================

    /// 更新引擎状态（每帧调用）
    ///
    /// 执行以下操作：
    /// 1. 从无锁队列接收所有缓冲弹幕
    /// 2. 对弹幕执行过滤
    /// 3. 将弹幕分配到轨道
    /// 4. 更新时间轴
    /// 5. 更新所有轨道中的弹幕状态（位置、生命周期）
    /// 6. 回收过期弹幕
    ///
    /// delta_ms: 距上一帧的实际时间增量（毫秒）
    pub fn update(&self, delta_ms: u64) {
        // 1. 处理时间轴（使用指定的 delta_ms 推进，避免依赖墙钟时间）
        let engine_delta = {
            let mut tl = self.timeline.write();
            tl.advance_by(delta_ms)
        };

        // 如果引擎时间没有前进（暂停中），仍然处理缓冲队列但不更新弹幕
        let effective_delta = if engine_delta > 0 { engine_delta } else { delta_ms };

        // 获取速度倍率
        let speed = {
            let tl = self.timeline.read();
            tl.speed()
        };

        // 获取引擎时间
        let engine_time_ms = {
            let tl = self.timeline.read();
            tl.engine_time_ms()
        };

        // 2. 从无锁队列接收所有缓冲弹幕
        let mut messages = Vec::new();
        loop {
            match self.barrage_receiver.try_recv() {
                Ok(msg) => messages.push(msg),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // 通道已断开，退出循环
                    break;
                }
            }
        }

        // 3. 处理每条弹幕消息
        for msg in messages {
            // 过滤
            let temp_obj = BarrageObject::new(
                0, // 临时 ID
                &msg.text,
                msg.track_type,
                0,
                msg.color_rgba,
                msg.font_size,
                msg.alpha,
                msg.duration_ms,
                engine_time_ms,
            );

            let filter_result = {
                let filter = self.filter.read();
                filter.filter(&temp_obj)
            };

            if filter_result != FilterResult::Pass {
                continue; // 被过滤拦截
            }

            // 分配对象池索引
            let next_id = {
                let mut id_counter = self.next_id.write();
                let id = *id_counter;
                *id_counter += 1;
                id
            };

            let obj = BarrageObject::new_with_effect(
                next_id,
                &msg.text,
                msg.segments,
                msg.track_type,
                0,
                msg.color_rgba,
                msg.font_size,
                msg.alpha,
                msg.duration_ms,
                engine_time_ms,
                msg.effect_config,
            );

            // 分配弹幕对象
            let pool_idx = {
                let mut pool = self.pool.write();
                if let Some((idx, obj_ref)) = pool.allocate(obj) {
                    // 对弹幕应用全局调节
                    {
                        let filter = self.filter.read();
                        filter.apply_global_adjustments(obj_ref);
                    }
                    Some(idx)
                } else {
                    None
                }
            };

            // 将弹幕分发到轨道
            if let Some(pool_idx) = pool_idx {
                let mut tm = self.track_manager.write();
                let _ = tm.add_barrage(
                    &mut *self.pool.write(),
                    pool_idx,
                    0, // 默认轨道
                    engine_time_ms,
                );
            }
        }

        // 4. 更新所有轨道中的弹幕
        {
            let mut tm = self.track_manager.write();
            let mut pool = self.pool.write();
            tm.update(&mut pool, effective_delta, speed, engine_time_ms);
        }

        // 5. 回收过期弹幕
        {
            let mut pool = self.pool.write();
            pool.reap_expired();
        }
    }

    // ============================================================
    // 渲染
    // ============================================================

    /// 渲染当前帧到 RGBA 缓冲区
    ///
    /// buffer: 输出缓冲区（必须足够大以容纳 width * height * 4 字节）
    /// width: 输出宽度
    /// height: 输出高度
    ///
    /// 返回渲染的弹幕数量，失败返回 -1
    pub fn render(&self, buffer: &mut [u8], width: u32, height: u32) -> i32 {
        // 检查缓冲区大小
        let expected_size = (width * height * 4) as usize;
        if buffer.len() < expected_size {
            return -1;
        }

        // 获取所有存活弹幕并执行渲染（在同一个作用域内保持池锁定）
        let tm = self.track_manager.read();
        let pool = self.pool.read();
        let barrages = tm.get_all_barrages(&pool);

        // 执行渲染
        let mut renderer = self.renderer.write();

        // 如果尺寸不一致，调整渲染器
        let (cw, ch) = renderer.dimensions();
        if cw != width || ch != height {
            renderer.resize(width, height);
        }

        // 应用全局特效配置（如果有）
        if let Some(ref effect) = self.global_effect {
            renderer.set_global_effect(Some(effect.clone()));
        }

        let count = renderer.render(&barrages, &mut *self.emoji_manager.write());

        // 拷贝到输出缓冲区
        renderer.copy_frame(buffer);

        count as i32
    }

    // ============================================================
    // 控制接口
    // ============================================================

    /// 跳转到指定时间点（毫秒）
    pub fn seek(&self, time_ms: u64) {
        let mut tl = self.timeline.write();
        tl.seek(time_ms);
    }

    /// 暂停播放
    pub fn pause(&self) {
        let mut tl = self.timeline.write();
        tl.pause();
    }

    /// 恢复播放
    pub fn resume(&self) {
        let mut tl = self.timeline.write();
        tl.resume();
    }

    /// 清空所有弹幕
    pub fn clear(&self) {
        // 清空轨道
        {
            let mut tm = self.track_manager.write();
            let mut pool = self.pool.write();
            tm.clear(&mut pool);
        }

        // 清空时间轴
        {
            let mut tl = self.timeline.write();
            tl.clear();
        }

        // 清空消息队列
        loop {
            match self.barrage_receiver.try_recv() {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    }

    /// 设置播放速度
    pub fn set_speed(&self, speed: f32) {
        let mut tl = self.timeline.write();
        tl.set_speed(speed);
    }

    /// 设置过滤配置
    pub fn set_filter(&self, config: BarrageFilterConfig) {
        let mut filter = self.filter.write();
        filter.set_config(config);
    }

    // ============================================================
    // 特效配置
    // ============================================================

    /// 设置全局特效配置
    pub fn set_global_effect(&mut self, effect: Option<Arc<TextEffectConfig>>) {
        self.global_effect = effect;
    }

    /// 获取全局特效配置引用
    pub fn global_effect(&self) -> Option<&Arc<TextEffectConfig>> {
        self.global_effect.as_ref()
    }

    // ============================================================
    // Emoji 管理
    // ============================================================

    /// 注册 RGBA Emoji
    pub fn register_emoji_rgba(
        &self,
        id: &str,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let mut em = self.emoji_manager.write();
        em.register_from_rgba(id, width, height, rgba_data)
    }

    /// 从文件注册 Emoji
    pub fn register_emoji_file(&self, id: &str, file_path: &str) -> Result<(), String> {
        let mut em = self.emoji_manager.write();
        em.register_from_file(id, file_path)
    }

    /// 从 URL 注册 Emoji
    pub fn register_emoji_url(&self, id: &str, url: &str) -> Result<(), String> {
        let mut em = self.emoji_manager.write();
        em.register_from_url(id, url)
    }

    /// 获取 Emoji 信息
    pub fn get_emoji_info(&self, id: &str) -> Option<(u32, u32, u64)> {
        let mut em = self.emoji_manager.write();
        em.get_emoji_info(id)
    }

    /// 拷贝 Emoji 像素到缓冲区
    pub fn copy_emoji_bitmap(&self, id: &str, buffer: &mut [u8], buffer_len: usize) -> bool {
        let mut em = self.emoji_manager.write();
        em.copy_emoji_bitmap(id, buffer, buffer_len)
    }

    /// 获取 Emoji 像素数据
    pub fn get_emoji_bitmap_bytes(&self, id: &str) -> Option<Vec<u8>> {
        let mut em = self.emoji_manager.write();
        em.get_emoji_bitmap_bytes(id)
    }

    /// 清除 Emoji 缓存
    pub fn clear_emoji_cache(&self) {
        let mut em = self.emoji_manager.write();
        em.clear_cache();
    }

    /// 移除指定 Emoji
    pub fn remove_emoji(&self, id: &str) -> bool {
        let mut em = self.emoji_manager.write();
        em.remove_emoji(id)
    }

    /// 获取缓存大小
    pub fn get_emoji_cache_size(&self) -> u64 {
        let em = self.emoji_manager.read();
        em.cache_size()
    }

    // ============================================================
    // 过滤器管理
    // ============================================================

    /// 添加黑名单关键字
    pub fn add_blacklist_keyword(&self, keyword: &str) {
        let mut filter = self.filter.write();
        filter.add_keyword(keyword);
    }

    // ============================================================
    // 查询接口
    // ============================================================

    /// 获取当前引擎状态
    pub fn get_stats(&self) -> EngineStats {
        let tl = self.timeline.read();
        let pool = self.pool.read();
        let em = self.emoji_manager.read();
        let renderer = self.renderer.read();

        EngineStats {
            engine_time_ms: tl.engine_time_ms(),
            speed: tl.speed(),
            paused: tl.is_paused(),
            alive_barrages: pool.alive_count() as u32,
            pool_capacity: pool.capacity() as u32,
            pool_free: pool.free_count() as u32,
            emoji_cache_entries: em.cache_entries() as u32,
            emoji_cache_bytes: em.cache_size(),
            last_render_count: renderer.stats.barrages_rendered,
            render_time_us: renderer.stats.render_time_us,
        }
    }
}

/// 引擎统计信息
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EngineStats {
    pub engine_time_ms: u64,
    pub speed: f32,
    pub paused: bool,
    pub alive_barrages: u32,
    pub pool_capacity: u32,
    pub pool_free: u32,
    pub emoji_cache_entries: u32,
    pub emoji_cache_bytes: u64,
    pub last_render_count: u32,
    pub render_time_us: u64,
}

impl Default for BarrageCore {
    fn default() -> Self {
        Self::with_default_config()
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_creation() {
        let core = BarrageCore::with_default_config();
        let stats = core.get_stats();
        assert!(!stats.paused);
        assert_eq!(stats.engine_time_ms, 0);
    }

    #[test]
    fn test_core_send_barrage() {
        let core = BarrageCore::with_default_config();
        core.send_barrage(
            "测试弹幕",
            TrackMode::Scroll,
            0xFFFFFFFF,
            28,
            1.0,
            5000,
            None,
        );

        // 更新一次，弹幕应被处理
        core.update(16);

        let stats = core.get_stats();
        assert_eq!(stats.alive_barrages, 1);
    }

    #[test]
    fn test_core_performance() {
        let core = BarrageCore::with_default_config();
        for i in 0..100 {
            core.send_barrage(
                &format!("弹幕{}", i),
                TrackMode::Scroll,
                0xFFFFFFFF,
                28,
                1.0,
                10000,
                None,
            );
        }

        core.update(16);
        let stats = core.get_stats();
        assert!(stats.alive_barrages > 0);
    }

    #[test]
    fn test_core_pause_resume() {
        let core = BarrageCore::with_default_config();
        assert!(!core.get_stats().paused);

        core.pause();
        assert!(core.get_stats().paused);

        core.resume();
        assert!(!core.get_stats().paused);
    }

    #[test]
    fn test_core_clear() {
        let core = BarrageCore::with_default_config();
        core.send_barrage("测试", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
        core.update(16);
        assert!(core.get_stats().alive_barrages > 0);

        core.clear();
        assert_eq!(core.get_stats().alive_barrages, 0);
    }

    #[test]
    fn test_core_set_speed() {
        let core = BarrageCore::with_default_config();
        core.set_speed(2.0);
        let stats = core.get_stats();
        assert!((stats.speed - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_core_seek() {
        let core = BarrageCore::with_default_config();
        core.seek(5000);
        let stats = core.get_stats();
        assert_eq!(stats.engine_time_ms, 5000);
    }

    #[test]
    fn test_core_emoji_rgba() {
        let core = BarrageCore::with_default_config();
        let rgba = vec![255u8; 16 * 16 * 4];
        assert!(core.register_emoji_rgba("test", &rgba, 16, 16).is_ok());

        let info = core.get_emoji_info("test");
        assert!(info.is_some());
        let (w, h, bytes) = info.unwrap();
        assert_eq!(w, 16);
        assert_eq!(h, 16);
    }

    #[test]
    fn test_render_output() {
        let core = BarrageCore::with_default_config();
        core.send_barrage("渲染测试", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
        core.update(16);

        let mut buffer = vec![0u8; 100 * 100 * 4];
        let count = core.render(&mut buffer, 100, 100);
        assert!(count >= 0);
    }
}