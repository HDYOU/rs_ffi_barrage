/// Emoji 管理器
///
/// 封装所有 Emoji 相关的 FFI 调用，提供三种注册方式（Flutter 位图、本地文件、网络 URL），
/// 以及贴图查询、移除、缓存管理功能。
///
/// 核心方法 [getEmojiBitmapFromFlutter] 完整执行两步骤 FFI 调用：
/// 1. rs_barrage_get_emoji_info — 查询贴图宽高和像素字节数
/// 2. rs_barrage_copy_emoji_bitmap — 分配缓冲区拷出像素数据

import 'dart:async';
import 'dart:ffi';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:ffi/ffi.dart';
import 'ffi_bindings.dart' as ffi;

/// 贴图管理器
///
/// 与 [BarrageEngine] 绑定，管理 Rust 侧贴图资源的注册与查询。
class EmojiManager {
  /// Rust 引擎句柄
  final ffi.RsHandle _engineHandle;

  /// 构造方法，需传入有效的引擎句柄
  EmojiManager(this._engineHandle);

  // ========================================================================
  // 注册方式一：从 Flutter 内存 RGBA 数据注册
  // ========================================================================

  /// 从 Flutter 内存 RGBA 数据注册贴图
  ///
  /// [emojiId] 贴图唯一标识，如 "666"、"好"
  /// [rgbaData] RGBA8888 格式的像素数据
  /// [width] 图片宽度
  /// [height] 图片高度
  /// 返回 0 表示成功，负数表示失败
  int registerEmojiFromFlutterBitmap(
    String emojiId,
    Uint8List rgbaData,
    int width,
    int height,
  ) {
    // 将 emojiId 转为 C 字符串
    final idPtr = emojiId.toNativeUtf8();
    // 分配像素数据缓冲区
    final rgbaPtr = malloc.allocate<Uint8>(rgbaData.length);
    // 拷贝 Dart 数据到 Native 内存
    for (var i = 0; i < rgbaData.length; i++) {
      rgbaPtr[i] = rgbaData[i];
    }

    try {
      final result = ffi.rs_barrage_register_emoji_bitmap(
        _engineHandle,
        idPtr,
        rgbaPtr,
        width,
        height,
      );
      return result;
    } catch (e) {
      print('registerEmojiFromFlutterBitmap 调用失败: $e');
      return -1;
    } finally {
      // 释放 Native 内存
      malloc.free(idPtr);
      malloc.free(rgbaPtr);
    }
  }

  // ========================================================================
  // 注册方式二：从本地文件注册
  // ========================================================================

  /// 从本地文件路径注册贴图
  ///
  /// [emojiId] 贴图唯一标识
  /// [filePath] 本地文件绝对路径，支持 PNG/JPG 等常见格式
  /// 返回 0 表示成功，负数表示失败
  int registerEmojiFromLocalPath(String emojiId, String filePath) {
    final idPtr = emojiId.toNativeUtf8();
    final pathPtr = filePath.toNativeUtf8();

    try {
      final result = ffi.rs_barrage_register_emoji_file(
        _engineHandle,
        idPtr,
        pathPtr,
      );
      return result;
    } catch (e) {
      print('registerEmojiFromLocalPath 调用失败: $e');
      return -1;
    } finally {
      malloc.free(idPtr);
      malloc.free(pathPtr);
    }
  }

  // ========================================================================
  // 注册方式三：从网络 URL 注册
  // ========================================================================

  /// 从网络 URL 注册贴图
  ///
  /// [emojiId] 贴图唯一标识
  /// [url] 远程图片 URL，引擎内部下载并解析
  /// 返回 0 表示成功，负数表示失败
  int registerEmojiFromUrl(String emojiId, String url) {
    final idPtr = emojiId.toNativeUtf8();
    final urlPtr = url.toNativeUtf8();

    try {
      final result = ffi.rs_barrage_register_emoji_url(
        _engineHandle,
        idPtr,
        urlPtr,
      );
      return result;
    } catch (e) {
      print('registerEmojiFromUrl 调用失败: $e');
      return -1;
    } finally {
      malloc.free(idPtr);
      malloc.free(urlPtr);
    }
  }

  // ========================================================================
  // 核心查询接口：两步 FFI 调用
  // ========================================================================

