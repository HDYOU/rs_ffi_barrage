/// timeline.rs — 时间轴控制器
///
/// 管理弹幕引擎的播放时间线，支持：
/// - 播放速度控制（0.25x ~ 4.0x）
/// - 暂停/恢复
/// - Seek 跳转到指定时间点
/// - 清空
/// - 根据时间和速度计算弹幕的当前显示状态
///
/// 时间单位：毫秒 (u64)

use std::time::Instant;

/// 时间轴控制器
///
/// 内部使用单调时钟（Instant）测量实际流逝时间，
/// 再乘以速度倍率得到引擎时间。
pub struct TimelineController {
    /// 播放速度倍率（0.25~4.0）
    speed: f32,
    /// 是否暂停
    paused: bool,
    /// 引擎累计时间（毫秒）
    engine_time_ms: u64,
    /// 上一次 update 时的实际时间
    last_update: Instant,
    /// 暂停时的引擎时间快照
    pause_engine_time_ms: u64,
    /// 基准时间（Seek 时重置）
    base_time_ms: u64,

    // ===== 统计信息 =====
    /// 总播放时间（实际时间，不含暂停）
    total_wall_time_ms: u64,
    /// 总引擎时间
    total_engine_time_ms: u64,
}

impl TimelineController {
    /// 创建新的时间轴控制器
    pub fn new() -> Self {
        Self {
            speed: 1.0,
            paused: false,
            engine_time_ms: 0,
            last_update: Instant::now(),
            pause_engine_time_ms: 0,
            base_time_ms: 0,
            total_wall_time_ms: 0,
            total_engine_time_ms: 0,
        }
    }

    /// 创建带初始速度的时间轴控制器
    pub fn with_speed(speed: f32) -> Self {
        let mut ctrl = Self::new();
        ctrl.set_speed(speed);
        ctrl
    }

    /// 更新引擎时间（每帧调用）
    ///
    /// 返回本次更新的引擎时间增量（毫秒）
    pub fn update(&mut self) -> u64 {
        if self.paused {
            return 0; // 暂停状态不更新时间
        }

        let now = Instant::now();
        let wall_delta = now.duration_since(self.last_update);
        self.last_update = now;

        // 实际流逝的毫秒数
        let wall_ms = wall_delta.as_millis() as u64;
        // 根据速度倍率计算引擎时间增量
        let engine_delta = ((wall_ms as f32) * self.speed) as u64;

        self.engine_time_ms += engine_delta;
        self.total_wall_time_ms += wall_ms;
        self.total_engine_time_ms += engine_delta;

        engine_delta
    }

    /// 按指定增量推进引擎时间（用于测试或确定性子弹更新）
    ///
    /// wall_delta_ms: 实际时间增量（毫秒）
    /// 返回引擎时间增量（含速度倍率）
    pub fn advance_by(&mut self, wall_delta_ms: u64) -> u64 {
        if self.paused {
            return 0;
        }

        // 根据速度倍率计算引擎时间增量
        let engine_delta = ((wall_delta_ms as f32) * self.speed) as u64;

        self.engine_time_ms += engine_delta;
        self.total_wall_time_ms += wall_delta_ms;
        self.total_engine_time_ms += engine_delta;

        engine_delta
    }

    // ===== 控制方法 =====

    /// 暂停播放
    pub fn pause(&mut self) {
        if !self.paused {
            self.paused = true;
            self.pause_engine_time_ms = self.engine_time_ms;
        }
    }

    /// 恢复播放
    pub fn resume(&mut self) {
        if self.paused {
            self.paused = false;
            self.last_update = Instant::now();
        }
    }

