/// FFI 绑定层 — Dart Hooks 版本
///
/// 使用 Dart Hooks 系统（Flutter 3.38+）自动编译和加载 Rust 原生代码。
/// 通过 @Native 注解声明 FFI 函数，构建系统自动解析符号。
///
/// 关键特性：
/// - 无需手动编写平台特定的动态库加载代码
/// - 使用 @ffi.DefaultAsset 注解指定资产位置
/// - @Native 注解自动处理符号解析
/// - 跨平台一致性：同一份代码支持 Android/iOS/macOS/Windows/Linux
///
/// 参考：
/// https://docs.flutter.dev/platform-integration/bind-native-code
/// https://api.dart.dev/dart-ffi/Native-class.html

// ── 使用 @ffi.DefaultAsset 指定原生资产位置 ──────────────────────────────────
// Dart Hooks 系统会自动将 Rust 编译的 cdylib 打包到应用中，
// 并通过此注解定位动态库。格式为 package:<包名>/<crate名>
@ffi.DefaultAsset('package:rs_ffi_barrage/rs_ffi_barrage')
library rs_ffi_barrage_bindings;

import 'dart:ffi' as ffi;
import 'package:ffi/ffi.dart';
import 'package:flutter/services.dart';

// ============================================================================
// 引擎生命周期 —— @Native 注解声明 FFI 函数
// ============================================================================

/// 创建弹幕引擎实例，返回不透明句柄
/// 参数：max_tracks (最大轨道数), pool_capacity (对象池容量)
/// 返回：引擎句柄指针
@ffi.Native<ffi.Pointer<ffi.Void> Function(ffi.Int32, ffi.Int32)>()
external ffi.Pointer<ffi.Void> rs_barrage_create(int max_tracks, int pool_capacity);

/// 销毁弹幕引擎，释放所有资源
/// 参数：engine (引擎句柄)
@ffi.Native<ffi.Void Function(ffi.Pointer<ffi.Void>)>()
external void rs_barrage_destroy(ffi.Pointer<ffi.Void> engine);

// ============================================================================
// 弹幕管理 —— @Native 注解声明 FFI 函数
// ============================================================================

/// 发送一条弹幕
/// 参数：
///   engine      - 引擎句柄
///   text        - 弹幕文本（UTF-8）
///   track_type  - 轨道类型：0=滚动, 1=顶部固定, 2=底部固定, 3=逆向滚动
///   color       - RGBA 颜色值（0xFFFFFFFF）
///   font_size   - 字号（像素）
///   alpha       - 透明度（0.0~1.0）
///   duration_ms - 显示时长（毫秒）
///   effect_json - 特效配置 JSON（可选）
/// 返回：0=成功, 负数=错误码
@ffi.Native<ffi.Int32 Function(
  ffi.Pointer<ffi.Void>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Int32,
  ffi.Uint32,
  ffi.Float,
  ffi.Float,
  ffi.Int32,
  ffi.Pointer<ffi.Uint8>,
)>()
external int rs_barrage_send(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> text,
  int track_type,
  int color,
  double font_size,
  double alpha,
  int duration_ms,
  ffi.Pointer<ffi.Uint8> effect_json,
);

/// 每帧更新引擎状态
/// 参数：engine (引擎句柄), delta_ms (时间增量毫秒)
/// 返回：当前活跃弹幕数
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Int32)>()
external int rs_barrage_update(ffi.Pointer<ffi.Void> engine, int delta_ms);

/// 渲染到 RGBA 缓冲区
/// 参数：engine, buffer (RGBA像素缓冲区), width, height
/// 返回：渲染的弹幕数
@ffi.Native<ffi.Int32 Function(
  ffi.Pointer<ffi.Void>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Int32,
  ffi.Int32,
)>()
external int rs_barrage_render(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> buffer,
  int width,
  int height,
);

/// 设置播放速度
/// 参数：engine, speed (0.25~4.0)
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Float)>()
external int rs_barrage_set_speed(ffi.Pointer<ffi.Void> engine, double speed);

/// 暂停弹幕播放
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>)>()
external int rs_barrage_pause(ffi.Pointer<ffi.Void> engine);

/// 恢复弹幕播放
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>)>()
external int rs_barrage_resume(ffi.Pointer<ffi.Void> engine);

