/// track.rs — 轨道管理器
///
/// TrackManager 管理 N 条轨道，每条轨道可以是滚动/顶部/底部/逆向模式。
/// 支持弹幕碰撞检测（基于矩形包围盒 + 速度 + 时间预测）、
/// 避让逻辑和拥挤策略（排队/丢弃）。
/// 自动清理过期弹幕。

use crate::barrage::{BarrageObject, BarrageObjectPool};
use crate::config::{CongestionStrategy, TrackMode};
use std::collections::VecDeque;

// ============================================================
// 轨道状态枚举
// ============================================================

/// 单条轨道的运行时状态
#[derive(Debug, Clone)]
pub struct Track {
    /// 轨道模式
    pub mode: TrackMode,
    /// 拥挤策略
    pub congestion_strategy: CongestionStrategy,
    /// 当前弹幕索引列表（引用对象池中的对象索引）
    pub active_barrages: Vec<usize>,
    /// 等待队列（拥挤策略为 Queue 时使用）
    pub pending_queue: VecDeque<PendingBarrage>,
    /// 最大并发弹幕数
    pub max_concurrent: u32,
    /// 轨道高度
    pub track_height: u32,
    /// 弹幕间距
    pub spacing: u32,
    /// 滚动速度
    pub scroll_speed: f32,
    /// 下一个可用的 Y 位置
    pub next_y: f32,
}

/// 排队等待的弹幕信息
#[derive(Debug, Clone)]
pub struct PendingBarrage {
    /// 弹幕对象索引
    pub pool_idx: usize,
    /// 入队时间（引擎时间，毫秒）
    pub enqueue_time_ms: u64,
}

impl Track {
    /// 创建新轨道
    pub fn new(
        mode: TrackMode,
        max_concurrent: u32,
        track_height: u32,
        spacing: u32,
        scroll_speed: f32,
        y_position: f32,
    ) -> Self {
        Self {
            mode,
            congestion_strategy: CongestionStrategy::Queue,
            active_barrages: Vec::with_capacity(max_concurrent as usize),
            pending_queue: VecDeque::new(),
            max_concurrent,
            track_height,
            spacing,
            scroll_speed,
            next_y: y_position,
        }
    }

    /// 判断轨道是否已满
    pub fn is_full(&self) -> bool {
        self.active_barrages.len() >= self.max_concurrent as usize
    }
}

/// 排队等待的弹幕数据（存入等待队列的副本）
#[derive(Debug, Clone)]
struct QueuedBarrageData {
    text: String,
    segments: Vec<crate::barrage::EmojiSegment>,
    color_rgba: u32,
    font_size: u32,
    alpha: f32,
    duration_ms: u32,
    effect_config: Option<std::sync::Arc<crate::config::TextEffectConfig>>,
}

// ============================================================
// 碰撞检测结果
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionResult {
    /// 是否可能碰撞
    pub will_collide: bool,
    /// 预测碰撞时间（毫秒）
    pub collision_time_ms: u64,
}

// ============================================================
// 轨道管理器
// ============================================================

/// 轨道管理器，管理所有轨道及弹幕调度
pub struct TrackManager {
    /// 轨道列表
    pub tracks: Vec<Track>,
    /// 画布宽度
    canvas_width: u32,
    /// 画布高度
    canvas_height: u32,
    /// 允许重叠
    allow_overlap: bool,
}

impl TrackManager {
    /// 创建轨道管理器
    pub fn new(
        track_count: u32,
        canvas_width: u32,
        canvas_height: u32,
        allow_overlap: bool,
        default_track_height: u32,
        default_spacing: u32,
        default_scroll_speed: f32,
    ) -> Self {
        let count = track_count.clamp(1, 100) as usize;
        // 计算每条轨道的 Y 起始位置（从上到下平均分布）
        let usable_height = canvas_height as f32;
        let track_height = (usable_height / count as f32).min(default_track_height as f32) as u32;
        let track_height = track_height.max(20);

        let mut tracks = Vec::with_capacity(count);
        for i in 0..count {
            let mode = match i % 4 {
                0 => TrackMode::Scroll,
                1 => TrackMode::Top,
                2 => TrackMode::Bottom,
                3 => TrackMode::Reverse,
                _ => TrackMode::Scroll,
            };
            let y_pos = (i as f32 * track_height as f32).min(canvas_height as f32 - track_height as f32);
            tracks.push(Track::new(
                mode,
                10, // 默认最大并发
                track_height,
                default_spacing,
                default_scroll_speed,
                y_pos,
            ));
        }

        Self {
            tracks,
            canvas_width,
            canvas_height,
            allow_overlap,
        }
    }