    /// 切换暂停状态
    pub fn toggle_pause(&mut self) {
        if self.paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    /// Seek 跳转到指定时间点（毫秒）
    pub fn seek(&mut self, time_ms: u64) {
        self.engine_time_ms = time_ms;
        self.base_time_ms = time_ms;
        self.last_update = Instant::now();
        // 如果暂停，同时更新暂停快照
        if self.paused {
            self.pause_engine_time_ms = time_ms;
        }
    }

    /// 清空时间轴（重置所有状态）
    pub fn clear(&mut self) {
        self.engine_time_ms = 0;
        self.base_time_ms = 0;
        self.pause_engine_time_ms = 0;
        self.total_wall_time_ms = 0;
        self.total_engine_time_ms = 0;
        self.paused = false;
        self.speed = 1.0;
        self.last_update = Instant::now();
    }

    // ===== 速度控制 =====

    /// 设置播放速度（自动 clamp 到 0.25~4.0）
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.25, 4.0);
    }

    /// 获取当前播放速度
    pub fn speed(&self) -> f32 {
        self.speed
    }

    // ===== 查询方法 =====

    /// 获取当前引擎时间（毫秒）
    pub fn engine_time_ms(&self) -> u64 {
        self.engine_time_ms
    }

    /// 是否暂停
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// 根据引擎时间计算弹幕的当前显示状态
    ///
    /// 返回 (引擎时间, 速度倍率, 是否活跃)
    pub fn get_state(&self) -> (u64, f32, bool) {
        (self.engine_time_ms, self.speed, !self.paused)
    }

    /// 判断弹幕在当前时间是否应显示
    ///
    /// created_ms: 弹幕创建时的引擎时间
    /// duration_ms: 弹幕持续时间
    pub fn is_barrage_visible(&self, created_ms: u64, duration_ms: u32) -> bool {
        if self.paused {
            return true; // 暂停时保持当前显示状态
        }
        let elapsed = self.engine_time_ms.saturating_sub(created_ms);
        elapsed < duration_ms as u64
    }

    /// 计算弹幕在当前时间的显示进度（0.0~1.0）
    ///
    /// 用于计算淡入淡出等效果
    pub fn get_barrage_progress(&self, created_ms: u64, duration_ms: u32) -> f32 {
        if duration_ms == 0 {
            return 1.0;
        }
        let elapsed = self.engine_time_ms.saturating_sub(created_ms);
        (elapsed as f32 / duration_ms as f32).clamp(0.0, 1.0)
    }

    /// 获取运行统计信息
    pub fn get_stats(&self) -> TimelineStats {
        TimelineStats {
            engine_time_ms: self.engine_time_ms,
            speed: self.speed,
            paused: self.paused,
            total_wall_time_ms: self.total_wall_time_ms,
            total_engine_time_ms: self.total_engine_time_ms,
        }
    }
}

impl Default for TimelineController {
    fn default() -> Self {
        Self::new()
    }
}

/// 时间轴统计信息
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TimelineStats {
    /// 当前引擎时间（毫秒）
    pub engine_time_ms: u64,
    /// 播放速度
    pub speed: f32,
    /// 是否暂停
    pub paused: bool,
    /// 总实际播放时间（毫秒，不含暂停）
    pub total_wall_time_ms: u64,
    /// 总引擎时间（毫秒）
    pub total_engine_time_ms: u64,
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_creation() {
        let tl = TimelineController::new();
        assert_eq!(tl.speed(), 1.0);
        assert!(!tl.is_paused());
        assert_eq!(tl.engine_time_ms(), 0);
    }

    #[test]
    fn test_timeline_speed_clamp() {
        let mut tl = TimelineController::new();
        tl.set_speed(10.0);
        assert!((tl.speed() - 4.0).abs() < 0.001);

        tl.set_speed(-1.0);
        assert!((tl.speed() - 0.25).abs() < 0.001);

        tl.set_speed(2.0);
        assert!((tl.speed() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_timeline_pause_resume() {
        let mut tl = TimelineController::new();
        assert!(!tl.is_paused());

        tl.pause();
        assert!(tl.is_paused());

        // 暂停时更新时间应返回 0
        let delta = tl.update();
        assert_eq!(delta, 0);

        tl.resume();
        assert!(!tl.is_paused());
    }

    #[test]
    fn test_timeline_seek() {
        let mut tl = TimelineController::new();
        tl.seek(5000);
        assert_eq!(tl.engine_time_ms(), 5000);

        tl.seek(0);
        assert_eq!(tl.engine_time_ms(), 0);
    }

    #[test]
    fn test_timeline_clear() {
        let mut tl = TimelineController::new();
        tl.seek(10000);
        tl.set_speed(2.0);
        tl.pause();

        tl.clear();
        assert_eq!(tl.engine_time_ms(), 0);
        assert!((tl.speed() - 1.0).abs() < 0.001);
        assert!(!tl.is_paused());
    }

    #[test]
    fn test_barrage_visibility() {
        let mut tl = TimelineController::new();
        tl.seek(1000);

        // 弹幕创建于 500ms，持续 2000ms
        assert!(tl.is_barrage_visible(500, 2000));
        // 弹幕创建于 0ms，持续 500ms
        assert!(!tl.is_barrage_visible(0, 500));
    }

    #[test]
    fn test_barrage_progress() {
        let mut tl = TimelineController::new();
        tl.seek(500);

        let progress = tl.get_barrage_progress(0, 1000);
        assert!((progress - 0.5).abs() < 0.01);
    }
}