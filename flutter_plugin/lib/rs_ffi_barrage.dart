/// rs_ffi_barrage —— 高性能Flutter弹幕插件
///
/// 底层基于 Rust FFI 引擎，支持：
/// - Emoji贴图三种注册（Flutter位图/本地文件/网络URL）
/// - 文字特效（描边/阴影/霓虹发光/七彩渐变）
/// - 多轨道调度、过滤器、黑名单
library rs_ffi_barrage;

export 'src/text_effect_config.dart';
export 'src/emoji_manager.dart';
export 'src/barrage_engine.dart';
export 'src/barrage_view.dart';
export 'src/ffi_bindings.dart';
export 'src/method_channel_bridge.dart';