/// 弹幕引擎管理类
///
/// BarrageEngine 负责管理 Rust 引擎句柄的生命周期，提供弹幕发送、
/// 每帧更新、渲染、速度控制、暂停恢复、特效配置等全功能接口。
///
/// 构造方法调用 rs_barrage_create() 获取引擎句柄，
/// dispose() 中调用 rs_barrage_destroy() 释放所有资源。

import 'dart:ffi';
import 'dart:typed_data';
import 'package:ffi/ffi.dart';
import 'ffi_bindings.dart' as ffi;
import 'emoji_manager.dart';
import 'text_effect_config.dart';

/// 弹幕引擎
///
/// 使用示例：
/// ```dart
/// final engine = BarrageEngine();
/// engine.sendBarrage('Hello 弹幕!', effectConfig: TextEffectConfig.neonGlow());
/// engine.update(16); // ~60fps 更新
/// engine.render(buffer, 1920, 1080);
/// engine.dispose(); // 使用完毕释放资源
/// ```
class BarrageEngine {
  /// Rust 引擎不透明句柄
  late final ffi.RsHandle _handle;

  /// 是否已初始化
  bool _initialized = false;

  /// 是否已释放
  bool _disposed = false;

  /// Emoji 管理器子成员
  late final EmojiManager emojiManager;

  /// 最大轨道数，默认 10
  final int maxTracks;

  /// 弹幕池容量，默认 500
  final int poolCapacity;

  // ========================================================================
  // 构造与析构
  // ========================================================================

  /// 创建弹幕引擎实例
  ///
  /// [maxTracks] 最大并行轨道数，默认 10
  /// [poolCapacity] 弹幕对象池容量，默认 500
  BarrageEngine({
    this.maxTracks = 10,
    this.poolCapacity = 500,
  }) {
    try {
      _handle = ffi.rs_barrage_create(maxTracks, poolCapacity);
      _initialized = true;
      // 初始化 Emoji 管理器
      emojiManager = EmojiManager(_handle);
    } catch (e) {
      print('BarrageEngine 构造失败: $e');
      rethrow;
    }
  }

