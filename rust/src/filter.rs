/// filter.rs — 过滤引擎
///
/// 过滤引擎负责根据配置对弹幕进行过滤处理：
/// - 关键字黑名单匹配
/// - 弹幕类型开关（开启/关闭特定轨道模式）
/// - 短时长过滤
/// - 全局字号/透明度统一调节
///
/// 过滤引擎在弹幕入轨前执行，通过过滤的弹幕才能进入轨道系统。

use crate::barrage::BarrageObject;
use crate::config::{BarrageFilterConfig, TrackMode};

/// 过滤结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// 通过过滤
    Pass,
    /// 被关键字黑名单拦截
    BlockedByKeyword,
    /// 弹幕类型被禁用
    BlockedByType,
    /// 短时长弹幕被过滤
    BlockedByDuration,
}

/// 过滤引擎
pub struct FilterEngine {
    /// 过滤配置
    config: BarrageFilterConfig,
    /// 编译后的黑名单关键字列表（小写，用于快速匹配）
    keywords_lower: Vec<String>,
}

impl FilterEngine {
    /// 创建新的过滤引擎
    pub fn new(config: BarrageFilterConfig) -> Self {
        let keywords = config.get_keywords();
        let keywords_lower = keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();
        Self {
            config,
            keywords_lower,
        }
    }

    /// 使用默认配置创建过滤引擎
    pub fn with_default_config() -> Self {
        Self::new(BarrageFilterConfig::default())
    }

    /// 更新过滤配置
    pub fn set_config(&mut self, config: BarrageFilterConfig) {
        let keywords = config.get_keywords();
        self.keywords_lower = keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();
        self.config = config;
    }

    /// 获取当前配置的引用
    pub fn config(&self) -> &BarrageFilterConfig {
        &self.config
    }

    /// 获取当前配置的可变引用
    pub fn config_mut(&mut self) -> &mut BarrageFilterConfig {
        &mut self.config
    }

    /// 对弹幕执行过滤检查
    ///
    /// 返回 FilterResult::Pass 表示通过过滤，可以进入轨道系统。
    /// 其他值表示被拦截，调用方应丢弃该弹幕。
    pub fn filter(&self, obj: &BarrageObject) -> FilterResult {
        // 1. 检查弹幕类型开关
        let type_check = self.check_type_allowed(obj.track_mode);
        if !type_check {
            return FilterResult::BlockedByType;
        }

        // 2. 检查短时长过滤
        if self.config.short_duration_filter_enabled
            && obj.duration_ms < self.config.min_duration_ms
        {
            return FilterResult::BlockedByDuration;
        }

        // 3. 检查关键字黑名单
        if self.config.keyword_blacklist_enabled && !self.keywords_lower.is_empty() {
            if self.check_keywords(&obj.text) {
                return FilterResult::BlockedByKeyword;
            }
        }

        FilterResult::Pass
    }

    /// 检查弹幕类型是否被允许
    fn check_type_allowed(&self, mode: TrackMode) -> bool {
        match mode {
            TrackMode::Scroll => self.config.enable_scroll,
            TrackMode::Top => self.config.enable_top,
            TrackMode::Bottom => self.config.enable_bottom,
            TrackMode::Reverse => self.config.enable_reverse,
        }
    }

    /// 检查文本是否包含黑名单关键字
    /// 使用简单子串匹配（不区分大小写）
    fn check_keywords(&self, text: &str) -> bool {
        if self.keywords_lower.is_empty() {
            return false;
        }
        let text_lower = text.to_lowercase();
        self.keywords_lower
            .iter()
            .any(|keyword| text_lower.contains(keyword.as_str()))
    }

    /// 对弹幕应用全局调节（字号、透明度）
    ///
    /// 注意：此方法会直接修改传入的 BarrageObject
    pub fn apply_global_adjustments(&self, obj: &mut BarrageObject) {
        // 全局字号调节
        if self.config.global_font_size_enabled {
            obj.font_size = self.config.global_font_size;
        }

        // 全局透明度调节
        if self.config.global_alpha_enabled {
            obj.alpha *= self.config.global_alpha;
            obj.alpha = obj.alpha.clamp(0.0, 1.0);
        }
    }