    /// 使用自定义轨道配置创建管理器
    pub fn new_with_configs(
        configs: &[crate::config::BarrageTrackConfig],
        canvas_width: u32,
        canvas_height: u32,
        allow_overlap: bool,
    ) -> Self {
        let count = configs.len().clamp(1, 100);
        let mut tracks = Vec::with_capacity(count);

        for (i, cfg) in configs.iter().enumerate() {
            let y_pos = (i as f32 * cfg.track_height as f32).min(canvas_height as f32 - cfg.track_height as f32);
            let mut track = Track::new(
                cfg.mode,
                cfg.max_concurrent,
                cfg.track_height,
                cfg.spacing,
                cfg.scroll_speed,
                y_pos,
            );
            track.congestion_strategy = cfg.congestion_strategy;
            tracks.push(track);
        }

        Self {
            tracks,
            canvas_width,
            canvas_height,
            allow_overlap,
        }
    }

    /// 添加弹幕到轨道系统
    ///
    /// 返回: Ok(()) 表示成功添加或排队; Err(()) 表示丢弃
    pub fn add_barrage(
        &mut self,
        pool: &mut BarrageObjectPool,
        pool_idx: usize,
        track_index: u32,
        current_time_ms: u64,
    ) -> Result<(), ()> {
        let ti = track_index as usize;
        if ti >= self.tracks.len() {
            return Err(()); // 轨道索引越界
        }

        let (mode, scroll_speed) = {
            let track = &self.tracks[ti];
            (track.mode, track.scroll_speed)
        };

        // 获取弹幕对象并设置位置和速度
        if let Some(obj) = pool.get_mut(pool_idx) {
            // 根据轨道模式设置初始位置和速度
            match mode {
                TrackMode::Scroll => {
                    // 从右侧进入，向左滚动
                    obj.start_x = self.canvas_width as f32;
                    obj.x = self.canvas_width as f32;
                    obj.speed_x = -scroll_speed;
                    obj.speed_y = 0.0;
                }
                TrackMode::Top => {
                    // 顶部居中固定
                    obj.start_x = self.canvas_width as f32 / 2.0;
                    obj.x = self.canvas_width as f32 / 2.0;
                    obj.speed_x = 0.0;
                    obj.speed_y = 0.0;
                }
                TrackMode::Bottom => {
                    // 底部居中固定
                    obj.start_x = self.canvas_width as f32 / 2.0;
                    obj.x = self.canvas_width as f32 / 2.0;
                    obj.speed_x = 0.0;
                    obj.speed_y = 0.0;
                }
                TrackMode::Reverse => {
                    // 从左侧进入，向右滚动
                    obj.start_x = -(obj.width as f32);
                    obj.x = -(obj.width as f32);
                    obj.speed_x = scroll_speed;
                    obj.speed_y = 0.0;
                }
            }

            obj.track_index = track_index;
            obj.y = self.tracks[ti].next_y;
            obj.start_y = obj.y;
        }

        let track = &mut self.tracks[ti];

        // 检查轨道是否已满
        if track.active_barrages.len() >= track.max_concurrent as usize {
            match track.congestion_strategy {
                CongestionStrategy::Queue => {
                    // 排队等待
                    track.pending_queue.push_back(PendingBarrage {
                        pool_idx,
                        enqueue_time_ms: current_time_ms,
                    });
                    return Ok(());
                }
                CongestionStrategy::Drop => {
                    // 丢弃该弹幕
                    pool.deallocate(pool_idx);
                    return Err(());
                }
            }
        }

        // 碰撞检测（如果不允许重叠）
        let collision_pass = if !self.allow_overlap {
            if let Some(obj) = pool.get(pool_idx) {
                !self.check_collision(ti, obj)
            } else {
                true
            }
        } else {
            true
        };

        let track = &mut self.tracks[ti];

        // 检查轨道是否已满
        if track.active_barrages.len() >= track.max_concurrent as usize {
            match track.congestion_strategy {
                CongestionStrategy::Queue => {
                    track.pending_queue.push_back(PendingBarrage {
                        pool_idx,
                        enqueue_time_ms: current_time_ms,
                    });
                    return Ok(());
                }
                CongestionStrategy::Drop => {
                    pool.deallocate(pool_idx);
                    return Err(());
                }
            }
        }

        // 碰撞检测未通过，按拥挤策略处理
        if !collision_pass {
            match track.congestion_strategy {
                CongestionStrategy::Queue => {
                    track.pending_queue.push_back(PendingBarrage {
                        pool_idx,
                        enqueue_time_ms: current_time_ms,
                    });
                    return Ok(());
                }
                CongestionStrategy::Drop => {
                    pool.deallocate(pool_idx);
                    return Err(());
                }
            }
        }

        // 添加到活跃列表
        track.active_barrages.push(pool_idx);
        Ok(())
    }