  /// 查询贴图并获取 RGBA 像素数据
  ///
  /// 入参 [emojiText] 为纯文本标识（如 "666" 或 "好"），不包含中括号。
  /// 内部执行两步骤 FFI 调用：
  ///   步骤1：rs_barrage_get_emoji_info — 查询贴图宽高和像素字节数
  ///   步骤2：rs_barrage_copy_emoji_bitmap — 分配缓冲区拷出 RGBA 像素数据
  ///
  /// 返回包含 RGBA 数据的 Uint8List，匹配不到或出错返回 null。
  /// 调用方可使用 [rgbaBytesToImage] 将返回数据转为 ui.Image 进行渲染展示。
  Uint8List? getEmojiBitmapFromFlutter(String emojiText) {
    try {
      // ---- 步骤1：查询贴图信息（宽、高、像素字节数） ----
      // 将 Dart 字符串转为 C 字符串（UTF-8）
      final textPtr = emojiText.toNativeUtf8();

      // 分配输出参数指针：宽、高、像素数据大小
      final outWidth = malloc<Uint32>();
      final outHeight = malloc<Uint32>();
      final outDataSize = malloc<Uint64>();

      // 调用 FFI 查询贴图信息
      final infoResult = ffi.rs_barrage_get_emoji_info(
        _engineHandle,
        textPtr,
        outWidth,
        outHeight,
        outDataSize,
      );

      // 检查查询结果
      if (infoResult != 0) {
        // 贴图不存在或查询失败，释放资源后返回 null
        malloc.free(textPtr);
        malloc.free(outWidth);
        malloc.free(outHeight);
        malloc.free(outDataSize);
        return null;
      }

      // 读取查询结果
      final width = outWidth.value;
      final height = outHeight.value;
      final dataSize = outDataSize.value;

      // 释放信息查询相关指针（textPtr 在步骤2 还需使用，暂不释放）
      malloc.free(outWidth);
      malloc.free(outHeight);
      malloc.free(outDataSize);

      // 校验数据合法性
      if (width == 0 || height == 0 || dataSize == 0) {
        malloc.free(textPtr);
        return null;
      }

      // ---- 步骤2：分配缓冲区，拷贝像素数据 ----
      // 在 Native 堆上分配足够容纳像素数据的缓冲区
      final bufferPtr = malloc.allocate<Uint8>(dataSize);

      // 调用 FFI 拷贝贴图像素数据到缓冲区
      final copyResult = ffi.rs_barrage_copy_emoji_bitmap(
        _engineHandle,
        textPtr,
        bufferPtr,
        dataSize,
      );

      // 释放 emojiText 的 C 字符串
      malloc.free(textPtr);

      if (copyResult != 0) {
        // 拷贝失败，释放缓冲区返回 null
        malloc.free(bufferPtr);
        return null;
      }

      // 将 Native 缓冲区数据拷到 Dart Uint8List
      final resultData = Uint8List(dataSize);
      for (var i = 0; i < dataSize; i++) {
        resultData[i] = bufferPtr[i];
      }

      // 释放 Native 缓冲区
      malloc.free(bufferPtr);

      return resultData;
    } catch (e) {
      print('getEmojiBitmapFromFlutter 查询失败 (emojiText="$emojiText"): $e');
      return null;
    }
  }

  // ========================================================================
  // 缓存管理方法
  // ========================================================================

  /// 移除指定贴图
  ///
  /// 从 Rust 引擎的贴图缓存中移除指定标识的贴图。
  /// 返回 0 表示成功，负数表示失败。
  int removeEmoji(String emojiId) {
    final idPtr = emojiId.toNativeUtf8();
    try {
      return ffi.rs_barrage_remove_emoji(_engineHandle, idPtr);
    } catch (e) {
      print('removeEmoji 调用失败 (emojiId="$emojiId"): $e');
      return -1;
    } finally {
      malloc.free(idPtr);
    }
  }

  /// 清空所有贴图缓存
  ///
  /// 释放 Rust 引擎中所有已注册贴图的内存。
  /// 返回 0 表示成功，负数表示失败。
  int clearCache() {
    try {
      return ffi.rs_barrage_clear_emoji_cache(_engineHandle);
    } catch (e) {
      print('clearCache 调用失败: $e');
      return -1;
    }
  }

  /// 获取当前贴图缓存总字节数
  ///
  /// 返回 Rust 引擎中所有贴图占用的内存字节数。
  /// 出错时返回 -1。
  int getCacheSize() {
    try {
      return ffi.rs_barrage_get_emoji_cache_size(_engineHandle);
    } catch (e) {
      print('getCacheSize 调用失败: $e');
      return -1;
    }
  }
}

// ============================================================================
// RGBA 转换工具函数
// ============================================================================

/// 将 ui.Image 转换为 Uint8List RGBA8888 格式
///
/// 使用 ui.Image 的 toByteData() 方法提取像素数据。
/// 返回 RGBA 格式的 Uint8List，失败时返回 null。
Future<Uint8List?> imageToRgbaBytes(ui.Image image) async {
  try {
    final byteData = await image.toByteData(
      format: ui.ImageByteFormat.rawRgba,
    );
    if (byteData == null) {
      print('imageToRgbaBytes: toByteData 返回 null');
      return null;
    }
    return byteData.buffer.asUint8List();
  } catch (e) {
    print('imageToRgbaBytes 转换失败: $e');
    return null;
  }
}

/// 将 RGBA 数据转换为 ui.Image
///
/// 用于将 FFI 查询返回的 RGBA 像素数据重新构造为 ui.Image，
/// 方便在 Flutter 中渲染展示。
/// [rgba] RGBA8888 格式的像素数据
/// [width] 图片宽度
/// [height] 图片高度
/// 返回 ui.Image，失败时返回 null。
Future<ui.Image?> rgbaBytesToImage(Uint8List rgba, int width, int height) async {
  try {
    // 将 Uint8List 转为编解码器可识别的 PNG 编码数据
    // 注意：直接使用 RGBA 数据需要借助 decodeImageFromPixels
    final completer = Completer<ui.Image>();
    ui.decodeImageFromPixels(
      rgba,
      width,
      height,
      ui.PixelFormat.rgba8888,
      (ui.Image img) {
        completer.complete(img);
      },
    );
    return await completer.future;
  } catch (e) {
    print('rgbaBytesToImage 转换失败: $e');
    return null;
  }
}