/// 跳转到指定时间
/// 参数：engine, time_ms (目标时间毫秒)
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Int32)>()
external int rs_barrage_seek(ffi.Pointer<ffi.Void> engine, int time_ms);

/// 清空所有弹幕
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>)>()
external int rs_barrage_clear(ffi.Pointer<ffi.Void> engine);

/// 设置全局默认特效
/// 参数：engine, effect_json (特效配置 JSON)
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Pointer<ffi.Uint8>)>()
external int rs_barrage_set_global_effect(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> effect_json,
);

/// 添加黑名单关键词
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Pointer<ffi.Uint8>)>()
external int rs_barrage_add_blacklist(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> keyword,
);

/// 设置过滤器
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Pointer<ffi.Uint8>)>()
external int rs_barrage_set_filter(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> filter_json,
);

// ============================================================================
// Emoji/贴图管理 —— @Native 注解声明 FFI 函数
// ============================================================================

/// 从 Flutter 内存 RGBA 数据注册贴图
/// 参数：engine, emoji_id, rgba_data, width, height
/// 返回：0=成功, 负数=错误码
@ffi.Native<ffi.Int32 Function(
  ffi.Pointer<ffi.Void>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Uint32,
  ffi.Uint32,
)>()
external int rs_barrage_register_emoji_bitmap(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> emoji_id,
  ffi.Pointer<ffi.Uint8> rgba_data,
  int width,
  int height,
);

/// 从本地文件路径注册贴图
/// 参数：engine, emoji_id, file_path
@ffi.Native<ffi.Int32 Function(
  ffi.Pointer<ffi.Void>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Pointer<ffi.Uint8>,
)>()
external int rs_barrage_register_emoji_file(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> emoji_id,
  ffi.Pointer<ffi.Uint8> file_path,
);

/// 从网络 URL 注册贴图（引擎内部下载）
/// 参数：engine, emoji_id, url
@ffi.Native<ffi.Int32 Function(
  ffi.Pointer<ffi.Void>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Pointer<ffi.Uint8>,
)>()
external int rs_barrage_register_emoji_url(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> emoji_id,
  ffi.Pointer<ffi.Uint8> url,
);

/// 查询贴图信息：宽、高、像素字节数
/// 参数：engine, emoji_text, out_width, out_height, out_data_size
/// 返回：0=找到, -1=未找到
@ffi.Native<ffi.Int32 Function(
  ffi.Pointer<ffi.Void>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Pointer<ffi.Uint32>,
  ffi.Pointer<ffi.Uint32>,
  ffi.Pointer<ffi.Uint64>,
)>()
external int rs_barrage_get_emoji_info(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> emoji_text,
  ffi.Pointer<ffi.Uint32> out_width,
  ffi.Pointer<ffi.Uint32> out_height,
  ffi.Pointer<ffi.Uint64> out_data_size,
);

/// 拷贝贴图像素数据到缓冲区
/// 参数：engine, emoji_text, out_buffer, buffer_size
/// 返回：0=成功, -1=未找到, -2=缓冲区不足
@ffi.Native<ffi.Int32 Function(
  ffi.Pointer<ffi.Void>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Pointer<ffi.Uint8>,
  ffi.Uint64,
)>()
external int rs_barrage_copy_emoji_bitmap(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> emoji_text,
  ffi.Pointer<ffi.Uint8> out_buffer,
  int buffer_size,
);

/// 移除单个贴图
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Pointer<ffi.Uint8>)>()
external int rs_barrage_remove_emoji(
  ffi.Pointer<ffi.Void> engine,
  ffi.Pointer<ffi.Uint8> emoji_id,
);

/// 清空所有贴图缓存
@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>)>()
external int rs_barrage_clear_emoji_cache(ffi.Pointer<ffi.Void> engine);

/// 获取贴图缓存总字节数
@ffi.Native<ffi.Int64 Function(ffi.Pointer<ffi.Void>)>()
external int rs_barrage_get_emoji_cache_size(ffi.Pointer<ffi.Void> engine);

/// 获取版本号
@ffi.Native<ffi.Pointer<ffi.Uint8> Function()>()
external ffi.Pointer<ffi.Uint8> rs_barrage_version();

