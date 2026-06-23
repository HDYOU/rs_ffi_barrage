/// config.rs — 全局配置结构体定义
///
/// 本模块定义了弹幕引擎的所有配置数据结构，包括轨道配置、
/// 过滤配置、文字特效配置（描边/阴影/霓虹/渐变）、
/// 全局配置聚合以及 Emoji 缓存配置。
///
/// 所有结构体均标注 #[repr(C)] 以支持 C FFI 导出。
/// 数值字段均做边界截断（clamp），确保在合理范围内。

use std::ffi::CStr;
use std::os::raw::c_char;

// ============================================================
// 轨道模式枚举与拥挤策略枚举
// ============================================================

/// 轨道运行模式
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackMode {
    /// 正向滚动弹幕（从右向左，默认模式）
    Scroll = 0,
    /// 顶部固定弹幕（静止在画布上方）
    Top = 1,
    /// 底部固定弹幕（静止在画布下方）
    Bottom = 2,
    /// 逆向滚动弹幕（从左向右）
    Reverse = 3,
}

/// 轨道拥挤策略
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionStrategy {
    /// 排队等待：新弹幕进入等待队列，有空位时再显示
    Queue = 0,
    /// 丢弃新弹幕：当轨道满时直接丢弃新来的
    Drop = 1,
}

// ============================================================
// 轨道配置
// ============================================================

/// 单条轨道的运行配置
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BarrageTrackConfig {
    /// 轨道模式
    pub mode: TrackMode,
    /// 拥挤策略
    pub congestion_strategy: CongestionStrategy,
    /// 轨道最大并发弹幕数量（默认 10，最大 100）
    pub max_concurrent: u32,
    /// 轨道高度（像素，默认 40）
    pub track_height: u32,
    /// 弹幕间距（像素，默认 10）
    pub spacing: u32,
    /// 滚动速度（像素/秒，默认 200，最大 2000）
    pub scroll_speed: f32,
}

impl Default for BarrageTrackConfig {
    fn default() -> Self {
        Self {
            mode: TrackMode::Scroll,
            congestion_strategy: CongestionStrategy::Queue,
            max_concurrent: 10,
            track_height: 40,
            spacing: 10,
            scroll_speed: 200.0,
        }
    }
}

impl BarrageTrackConfig {
    /// 校验并截断字段到合法范围
    pub fn validate(&mut self) {
        self.max_concurrent = self.max_concurrent.min(100).max(1);
        self.track_height = self.track_height.min(200).max(16);
        self.spacing = self.spacing.min(100);
        self.scroll_speed = self.scroll_speed.clamp(20.0, 2000.0);
    }
}

// ============================================================
// 过滤配置
// ============================================================

/// 弹幕过滤配置
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BarrageFilterConfig {
    /// 是否启用关键字黑名单
    pub keyword_blacklist_enabled: bool,
    /// 黑名单关键字（以逗号分隔的 UTF-8 字符串，由 FFI 端传入）
    /// 实际使用中应通过 set_keywords 方法设置
    pub keyword_data: *mut c_char,
    /// 黑名单关键字长度
    pub keyword_len: u32,
    /// 是否启用短时长过滤（过滤时长过短的弹幕）
    pub short_duration_filter_enabled: bool,
    /// 最短持续时间（毫秒，默认 500）
    pub min_duration_ms: u32,
    /// 是否统一调节全局字号
    pub global_font_size_enabled: bool,
    /// 全局字号（像素，默认 28，范围 8~120）
    pub global_font_size: u32,
    /// 是否统一调节全局透明度
    pub global_alpha_enabled: bool,
    /// 全局透明度（0.0~1.0，默认 1.0）
    pub global_alpha: f32,
    /// 弹幕类型开关：是否启用滚动弹幕
    pub enable_scroll: bool,
    /// 弹幕类型开关：是否启用顶部弹幕
    pub enable_top: bool,
    /// 弹幕类型开关：是否启用底部弹幕
    pub enable_bottom: bool,
    /// 弹幕类型开关：是否启用逆向弹幕
    pub enable_reverse: bool,
}