    /// 添加黑名单关键字
    pub fn add_keyword(&mut self, keyword: &str) {
        let kw = keyword.trim().to_lowercase();
        if !kw.is_empty() && !self.keywords_lower.contains(&kw) {
            self.keywords_lower.push(kw);
            self.config.keyword_blacklist_enabled = true;
        }
    }

    /// 清空黑名单
    pub fn clear_keywords(&mut self) {
        self.keywords_lower.clear();
    }

    /// 获取黑名单关键字列表
    pub fn keywords(&self) -> &[String] {
        &self.keywords_lower
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
    fn test_filter_keyword_block() {
        let mut config = BarrageFilterConfig::default();
        config.keyword_blacklist_enabled = true;

        // 使用内部 set_config 方法来设置关键字
        let mut engine = FilterEngine::new(config);
        engine.add_keyword("广告");

        let obj = BarrageObject::new(
            1, "这是一个广告弹幕", TrackMode::Scroll, 0,
            0xFFFFFFFF, 28, 1.0, 5000, 0,
        );

        assert_eq!(engine.filter(&obj), FilterResult::BlockedByKeyword);
    }

    #[test]
    fn test_filter_keyword_pass() {
        let mut engine = FilterEngine::with_default_config();
        engine.add_keyword("广告");

        let obj = BarrageObject::new(
            1, "正常弹幕内容", TrackMode::Scroll, 0,
            0xFFFFFFFF, 28, 1.0, 5000, 0,
        );

        assert_eq!(engine.filter(&obj), FilterResult::Pass);
    }

    #[test]
    fn test_filter_type_block() {
        let mut config = BarrageFilterConfig::default();
        config.enable_top = false; // 禁用顶部弹幕

        let engine = FilterEngine::new(config);

        let obj = BarrageObject::new(
            1, "顶部弹幕", TrackMode::Top, 0,
            0xFFFFFFFF, 28, 1.0, 5000, 0,
        );

        assert_eq!(engine.filter(&obj), FilterResult::BlockedByType);
    }

    #[test]
    fn test_filter_duration_block() {
        let mut config = BarrageFilterConfig::default();
        config.short_duration_filter_enabled = true;
        config.min_duration_ms = 1000;

        let engine = FilterEngine::new(config);

        let obj = BarrageObject::new(
            1, "短弹幕", TrackMode::Scroll, 0,
            0xFFFFFFFF, 28, 1.0, 200, 0, // 200ms < 1000ms
        );

        assert_eq!(engine.filter(&obj), FilterResult::BlockedByDuration);
    }

    #[test]
    fn test_global_adjustments() {
        let mut config = BarrageFilterConfig::default();
        config.global_font_size_enabled = true;
        config.global_font_size = 36;
        config.global_alpha_enabled = true;
        config.global_alpha = 0.5;

        let engine = FilterEngine::new(config);

        let mut obj = BarrageObject::new(
            1, "测试弹幕", TrackMode::Scroll, 0,
            0xFFFFFFFF, 28, 1.0, 5000, 0,
        );

        engine.apply_global_adjustments(&mut obj);
        assert_eq!(obj.font_size, 36);
        assert!((obj.alpha - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_add_clear_keywords() {
        let mut engine = FilterEngine::with_default_config();
        assert!(engine.keywords().is_empty());

        engine.add_keyword("关键词1");
        engine.add_keyword("关键词2");
        assert_eq!(engine.keywords().len(), 2);

        engine.clear_keywords();
        assert!(engine.keywords().is_empty());
    }

    #[test]
    fn test_filter_passes_clean_barrage() {
        let engine = FilterEngine::with_default_config();

        let obj = BarrageObject::new(
            1, "正常弹幕", TrackMode::Scroll, 0,
            0xFFFFFFFF, 28, 1.0, 5000, 0,
        );

        assert_eq!(engine.filter(&obj), FilterResult::Pass);
    }
}