/// 文字特效配置实体类
///
/// 包含描边、阴影、霓虹发光、七彩渐变四大类特效参数。
/// 提供多种预设静态工厂方法及 JSON 序列化能力。

import 'dart:convert';

/// 文字特效配置
///
/// 用于控制弹幕文字的视觉特效叠加。所有特效可同时启用，
/// 最终效果由 Rust 引擎按标准顺序混合。
class TextEffectConfig {
  // ========================================================================
  // 描边参数（Outline）
  // ========================================================================
  /// 是否启用描边效果
  bool outlineEnabled;
  /// 描边宽度，范围 0.0 ~ 10.0，默认 2.0
  double outlineWidth;
  /// 描边颜色 RGBA 格式（如 0xFFFFFFFF 白色），默认白色
  int outlineColorRgba;
  /// 是否启用描边柔边，默认 true
  bool outlineSoftEdge;

  // ========================================================================
  // 阴影参数（Shadow）
  // ========================================================================
  /// 是否启用阴影效果
  bool shadowEnabled;
  /// 阴影层数，范围 0 ~ 8，默认 3
  int shadowLayers;
  /// 阴影水平偏移，默认 2.0
  double shadowOffsetX;
  /// 阴影垂直偏移，默认 2.0
  double shadowOffsetY;
  /// 阴影模糊半径，范围 0.0 ~ 20.0，默认 4.0
  double shadowBlurRadius;
  /// 阴影颜色 RGBA 格式（如 0x80000000 半透明黑），默认半透明黑
  int shadowColorRgba;
  /// 阴影光照方向角度，范围 0 ~ 360，默认 135.0（右下方向）
  double shadowLightDir;

  // ========================================================================
  // 霓虹发光参数（Glow）
  // ========================================================================
  /// 是否启用霓虹发光效果
  bool glowEnabled;
  /// 发光半径，范围 0.0 ~ 30.0，默认 8.0
  double glowRadius;
  /// 发光颜色阶梯列表（RGBA 格式整数列表）
  List<int> glowColorStops;
  /// 发光强度，范围 0.0 ~ 1.0，默认 0.7
  double glowIntensity;
  /// 发光预设：0=自定义, 1=单色蓝, 2=双色紫粉
  int glowPreset;

  // ========================================================================
  // 七彩渐变参数（Gradient）
  // ========================================================================
  /// 是否启用七彩渐变效果
  bool gradientEnabled;
  /// 渐变类型：0=线性渐变, 1=径向渐变
  int gradientType;
  /// 线性渐变角度，范围 0 ~ 360，默认 0（水平方向）
  double gradientAngle;
  /// 渐变颜色列表（RGBA 格式整数列表）
  List<int> gradientColorStops;
  /// 是否使用彩虹预设，默认 false
  bool rainbowPreset;

  // ========================================================================
  // 构造函数 —— 含默认值
  // ========================================================================
  TextEffectConfig({
    // 描边
    this.outlineEnabled = false,
    this.outlineWidth = 2.0,
    this.outlineColorRgba = 0xFFFFFFFF,
    this.outlineSoftEdge = true,
    // 阴影
    this.shadowEnabled = false,
    this.shadowLayers = 3,
    this.shadowOffsetX = 2.0,
    this.shadowOffsetY = 2.0,
    this.shadowBlurRadius = 4.0,
    this.shadowColorRgba = 0x80000000,
    this.shadowLightDir = 135.0,
    // 霓虹发光
    this.glowEnabled = false,
    this.glowRadius = 8.0,
    this.glowColorStops = const [],
    this.glowIntensity = 0.7,
    this.glowPreset = 0,
    // 七彩渐变
    this.gradientEnabled = false,
    this.gradientType = 0,
    this.gradientAngle = 0.0,
    this.gradientColorStops = const [],
    this.rainbowPreset = false,
  });

  // ========================================================================
  // 预设工厂方法
  // ========================================================================

  /// 彩虹渐变预设
  ///
  /// 红橙黄绿青蓝紫 7 色，90 度线性渐变。
  static TextEffectConfig rainbowGradient() {
    return TextEffectConfig(
      gradientEnabled: true,
      gradientType: 0, // 线性渐变
      gradientAngle: 90.0,
      gradientColorStops: [
        // 红、橙、黄、绿、青、蓝、紫 —— 各色完全不透明
        0xFFFF0000, // 红
        0xFFFF8C00, // 橙
        0xFFFFFF00, // 黄
        0xFF00FF00, // 绿
        0xFF00FFFF, // 青
        0xFF0000FF, // 蓝
        0xFF8B00FF, // 紫
      ],
      rainbowPreset: true,
    );
  }

  /// 霓虹发光预设
  ///
  /// 单色蓝辉光效果。
  static TextEffectConfig neonGlow() {
    return TextEffectConfig(
      glowEnabled: true,
      glowRadius: 8.0,
      glowColorStops: [
        0xFF0044FF, // 蓝色主色调
        0xFF0088FF, // 亮蓝过渡
        0xFF00CCFF, // 浅蓝边缘
      ],
      glowIntensity: 0.7,
      glowPreset: 1, // 单色蓝预设
    );
  }

  /// 浮雕阴影预设
  ///
  /// 多层阴影叠加，产生立体浮雕效果。
  static TextEffectConfig embossShadow() {
    return TextEffectConfig(
      shadowEnabled: true,
      shadowLayers: 6,
      shadowOffsetX: 1.5,
      shadowOffsetY: 1.5,
      shadowBlurRadius: 2.0,
      shadowColorRgba: 0x66000000,
      shadowLightDir: 135.0,
    );
  }