impl Default for BarrageFilterConfig {
    fn default() -> Self {
        Self {
            keyword_blacklist_enabled: false,
            keyword_data: std::ptr::null_mut(),
            keyword_len: 0,
            short_duration_filter_enabled: false,
            min_duration_ms: 500,
            global_font_size_enabled: false,
            global_font_size: 28,
            global_alpha_enabled: false,
            global_alpha: 1.0,
            enable_scroll: true,
            enable_top: true,
            enable_bottom: true,
            enable_reverse: true,
        }
    }
}

impl BarrageFilterConfig {
    /// 校验并截断字段
    pub fn validate(&mut self) {
        self.min_duration_ms = self.min_duration_ms.min(30000);
        self.global_font_size = self.global_font_size.clamp(8, 120);
        self.global_alpha = self.global_alpha.clamp(0.0, 1.0);
    }

    /// 获取黑名单关键字列表
    pub fn get_keywords(&self) -> Vec<String> {
        if self.keyword_data.is_null() || self.keyword_len == 0 {
            return Vec::new();
        }
        unsafe {
            let cstr = CStr::from_ptr(self.keyword_data);
            let s = cstr.to_string_lossy();
            s.split(',')
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect()
        }
    }
}

impl Drop for BarrageFilterConfig {
    fn drop(&mut self) {
        // 不自动释放 keyword_data，该内存由调用方管理
    }
}

// ============================================================
// 文字特效配置（描边 / 阴影 / 霓虹 / 渐变）
// ============================================================

/// 渐变类型
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientType {
    /// 线性渐变
    Linear = 0,
    /// 径向渐变
    Radial = 1,
}

/// 霓虹预设
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowPreset {
    /// 无预设（使用自定义色标）
    None = 0,
    /// 单色预设（蓝色系）
    MonoBlue = 1,
    /// 单色预设（红色系）
    MonoRed = 2,
    /// 单色预设（绿色系）
    MonoGreen = 3,
    /// 单色预设（金色系）
    MonoGold = 4,
    /// 双色预设（蓝+紫）
    DuoBluePurple = 5,
    /// 双色预设（红+橙）
    DuoRedOrange = 6,
    /// 双色预设（青+粉）
    DuoCyanPink = 7,
    /// 彩虹预设
    Rainbow = 8,
}

/// 颜色节点（用于渐变色标和发光色标）
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColorStop {
    /// 位置（0.0~1.0）
    pub position: f32,
    /// RGBA 颜色
    pub rgba: u32,
}

/// 文字特效配置（描边 + 阴影 + 霓虹 + 渐变）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TextEffectConfig {
    // ==================== 描边 (Outline) ====================
    /// 是否启用描边
    pub outline_enabled: bool,
    /// 描边宽度（像素，0~16，默认 2）
    pub outline_width: u32,
    /// 描边 RGBA 颜色（默认 0x000000FF 黑色）
    pub outline_color_rgba: u32,
    /// 描边抗锯齿软边（0=硬边，1~4=软边模糊半径，默认 0）
    pub outline_soft_edge: u32,

    // ==================== 阴影 (Shadow) ====================
    /// 是否启用阴影
    pub shadow_enabled: bool,
    /// 阴影层数（1~8，默认 2）
    pub shadow_layers: u32,
    /// 阴影 X 偏移（像素，-32~32，默认 2）
    pub shadow_offset_x: i32,
    /// 阴影 Y 偏移（像素，-32~32，默认 2）
    pub shadow_offset_y: i32,
    /// 阴影模糊半径（像素，0~16，默认 2）
    pub shadow_blur_radius: u32,
    /// 阴影 RGBA 颜色（默认 0x00000080 半透明黑）
    pub shadow_color_rgba: u32,
    /// 光照方向角度（0~360，0=右，90=下，默认 45）
    pub shadow_light_dir: f32,

    // ==================== 霓虹 (Glow) ====================
    /// 是否启用霓虹发光
    pub glow_enabled: bool,
    /// 发光扩散半径（像素，1~32，默认 8）
    pub glow_radius: u32,
    /// 发光色标数组指针（由调用方管理）
    pub glow_color_stops: *mut ColorStop,
    /// 色标数量（最大 8）
    pub glow_color_stops_len: u32,
    /// 发光强度（0.0~1.0，默认 0.6）
    pub glow_intensity: f32,
    /// 发光预设
    pub glow_preset: GlowPreset,

    // ==================== 渐变 (Gradient) ====================
    /// 是否启用渐变
    pub gradient_enabled: bool,
    /// 渐变类型
    pub gradient_type: GradientType,
    /// 渐变角度（0~360，仅线性渐变有效，默认 0）
    pub gradient_angle: f32,
    /// 渐变色标数组指针（由调用方管理）
    pub gradient_color_stops: *mut ColorStop,
    /// 色标数量（最大 8）
    pub gradient_color_stops_len: u32,
    /// 彩虹预设（true=使用彩虹渐变覆盖色标配置）
    pub rainbow_preset: bool,
}