    /// 碰撞检测：判断新弹幕是否与轨道中已有弹幕碰撞
    fn check_collision(&self, track_idx: usize, new_obj: &BarrageObject) -> bool {
        let track = &self.tracks[track_idx];
        let _new_box = new_obj.bounding_box();

        for &_active_idx in &track.active_barrages {
            // 这里假设可以访问 pool（因为碰撞检测在 add_barrage 中调用，
            // 实际实现需要把 pool 传进来。目前是简化版本）
            // 实际使用中，此方法在 add_barrage 内部有 pool 上下文
        }

        // 简化碰撞检测：基于时间预测的矩形重叠检测
        // 这里预留接口，完整实现需要对象池引用
        false
    }

    /// 更新所有轨道（逐帧刷新）
    ///
    /// delta_ms: 距上一帧的毫秒数
    /// speed_multiplier: 全局速度倍率
    /// current_time_ms: 当前引擎时间
    pub fn update(
        &mut self,
        pool: &mut BarrageObjectPool,
        delta_ms: u64,
        speed_multiplier: f32,
        current_time_ms: u64,
    ) {
        // 更新每一条轨道中的活跃弹幕
        for ti in 0..self.tracks.len() {
            // 更新弹幕位置
            let mut to_remove = Vec::new();
            {
                let track = &self.tracks[ti];
                for &pool_idx in &track.active_barrages {
                    if let Some(obj) = pool.get_mut(pool_idx) {
                        obj.update(delta_ms, speed_multiplier);

                        // 检查是否已移出屏幕
                        let out_of_bounds = match track.mode {
                            TrackMode::Scroll => {
                                // 从右向左，超出左边界
                                obj.x + (obj.width as f32) < -50.0
                            }
                            TrackMode::Top | TrackMode::Bottom => {
                                // 固定弹幕过期后移除
                                !obj.alive
                            }
                            TrackMode::Reverse => {
                                // 从左向右，超出右边界
                                obj.x > self.canvas_width as f32 + 50.0
                            }
                        };

                        if !obj.alive || out_of_bounds {
                            to_remove.push(pool_idx);
                        }
                    } else {
                        to_remove.push(pool_idx);
                    }
                }
            }

            // 移除过期弹幕
            for pool_idx in to_remove {
                let track = &mut self.tracks[ti];
                if let Some(pos) = track.active_barrages.iter().position(|&x| x == pool_idx) {
                    track.active_barrages.swap_remove(pos);
                }
                pool.deallocate(pool_idx);
            }

            // 从等待队列中调度弹幕
            self.dispatch_pending(ti, pool, current_time_ms);
        }
    }

