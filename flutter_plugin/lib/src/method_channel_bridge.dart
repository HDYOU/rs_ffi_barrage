/// MethodChannel 兼容封装
///
/// 简易封装，通过 MethodChannel 调用 Rust 功能。
/// 主推 FFI 直调，此类作为备选兼容方案提供。
/// 当 FFI 动态库加载失败或特定平台不支持 FFI 时使用。

import 'package:flutter/services.dart';

/// MethodChannel 桥接器
///
/// 使用 Flutter 标准的 MethodChannel 机制与平台端通信。
/// 接口签名与 [BarrageEngine] 保持一致，方便切换。
class MethodChannelBridge {
  /// MethodChannel 名称，与 Android/iOS 原生侧对应
  static const String _channelName = 'rs_ffi_barrage_bridge';

  /// MethodChannel 实例
  final MethodChannel _channel = const MethodChannel(_channelName);

  // ========================================================================
  // 引擎生命周期
  // ========================================================================

  /// 创建引擎
  Future<bool> createEngine({int maxTracks = 10, int poolCapacity = 500}) async {
    try {
      final result = await _channel.invokeMethod<bool>('createEngine', {
        'maxTracks': maxTracks,
        'poolCapacity': poolCapacity,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge createEngine 失败: $e');
      return false;
    }
  }

  /// 销毁引擎
  Future<bool> destroyEngine() async {
    try {
      final result = await _channel.invokeMethod<bool>('destroyEngine');
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge destroyEngine 失败: $e');
      return false;
    }
  }

  // ========================================================================
  // 弹幕管理
  // ========================================================================

  /// 发送弹幕
  Future<bool> sendBarrage({
    required String text,
    int trackType = 0,
    int color = 0xFFFFFFFF,
    double fontSize = 24.0,
    double alpha = 1.0,
    int durationMs = 5000,
    String? effectJson,
  }) async {
    try {
      final result = await _channel.invokeMethod<bool>('sendBarrage', {
        'text': text,
        'trackType': trackType,
        'color': color,
        'fontSize': fontSize,
        'alpha': alpha,
        'durationMs': durationMs,
        'effectJson': effectJson ?? '',
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge sendBarrage 失败: $e');
      return false;
    }
  }

  /// 更新引擎状态
  Future<bool> update(int deltaMs) async {
    try {
      final result = await _channel.invokeMethod<bool>('update', {
        'deltaMs': deltaMs,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge update 失败: $e');
      return false;
    }
  }

  /// 设置播放速度
  Future<bool> setSpeed(double speed) async {
    try {
      final result = await _channel.invokeMethod<bool>('setSpeed', {
        'speed': speed,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge setSpeed 失败: $e');
      return false;
    }
  }

  /// 暂停
  Future<bool> pause() async {
    try {
      final result = await _channel.invokeMethod<bool>('pause');
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge pause 失败: $e');
      return false;
    }
  }

  /// 恢复
  Future<bool> resume() async {
    try {
      final result = await _channel.invokeMethod<bool>('resume');
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge resume 失败: $e');
      return false;
    }
  }

  /// 清空弹幕
  Future<bool> clear() async {
    try {
      final result = await _channel.invokeMethod<bool>('clear');
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge clear 失败: $e');
      return false;
    }
  }

  // ========================================================================
  // Emoji 管理
  // ========================================================================

  /// 注册本地文件贴图
  Future<bool> registerEmojiFile(String emojiId, String filePath) async {
    try {
      final result = await _channel.invokeMethod<bool>('registerEmojiFile', {
        'emojiId': emojiId,
        'filePath': filePath,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge registerEmojiFile 失败: $e');
      return false;
    }
  }

  /// 注册网络 URL 贴图
  Future<bool> registerEmojiUrl(String emojiId, String url) async {
    try {
      final result = await _channel.invokeMethod<bool>('registerEmojiUrl', {
        'emojiId': emojiId,
        'url': url,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge registerEmojiUrl 失败: $e');
      return false;
    }
  }

  /// 移除贴图
  Future<bool> removeEmoji(String emojiId) async {
    try {
      final result = await _channel.invokeMethod<bool>('removeEmoji', {
        'emojiId': emojiId,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge removeEmoji 失败: $e');
      return false;
    }
  }

  /// 清空贴图缓存
  Future<bool> clearEmojiCache() async {
    try {
      final result = await _channel.invokeMethod<bool>('clearEmojiCache');
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge clearEmojiCache 失败: $e');
      return false;
    }
  }

  // ========================================================================
  // 特效配置
  // ========================================================================

  /// 设置全局特效
  Future<bool> setGlobalEffect(String effectJson) async {
    try {
      final result = await _channel.invokeMethod<bool>('setGlobalEffect', {
        'effectJson': effectJson,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge setGlobalEffect 失败: $e');
      return false;
    }
  }

  /// 添加黑名单词
  Future<bool> addBlacklistKeyword(String keyword) async {
    try {
      final result = await _channel.invokeMethod<bool>('addBlacklistKeyword', {
        'keyword': keyword,
      });
      return result ?? false;
    } catch (e) {
      print('MethodChannelBridge addBlacklistKeyword 失败: $e');
      return false;
    }
  }
}