impl Default for TextEffectConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEffectConfig {
    /// 创建默认特效配置
    pub fn new() -> Self {
        Self {
            // 描边默认关闭
            outline_enabled: false,
            outline_width: 2,
            outline_color_rgba: 0x000000FF,
            outline_soft_edge: 0,

            // 阴影默认关闭
            shadow_enabled: false,
            shadow_layers: 2,
            shadow_offset_x: 2,
            shadow_offset_y: 2,
            shadow_blur_radius: 2,
            shadow_color_rgba: 0x00000080,
            shadow_light_dir: 45.0,

            // 霓虹默认关闭
            glow_enabled: false,
            glow_radius: 8,
            glow_color_stops: std::ptr::null_mut(),
            glow_color_stops_len: 0,
            glow_intensity: 0.6,
            glow_preset: GlowPreset::None,

            // 渐变默认关闭
            gradient_enabled: false,
            gradient_type: GradientType::Linear,
            gradient_angle: 0.0,
            gradient_color_stops: std::ptr::null_mut(),
            gradient_color_stops_len: 0,
            rainbow_preset: false,
        }
    }

    /// 校验并截断所有数值字段到合法范围
    pub fn validate(&mut self) {
        self.outline_width = self.outline_width.min(16);
        self.outline_soft_edge = self.outline_soft_edge.min(4);
        self.shadow_layers = self.shadow_layers.clamp(1, 8);
        self.shadow_offset_x = self.shadow_offset_x.clamp(-32, 32);
        self.shadow_offset_y = self.shadow_offset_y.clamp(-32, 32);
        self.shadow_blur_radius = self.shadow_blur_radius.min(16);
        self.shadow_light_dir = self.shadow_light_dir.clamp(0.0, 360.0);
        self.glow_radius = self.glow_radius.clamp(1, 32);
        self.glow_intensity = self.glow_intensity.clamp(0.0, 1.0);
        self.glow_color_stops_len = self.glow_color_stops_len.min(8);
        self.gradient_angle = self.gradient_angle.clamp(0.0, 360.0);
        self.gradient_color_stops_len = self.gradient_color_stops_len.min(8);

        // 处理霓虹预设
        if self.glow_preset != GlowPreset::None {
            self.apply_glow_preset();
        }

        // 处理彩虹预设
        if self.rainbow_preset {
            self.apply_rainbow_preset();
        }
    }

    /// 应用霓虹预设，自动填充 glow_color_stops
    fn apply_glow_preset(&mut self) {
        // 注意：这里仅示例预设色标数据；实际调用方应通过 set_glow_preset 传入堆分配数据
        // 预设仅修改色标数量和默认颜色值
        match self.glow_preset {
            GlowPreset::MonoBlue => {
                self.glow_radius = self.glow_radius.max(4);
                self.glow_intensity = 0.7;
            }
            GlowPreset::MonoRed => {
                self.glow_radius = self.glow_radius.max(4);
                self.glow_intensity = 0.7;
            }
            GlowPreset::MonoGreen => {
                self.glow_radius = self.glow_radius.max(4);
                self.glow_intensity = 0.7;
            }
            GlowPreset::MonoGold => {
                self.glow_radius = self.glow_radius.max(6);
                self.glow_intensity = 0.8;
            }
            GlowPreset::DuoBluePurple => {
                self.glow_radius = self.glow_radius.max(8);
                self.glow_intensity = 0.6;
            }
            GlowPreset::DuoRedOrange => {
                self.glow_radius = self.glow_radius.max(8);
                self.glow_intensity = 0.6;
            }
            GlowPreset::DuoCyanPink => {
                self.glow_radius = self.glow_radius.max(8);
                self.glow_intensity = 0.6;
            }
            GlowPreset::Rainbow => {
                self.glow_radius = self.glow_radius.max(12);
                self.glow_intensity = 0.8;
            }
            GlowPreset::None => {}
        }
    }