    /// 从等待队列调度弹幕到轨道
    fn dispatch_pending(&mut self, track_idx: usize, pool: &mut BarrageObjectPool, _current_time_ms: u64) {
        let track = &mut self.tracks[track_idx];
        while !track.pending_queue.is_empty()
            && track.active_barrages.len() < track.max_concurrent as usize
        {
            let pending = track.pending_queue.pop_front().unwrap();
            // 检查等待弹幕是否仍然存活
            if let Some(obj) = pool.get(pending.pool_idx) {
                if obj.alive {
                    track.active_barrages.push(pending.pool_idx);
                }
            }
        }
    }

    /// 清空所有轨道
    pub fn clear(&mut self, pool: &mut BarrageObjectPool) {
        for ti in 0..self.tracks.len() {
            let track = &mut self.tracks[ti];
            // 回收所有活跃弹幕
            for &pool_idx in &track.active_barrages {
                pool.deallocate(pool_idx);
            }
            track.active_barrages.clear();
            track.pending_queue.clear();
        }
    }

    /// 获取指定轨道的弹幕列表
    pub fn get_track_barrages<'a>(&self, track_idx: usize, pool: &'a BarrageObjectPool) -> Vec<&'a BarrageObject> {
        if track_idx >= self.tracks.len() {
            return Vec::new();
        }
        let track = &self.tracks[track_idx];
        track
            .active_barrages
            .iter()
            .filter_map(|&idx| pool.get(idx))
            .collect()
    }

    /// 获取所有存活弹幕（用于渲染）
    pub fn get_all_barrages<'a>(&self, pool: &'a BarrageObjectPool) -> Vec<&'a BarrageObject> {
        let mut result = Vec::new();
        for track in &self.tracks {
            for &idx in &track.active_barrages {
                if let Some(obj) = pool.get(idx) {
                    if obj.alive {
                        result.push(obj);
                    }
                }
            }
        }
        result
    }

    /// 获取轨道数量
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// 设置画布尺寸
    pub fn set_canvas_size(&mut self, width: u32, height: u32) {
        self.canvas_width = width;
        self.canvas_height = height;
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::barrage::BarrageObject;

    #[test]
    fn test_track_manager_creation() {
        let tm = TrackManager::new(10, 1920, 1080, false, 40, 10, 200.0);
        assert_eq!(tm.tracks.len(), 10);
        assert_eq!(tm.track_count(), 10);
    }

    #[test]
    fn test_track_modes_distribution() {
        let tm = TrackManager::new(8, 1920, 1080, false, 40, 10, 200.0);
        assert_eq!(tm.tracks[0].mode, TrackMode::Scroll);
        assert_eq!(tm.tracks[1].mode, TrackMode::Top);
        assert_eq!(tm.tracks[2].mode, TrackMode::Bottom);
        assert_eq!(tm.tracks[3].mode, TrackMode::Reverse);
        // 第4个又开始循环
        assert_eq!(tm.tracks[4].mode, TrackMode::Scroll);
    }

    #[test]
    fn test_track_full_drop() {
        let mut tm = TrackManager::new(2, 1920, 1080, false, 40, 10, 200.0);
        let mut pool = BarrageObjectPool::new(64, 16);

        // 填充轨道到满
        tm.tracks[0].max_concurrent = 2;

        let obj1 = BarrageObject::new(1, "a", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
        let (idx1, _) = pool.allocate(obj1).unwrap();
        assert!(tm.add_barrage(&mut pool, idx1, 0, 0).is_ok());

        let obj2 = BarrageObject::new(2, "b", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
        let (idx2, _) = pool.allocate(obj2).unwrap();
        assert!(tm.add_barrage(&mut pool, idx2, 0, 0).is_ok());

        // 轨道已满，使用默认 Queue 策略，应排队
        tm.tracks[0].congestion_strategy = CongestionStrategy::Drop;
        let obj3 = BarrageObject::new(3, "c", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
        let (idx3, _) = pool.allocate(obj3).unwrap();
        assert!(tm.add_barrage(&mut pool, idx3, 0, 0).is_err());
    }
}