  /// 释放引擎资源
  ///
  /// 调用 Rust 侧的 rs_barrage_destroy() 释放句柄关联的所有内存。
  /// 调用后引擎不再可用。
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _initialized = false;
    try {
      ffi.rs_barrage_destroy(_handle);
    } catch (e) {
      print('BarrageEngine dispose 失败: $e');
    }
  }

  /// 检查引擎是否可用
  bool get isActive => _initialized && !_disposed;

  // ========================================================================
  // 弹幕发送
  // ========================================================================

  /// 发送一条弹幕
  ///
  /// [text] 弹幕文本内容，支持贴图引用如 "[666]" 嵌入贴图
  /// [trackType] 轨道类型：0=自动, 1=顶部, 2=底部, 3=滚动
  /// [color] 文字颜色 RGBA 格式，默认 0xFFFFFFFF（白色）
  /// [fontSize] 字号，默认 24.0
  /// [alpha] 透明度 0.0~1.0，默认 1.0
  /// [durationMs] 弹幕持续时间（毫秒），默认 5000
  /// [effectConfig] 文字特效配置，不传则使用全局默认
  /// 返回 0 表示成功，负数表示失败
  int sendBarrage(
    String text, {
    int trackType = 0,
    int color = 0xFFFFFFFF,
    double fontSize = 24.0,
    double alpha = 1.0,
    int durationMs = 5000,
    TextEffectConfig? effectConfig,
  }) {
    if (!isActive) {
      print('sendBarrage 失败：引擎未初始化或已释放');
      return -1;
    }

    // 将文本转为 C 字符串
    final textPtr = text.toNativeUtf8();
    // 将特效配置转为 JSON C 字符串
    final effectJson = effectConfig?.toJson() ?? '';
    final effectPtr = effectJson.toNativeUtf8();

    try {
      final result = ffi.rs_barrage_send(
        _handle,
        textPtr,
        trackType,
        color,
        fontSize,
        alpha,
        durationMs,
        effectPtr,
      );
      return result;
    } catch (e) {
      print('sendBarrage 调用失败: $e');
      return -1;
    } finally {
      malloc.free(textPtr);
      malloc.free(effectPtr);
    }
  }

  // ========================================================================
  // 更新与渲染
  // ========================================================================

  /// 每帧更新引擎状态
  ///
  /// [deltaMs] 距离上一帧的毫秒数
  /// 返回 0 表示成功，负数表示失败
  int update(int deltaMs) {
    if (!isActive) return -1;
    try {
      return ffi.rs_barrage_update(_handle, deltaMs);
    } catch (e) {
      print('update 调用失败: $e');
      return -1;
    }
  }

  /// 渲染到 RGBA 缓冲区
  ///
  /// [buffer] RGBA8888 格式的像素缓冲区（Uint8List）
  /// [width] 渲染宽度
  /// [height] 渲染高度
  /// 注意：buffer 的长度必须至少为 width * height * 4
  /// 返回 0 表示成功，负数表示失败
  int render(Uint8List buffer, int width, int height) {
    if (!isActive) return -1;
    try {
      // 将 Dart Uint8List 数据拷到 Native 指针
      final bufLen = width * height * 4;
      // 使用 Pointer<Uint8> 从 Dart 数据的 elementData 获取地址
      // 或使用 malloc 分配再拷贝
      // 这里用更安全的方式：通过 typedData 的 buffer 获取
      return ffi.rs_barrage_render(_handle, buffer.asPointer(), width, height);
    } catch (e) {
      print('render 调用失败: $e');
      return -1;
    }
  }

  // ========================================================================
  // 播放控制
  // ========================================================================

  /// 设置播放速度
  ///
  /// [speed] 速度倍率，范围 0.25 ~ 4.0
  /// 返回 0 表示成功，负数表示失败
  int setSpeed(double speed) {
    if (!isActive) return -1;
    try {
      return ffi.rs_barrage_set_speed(_handle, speed);
    } catch (e) {
      print('setSpeed 调用失败: $e');
      return -1;
    }
  }

  /// 暂停弹幕播放
  /// 返回 0 表示成功，负数表示失败
  int pause() {
    if (!isActive) return -1;
    try {
      return ffi.rs_barrage_pause(_handle);
    } catch (e) {
      print('pause 调用失败: $e');
      return -1;
    }
  }

  /// 恢复弹幕播放
  /// 返回 0 表示成功，负数表示失败
  int resume() {
    if (!isActive) return -1;
    try {
      return ffi.rs_barrage_resume(_handle);
    } catch (e) {
      print('resume 调用失败: $e');
      return -1;
    }
  }

  /// 跳转到指定时间位置
  ///
  /// [timeMs] 目标时间（毫秒）
  /// 返回 0 表示成功，负数表示失败
  int seek(int timeMs) {
    if (!isActive) return -1;
    try {
      return ffi.rs_barrage_seek(_handle, timeMs);
    } catch (e) {
      print('seek 调用失败: $e');
      return -1;
    }
  }

  /// 清空所有弹幕
  /// 返回 0 表示成功，负数表示失败
  int clear() {
    if (!isActive) return -1;
    try {
      return ffi.rs_barrage_clear(_handle);
    } catch (e) {
      print('clear 调用失败: $e');
      return -1;
    }
  }

  // ========================================================================
  // 特效配置
  // ========================================================================

  /// 设置全局默认特效
  ///
  /// 设置后，不传 effectConfig 的弹幕将使用此默认特效。
  /// 返回 0 表示成功，负数表示失败
  int setGlobalEffect(TextEffectConfig config) {
    if (!isActive) return -1;
    final jsonPtr = config.toJson().toNativeUtf8();
    try {
      return ffi.rs_barrage_set_global_effect(_handle, jsonPtr);
    } catch (e) {
      print('setGlobalEffect 调用失败: $e');
      return -1;
    } finally {
      malloc.free(jsonPtr);
    }
  }

  // ========================================================================
  // 过滤与黑名单
  // ========================================================================

  /// 添加黑名单关键词
  ///
  /// 包含黑名单词的弹幕将被过滤不显示。
  /// 返回 0 表示成功，负数表示失败
  int addBlacklistKeyword(String keyword) {
    if (!isActive) return -1;
    final kwPtr = keyword.toNativeUtf8();
    try {
      return ffi.rs_barrage_add_blacklist(_handle, kwPtr);
    } catch (e) {
      print('addBlacklistKeyword 调用失败: $e');
      return -1;
    } finally {
      malloc.free(kwPtr);
    }
  }

  /// 设置过滤器
  ///
  /// [filterJson] JSON 格式的过滤器配置字符串
  /// 返回 0 表示成功，负数表示失败
  int setFilter(String filterJson) {
    if (!isActive) return -1;
    final filterPtr = filterJson.toNativeUtf8();
    try {
      return ffi.rs_barrage_set_filter(_handle, filterPtr);
    } catch (e) {
      print('setFilter 调用失败: $e');
      return -1;
    } finally {
      malloc.free(filterPtr);
    }
  }
}

/// Uint8List 的扩展方法，用于获取 Native 指针
extension Uint8ListPointer on Uint8List {
  /// 将 Uint8List 中的数据映射为 Pointer<Uint8>
  /// 注意：此操作直接操作 Dart 内存，需要确保在 FFI 调用期间
  /// Dart GC 不会移动或回收该内存
  Pointer<Uint8> asPointer() {
    return Pointer<Uint8>.fromAddress(this.buffer.addressInScope);
  }
}