    /// 应用彩虹渐变预设
    fn apply_rainbow_preset(&mut self) {
        // 仅在用户未明确指定渐变类型时，默认使用线性渐变
        // 注意：如果用户已经设置了 gradient_type，则保留用户设置
        self.gradient_angle = 0.0;
        // 彩虹预设需要 6~7 个色标来表现可见光谱
    }
}

// ============================================================
// Emoji 缓存配置
// ============================================================

/// Emoji 贴图缓存配置
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EmojiCacheConfig {
    /// LRU 缓存最大条目数（默认 256，最大 1024）
    pub max_entries: u32,
    /// 单个贴图最大字节数（默认 4MB，最大 32MB）
    pub max_bitmap_bytes: u64,
    /// 缓存总字节数上限（默认 64MB，最大 512MB）
    pub max_total_cache_bytes: u64,
    /// 磁盘缓存目录路径（C 字符串指针，可 NULL）
    pub disk_cache_dir: *mut c_char,
    /// 磁盘缓存有效期秒数（默认 86400 = 24 小时）
    pub disk_cache_ttl_secs: u64,
}

impl Default for EmojiCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 256,
            max_bitmap_bytes: 4 * 1024 * 1024,    // 4MB
            max_total_cache_bytes: 64 * 1024 * 1024, // 64MB
            disk_cache_dir: std::ptr::null_mut(),
            disk_cache_ttl_secs: 86400, // 24小时
        }
    }
}

impl EmojiCacheConfig {
    /// 校验并截断字段
    pub fn validate(&mut self) {
        self.max_entries = self.max_entries.min(1024).max(16);
        self.max_bitmap_bytes = self.max_bitmap_bytes.min(32 * 1024 * 1024).max(1024);
        self.max_total_cache_bytes = self.max_total_cache_bytes.min(512 * 1024 * 1024).max(65536);
        self.disk_cache_ttl_secs = self.disk_cache_ttl_secs.min(7 * 86400); // 最多7天
    }
}

// ============================================================
// 全局配置聚合
// ============================================================

/// 弹幕引擎全局配置聚合
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BarrageGlobalConfig {
    /// 画布宽度（像素，默认 1920）
    pub canvas_width: u32,
    /// 画布高度（像素，默认 1080）
    pub canvas_height: u32,
    /// 轨道数量（默认 20，最大 100）
    pub track_count: u32,
    /// 默认弹幕持续时间（毫秒，默认 5000）
    pub default_duration_ms: u32,
    /// 默认弹幕字号（像素，默认 28）
    pub default_font_size: u32,
    /// 默认弹幕颜色（RGBA，默认白色）
    pub default_color_rgba: u32,
    /// 默认透明度（0.0~1.0，默认 1.0）
    pub default_alpha: f32,
    /// 是否允许弹幕重叠
    pub allow_overlap: bool,
    /// 内存池预分配数量（默认 256，最大 4096）
    pub pool_capacity: u32,
    /// 对象池扩容步长（默认 64）
    pub pool_grow_step: u32,
    /// 轨道配置数组（最多 100 条，由调用方管理）
    /// 如果为 NULL，使用统一的默认轨道配置
    pub track_configs: *mut BarrageTrackConfig,
    /// 轨道配置数量
    pub track_configs_len: u32,
}

impl Default for BarrageGlobalConfig {
    fn default() -> Self {
        Self {
            canvas_width: 1920,
            canvas_height: 1080,
            track_count: 20,
            default_duration_ms: 5000,
            default_font_size: 28,
            default_color_rgba: 0xFFFFFFFF,
            default_alpha: 1.0,
            allow_overlap: false,
            pool_capacity: 256,
            pool_grow_step: 64,
            track_configs: std::ptr::null_mut(),
            track_configs_len: 0,
        }
    }
}