// ============================================================================
// 辅助工具函数 —— 简化 FFI 调用
// ============================================================================

/// 将 Dart String 转为 Native UTF-8 字符串指针
/// 使用 Arena 自动管理内存生命周期
extension StringToNativeUtf8 on String {
  ffi.Pointer<ffi.Uint8> toNativeUtf8(Arena arena) {
    final units = utf8.encode(this);
    final result = arena<ffi.Uint8>(units.length + 1);
    final nativeArray = result.asTypedList(units.length + 1);
    nativeArray.setAll(0, units);
    nativeArray[units.length] = 0; // NULL 终止符
    return result;
  }
}

/// 从 Native UTF-8 指针读取 Dart String
ffi.Pointer<ffi.Uint8> toNativeUtf8String(String str, Arena arena) {
  return str.toNativeUtf8(arena);
}

// ============================================================================
// MethodChannel 简易兼容封装（备选方案）
// ============================================================================

/// BarrageMethodChannel —— 通过 MethodChannel 调用 Rust 功能的兼容封装
///
/// 当 FFI 直调不可用时，可使用此备选方案。
/// 注意：MethodChannel 性能低于 FFI 直调，仅作为兼容方案。
class BarrageMethodChannel {
  static const MethodChannel _channel = MethodChannel('rs_ffi_barrage');

  /// 发送弹幕（通过 MethodChannel）
  Future<int> sendBarrage({
    required String text,
    int trackType = 0,
    int color = 0xFFFFFFFF,
    double fontSize = 24.0,
    double alpha = 1.0,
    int durationMs = 5000,
    String? effectJson,
  }) async {
    try {
      final result = await _channel.invokeMethod<int>('sendBarrage', {
        'text': text,
        'trackType': trackType,
        'color': color,
        'fontSize': fontSize,
        'alpha': alpha,
        'durationMs': durationMs,
        'effectJson': effectJson ?? '',
      });
      return result ?? -1;
    } catch (e) {
      print('MethodChannel sendBarrage 调用失败: $e');
      return -1;
    }
  }

  /// 更新引擎状态
  Future<int> update(int deltaMs) async {
    try {
      final result = await _channel.invokeMethod<int>('update', {'deltaMs': deltaMs});
      return result ?? -1;
    } catch (e) {
      print('MethodChannel update 调用失败: $e');
      return -1;
    }
  }

  /// 设置播放速度
  Future<int> setSpeed(double speed) async {
    try {
      final result = await _channel.invokeMethod<int>('setSpeed', {'speed': speed});
      return result ?? -1;
    } catch (e) {
      print('MethodChannel setSpeed 调用失败: $e');
      return -1;
    }
  }

  /// 暂停
  Future<int> pause() async {
    try {
      final result = await _channel.invokeMethod<int>('pause');
      return result ?? -1;
    } catch (e) {
      print('MethodChannel pause 调用失败: $e');
      return -1;
    }
  }

  /// 恢复
  Future<int> resume() async {
    try {
      final result = await _channel.invokeMethod<int>('resume');
      return result ?? -1;
    } catch (e) {
      print('MethodChannel resume 调用失败: $e');
      return -1;
    }
  }

  /// 清空
  Future<int> clear() async {
    try {
      final result = await _channel.invokeMethod<int>('clear');
      return result ?? -1;
    } catch (e) {
      print('MethodChannel clear 调用失败: $e');
      return -1;
    }
  }

  /// 注册 Emoji（从本地路径）
  Future<int> registerEmojiFile(String emojiId, String filePath) async {
    try {
      final result = await _channel.invokeMethod<int>('registerEmojiFile', {
        'emojiId': emojiId,
        'filePath': filePath,
      });
      return result ?? -1;
    } catch (e) {
      print('MethodChannel registerEmojiFile 调用失败: $e');
      return -1;
    }
  }

  /// 注册 Emoji（从网络 URL）
  Future<int> registerEmojiUrl(String emojiId, String url) async {
    try {
      final result = await _channel.invokeMethod<int>('registerEmojiUrl', {
        'emojiId': emojiId,
        'url': url,
      });
      return result ?? -1;
    } catch (e) {
      print('MethodChannel registerEmojiUrl 调用失败: $e');
      return -1;
    }
  }
}