  /// 仅描边预设
  ///
  /// 简单的白色描边，常用于提高文字可读性。
  static TextEffectConfig outlineOnly() {
    return TextEffectConfig(
      outlineEnabled: true,
      outlineWidth: 3.0,
      outlineColorRgba: 0xFFFFFFFF,
      outlineSoftEdge: true,
    );
  }

  /// 全特效叠加预设
  ///
  /// 同时启用：彩虹渐变 + 描边 + 浮雕阴影 + 霓虹发光。
  static TextEffectConfig allEffects() {
    return TextEffectConfig(
      // 描边
      outlineEnabled: true,
      outlineWidth: 2.0,
      outlineColorRgba: 0xFFCCCCCC,
      outlineSoftEdge: true,
      // 阴影
      shadowEnabled: true,
      shadowLayers: 4,
      shadowOffsetX: 2.0,
      shadowOffsetY: 2.0,
      shadowBlurRadius: 4.0,
      shadowColorRgba: 0x99000000,
      shadowLightDir: 135.0,
      // 霓虹发光
      glowEnabled: true,
      glowRadius: 10.0,
      glowColorStops: [
        0xFF0044FF,
        0xFF0088FF,
        0xFF00CCFF,
      ],
      glowIntensity: 0.6,
      glowPreset: 1,
      // 七彩渐变
      gradientEnabled: true,
      gradientType: 0,
      gradientAngle: 90.0,
      gradientColorStops: [
        0xFFFF0000,
        0xFFFF8C00,
        0xFFFFFF00,
        0xFF00FF00,
        0xFF00FFFF,
        0xFF0000FF,
        0xFF8B00FF,
      ],
      rainbowPreset: true,
    );
  }

  // ========================================================================
  // 序列化方法 —— 用于传递到 Rust 层
  // ========================================================================

  /// 将当前配置转换为 Map（用于 JSON 序列化）
  Map<String, dynamic> toMap() {
    return {
      'outlineEnabled': outlineEnabled,
      'outlineWidth': outlineWidth,
      'outlineColorRgba': outlineColorRgba,
      'outlineSoftEdge': outlineSoftEdge,
      'shadowEnabled': shadowEnabled,
      'shadowLayers': shadowLayers,
      'shadowOffsetX': shadowOffsetX,
      'shadowOffsetY': shadowOffsetY,
      'shadowBlurRadius': shadowBlurRadius,
      'shadowColorRgba': shadowColorRgba,
      'shadowLightDir': shadowLightDir,
      'glowEnabled': glowEnabled,
      'glowRadius': glowRadius,
      'glowColorStops': glowColorStops,
      'glowIntensity': glowIntensity,
      'glowPreset': glowPreset,
      'gradientEnabled': gradientEnabled,
      'gradientType': gradientType,
      'gradientAngle': gradientAngle,
      'gradientColorStops': gradientColorStops,
      'rainbowPreset': rainbowPreset,
    };
  }

  /// 从 Map 构造配置
  factory TextEffectConfig.fromMap(Map<String, dynamic> map) {
    return TextEffectConfig(
      outlineEnabled: map['outlineEnabled'] as bool? ?? false,
      outlineWidth: (map['outlineWidth'] as num?)?.toDouble() ?? 2.0,
      outlineColorRgba: map['outlineColorRgba'] as int? ?? 0xFFFFFFFF,
      outlineSoftEdge: map['outlineSoftEdge'] as bool? ?? true,
      shadowEnabled: map['shadowEnabled'] as bool? ?? false,
      shadowLayers: map['shadowLayers'] as int? ?? 3,
      shadowOffsetX: (map['shadowOffsetX'] as num?)?.toDouble() ?? 2.0,
      shadowOffsetY: (map['shadowOffsetY'] as num?)?.toDouble() ?? 2.0,
      shadowBlurRadius: (map['shadowBlurRadius'] as num?)?.toDouble() ?? 4.0,
      shadowColorRgba: map['shadowColorRgba'] as int? ?? 0x80000000,
      shadowLightDir: (map['shadowLightDir'] as num?)?.toDouble() ?? 135.0,
      glowEnabled: map['glowEnabled'] as bool? ?? false,
      glowRadius: (map['glowRadius'] as num?)?.toDouble() ?? 8.0,
      glowColorStops: (map['glowColorStops'] as List<dynamic>?)
              ?.map((e) => e as int)
              .toList() ??
          [],
      glowIntensity: (map['glowIntensity'] as num?)?.toDouble() ?? 0.7,
      glowPreset: map['glowPreset'] as int? ?? 0,
      gradientEnabled: map['gradientEnabled'] as bool? ?? false,
      gradientType: map['gradientType'] as int? ?? 0,
      gradientAngle: (map['gradientAngle'] as num?)?.toDouble() ?? 0.0,
      gradientColorStops: (map['gradientColorStops'] as List<dynamic>?)
              ?.map((e) => e as int)
              .toList() ??
          [],
      rainbowPreset: map['rainbowPreset'] as bool? ?? false,
    );
  }

  /// 序列化为 JSON 字符串
  String toJson() => jsonEncode(toMap());

  /// 从 JSON 字符串反序列化
  factory TextEffectConfig.fromJson(String json) {
    try {
      final map = jsonDecode(json) as Map<String, dynamic>;
      return TextEffectConfig.fromMap(map);
    } catch (e) {
      print('TextEffectConfig.fromJson 解析失败: $e，返回默认配置');
      return TextEffectConfig();
    }
  }

  @override
  String toString() {
    return 'TextEffectConfig('
        'outline: $outlineEnabled/$outlineWidth, '
        'shadow: $shadowEnabled/$shadowLayers, '
        'glow: $glowEnabled/${glowPreset == 0 ? "custom" : glowPreset}, '
        'gradient: $gradientEnabled/${rainbowPreset ? "rainbow" : gradientType}'
        ')';
  }
}