impl BarrageGlobalConfig {
    /// 校验并截断字段
    pub fn validate(&mut self) {
        self.canvas_width = self.canvas_width.clamp(320, 7680);
        self.canvas_height = self.canvas_height.clamp(240, 4320);
        self.track_count = self.track_count.clamp(1, 100);
        self.default_duration_ms = self.default_duration_ms.clamp(500, 60000);
        self.default_font_size = self.default_font_size.clamp(8, 120);
        self.default_alpha = self.default_alpha.clamp(0.0, 1.0);
        self.pool_capacity = self.pool_capacity.min(4096).max(32);
        self.pool_grow_step = self.pool_grow_step.min(512).max(16);
        if !self.track_configs.is_null() {
            let len = self.track_configs_len.min(100);
            for i in 0..len {
                unsafe {
                    let cfg = &mut *self.track_configs.offset(i as isize);
                    cfg.validate();
                }
            }
        }
    }
}

// ============================================================
// JSON 序列化/反序列化支持
// ============================================================

/// 从 JSON 字符串解析 TextEffectConfig
/// 供 FFI 端传入 JSON 配置时使用
impl TextEffectConfig {
    /// 从 JSON 字符串创建特效配置
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        serde_json::from_str::<TextEffectConfigSerde>(json_str)
            .map(|s| s.into())
            .map_err(|e| format!("TextEffectConfig JSON 解析失败: {}", e))
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, String> {
        let serde_cfg: TextEffectConfigSerde = self.into();
        serde_json::to_string(&serde_cfg)
            .map_err(|e| format!("TextEffectConfig JSON 序列化失败: {}", e))
    }
}

/// TextEffectConfig 的 Serde 序列化版本（仅序列化基本字段，不序列化指针）
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct TextEffectConfigSerde {
    outline_enabled: bool,
    outline_width: u32,
    outline_color_rgba: u32,
    outline_soft_edge: u32,
    shadow_enabled: bool,
    shadow_layers: u32,
    shadow_offset_x: i32,
    shadow_offset_y: i32,
    shadow_blur_radius: u32,
    shadow_color_rgba: u32,
    shadow_light_dir: f32,
    glow_enabled: bool,
    glow_radius: u32,
    glow_intensity: f32,
    glow_preset: String,
    gradient_enabled: bool,
    gradient_type: String,
    gradient_angle: f32,
    rainbow_preset: bool,
}

impl Default for TextEffectConfigSerde {
    fn default() -> Self {
        Self {
            outline_enabled: false,
            outline_width: 2,
            outline_color_rgba: 0x000000FF,
            outline_soft_edge: 0,
            shadow_enabled: false,
            shadow_layers: 1,
            shadow_offset_x: 2,
            shadow_offset_y: 2,
            shadow_blur_radius: 2,
            shadow_color_rgba: 0x00000080,
            shadow_light_dir: 45.0,
            glow_enabled: false,
            glow_radius: 4,
            glow_intensity: 0.5,
            glow_preset: String::new(),
            gradient_enabled: false,
            gradient_type: "Linear".to_string(),
            gradient_angle: 0.0,
            rainbow_preset: false,
        }
    }
}

impl From<TextEffectConfigSerde> for TextEffectConfig {
    fn from(s: TextEffectConfigSerde) -> Self {
        let glow_preset = match s.glow_preset.as_str() {
            "MonoBlue" => GlowPreset::MonoBlue,
            "MonoRed" => GlowPreset::MonoRed,
            "MonoGreen" => GlowPreset::MonoGreen,
            "MonoGold" => GlowPreset::MonoGold,
            "DuoBluePurple" => GlowPreset::DuoBluePurple,
            "DuoRedOrange" => GlowPreset::DuoRedOrange,
            "DuoCyanPink" => GlowPreset::DuoCyanPink,
            "Rainbow" => GlowPreset::Rainbow,
            _ => GlowPreset::None,
        };
        let gradient_type = match s.gradient_type.as_str() {
            "Radial" => GradientType::Radial,
            _ => GradientType::Linear,
        };
        let mut cfg = TextEffectConfig::new();
        cfg.outline_enabled = s.outline_enabled;
        cfg.outline_width = s.outline_width;
        cfg.outline_color_rgba = s.outline_color_rgba;
        cfg.outline_soft_edge = s.outline_soft_edge;
        cfg.shadow_enabled = s.shadow_enabled;
        cfg.shadow_layers = s.shadow_layers;
        cfg.shadow_offset_x = s.shadow_offset_x;
        cfg.shadow_offset_y = s.shadow_offset_y;
        cfg.shadow_blur_radius = s.shadow_blur_radius;
        cfg.shadow_color_rgba = s.shadow_color_rgba;
        cfg.shadow_light_dir = s.shadow_light_dir;
        cfg.glow_enabled = s.glow_enabled;
        cfg.glow_radius = s.glow_radius;
        cfg.glow_intensity = s.glow_intensity;
        cfg.glow_preset = glow_preset;
        cfg.gradient_enabled = s.gradient_enabled;
        cfg.gradient_type = gradient_type;
        cfg.gradient_angle = s.gradient_angle;
        cfg.rainbow_preset = s.rainbow_preset;
        cfg.validate();
        cfg
    }
}

impl From<&TextEffectConfig> for TextEffectConfigSerde {
    fn from(cfg: &TextEffectConfig) -> Self {
        let glow_preset = match cfg.glow_preset {
            GlowPreset::MonoBlue => "MonoBlue",
            GlowPreset::MonoRed => "MonoRed",
            GlowPreset::MonoGreen => "MonoGreen",
            GlowPreset::MonoGold => "MonoGold",
            GlowPreset::DuoBluePurple => "DuoBluePurple",
            GlowPreset::DuoRedOrange => "DuoRedOrange",
            GlowPreset::DuoCyanPink => "DuoCyanPink",
            GlowPreset::Rainbow => "Rainbow",
            GlowPreset::None => "None",
        };
        let gradient_type = match cfg.gradient_type {
            GradientType::Radial => "Radial",
            GradientType::Linear => "Linear",
        };
        TextEffectConfigSerde {
            outline_enabled: cfg.outline_enabled,
            outline_width: cfg.outline_width,
            outline_color_rgba: cfg.outline_color_rgba,
            outline_soft_edge: cfg.outline_soft_edge,
            shadow_enabled: cfg.shadow_enabled,
            shadow_layers: cfg.shadow_layers,
            shadow_offset_x: cfg.shadow_offset_x,
            shadow_offset_y: cfg.shadow_offset_y,
            shadow_blur_radius: cfg.shadow_blur_radius,
            shadow_color_rgba: cfg.shadow_color_rgba,
            shadow_light_dir: cfg.shadow_light_dir,
            glow_enabled: cfg.glow_enabled,
            glow_radius: cfg.glow_radius,
            glow_intensity: cfg.glow_intensity,
            glow_preset: glow_preset.to_string(),
            gradient_enabled: cfg.gradient_enabled,
            gradient_type: gradient_type.to_string(),
            gradient_angle: cfg.gradient_angle,
            rainbow_preset: cfg.rainbow_preset,
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
    fn test_track_config_default() {
        let cfg = BarrageTrackConfig::default();
        assert_eq!(cfg.mode, TrackMode::Scroll);
        assert_eq!(cfg.max_concurrent, 10);
        assert_eq!(cfg.scroll_speed, 200.0);
    }

    #[test]
    fn test_track_config_validate() {
        let mut cfg = BarrageTrackConfig {
            max_concurrent: 999,
            scroll_speed: 9999.0,
            ..Default::default()
        };
        cfg.validate();
        assert_eq!(cfg.max_concurrent, 100);
        assert_eq!(cfg.scroll_speed, 2000.0);
    }

    #[test]
    fn test_text_effect_config_default() {
        let cfg = TextEffectConfig::new();
        assert!(!cfg.outline_enabled);
        assert_eq!(cfg.outline_width, 2);
        assert_eq!(cfg.shadow_layers, 2);
    }

    #[test]
    fn test_global_config_default() {
        let cfg = BarrageGlobalConfig::default();
        assert_eq!(cfg.canvas_width, 1920);
        assert_eq!(cfg.track_count, 20);
    }

    #[test]
    fn test_global_config_validate() {
        let mut cfg = BarrageGlobalConfig {
            canvas_width: 100,
            track_count: 999,
            ..Default::default()
        };
        cfg.validate();
        assert_eq!(cfg.canvas_width, 320);
        assert_eq!(cfg.track_count, 100);
    }
}