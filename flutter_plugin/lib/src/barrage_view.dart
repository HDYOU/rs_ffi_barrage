/// 弹幕渲染 Widget
///
/// BarrageView 是一个 StatelessWidget，接收 [BarrageEngine] 输出的 RGBA 像素流，
/// 通过 Timer 周期性调用 engine.render() 并将结果绘制到 RawImage 上展示。
///
/// 渲染流程：
/// 1. Timer 按指定 FPS 触发渲染 tick
/// 2. 调用 engine.update() 更新弹幕状态
/// 3. 调用 engine.render() 渲染到 RGBA 缓冲区
/// 4. 通过 decodeImageFromPixels 转为 ui.Image
/// 5. 使用 RawImage widget 展示

import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'barrage_engine.dart';

/// 弹幕渲染组件
///
/// 将 Rust FFI 引擎渲染输出的 RGBA 数据实时显示为 Flutter Widget。
class BarrageView extends StatefulWidget {
  /// 弹幕引擎实例
  final BarrageEngine engine;

  /// 渲染宽度
  final double width;

  /// 渲染高度
  final double height;

  /// 渲染帧率，默认 60
  final int fps;

  /// 背景色（RGBA 格式），默认 0x00000000（透明）
  final int backgroundColor;

  const BarrageView({
    super.key,
    required this.engine,
    required this.width,
    required this.height,
    this.fps = 60,
    this.backgroundColor = 0x00000000,
  });

  @override
  State<BarrageView> createState() => _BarrageViewState();
}

class _BarrageViewState extends State<BarrageView> {
  /// 当前帧的 ui.Image，用于 RawImage 渲染
  ui.Image? _currentFrame;

  /// 渲染计时器
  Timer? _renderTimer;

  /// RGBA 像素缓冲区
  Uint8List? _buffer;

  /// 缓冲区的宽度
  int _bufferWidth = 0;

  /// 缓冲区高度
  int _bufferHeight = 0;

  /// 帧间隔（毫秒）
  int get _frameIntervalMs => (1000 / widget.fps).round();

  @override
  void initState() {
    super.initState();
    // 初始化缓冲区
    _allocateBuffer();
    // 启动渲染循环
    _startRendering();
  }

  @override
  void didUpdateWidget(BarrageView oldWidget) {
    super.didUpdateWidget(oldWidget);
    // 如果尺寸变化，重新分配缓冲区
    if (oldWidget.width != widget.width || oldWidget.height != widget.height) {
      _allocateBuffer();
    }
    // 如果 FPS 变化，重启计时器
    if (oldWidget.fps != widget.fps) {
      _renderTimer?.cancel();
      _startRendering();
    }
  }

  @override
  void dispose() {
    // 停止渲染定时器
    _renderTimer?.cancel();
    // 释放 ui.Image 资源
    _currentFrame?.dispose();
    super.dispose();
  }

  /// 分配 RGBA 渲染缓冲区
  void _allocateBuffer() {
    final w = widget.width.ceil();
    final h = widget.height.ceil();
    _bufferWidth = w;
    _bufferHeight = h;
    // RGBA 每个像素 4 字节
    _buffer = Uint8List(w * h * 4);
  }

  /// 启动周期渲染循环
  void _startRendering() {
    _renderTimer = Timer.periodic(
      Duration(milliseconds: _frameIntervalMs),
      (_) => _renderFrame(),
    );
  }

  /// 渲染一帧：更新引擎 → 渲染到缓冲区 → 转为 ui.Image
  Future<void> _renderFrame() async {
    // 检查引擎是否可用
    if (!widget.engine.isActive) {
      return;
    }
    // 检查缓冲区
    if (_buffer == null || _bufferWidth <= 0 || _bufferHeight <= 0) {
      return;
    }

    try {
      // 步骤1：更新引擎状态（delta 为帧间隔毫秒数）
      final updateResult = widget.engine.update(_frameIntervalMs);
      if (updateResult != 0) {
        // 更新失败，跳过本帧
        return;
      }

      // 步骤2：渲染到 RGBA 缓冲区
      final renderResult = widget.engine.render(
        _buffer!,
        _bufferWidth,
        _bufferHeight,
      );
      if (renderResult != 0) {
        // 渲染失败，跳过本帧
        return;
      }

      // 步骤3：将 RGBA 像素数据解码为 ui.Image
      final completer = Completer<ui.Image>();
      ui.decodeImageFromPixels(
        _buffer!,
        _bufferWidth,
        _bufferHeight,
        ui.PixelFormat.rgba8888,
        (ui.Image image) {
          completer.complete(image);
        },
      );
      final image = await completer.future;

      // 在主 Isolate 中更新 UI
      if (mounted) {
        setState(() {
          // 释放上一帧的图片资源
          _currentFrame?.dispose();
          _currentFrame = image;
        });
      } else {
        // widget 已销毁，直接释放图片
        image.dispose();
      }
    } catch (e) {
      print('BarrageView 渲染帧失败: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      width: widget.width,
      height: widget.height,
      decoration: BoxDecoration(
        color: Color(widget.backgroundColor),
      ),
      child: _currentFrame != null
          ? RawImage(
              image: _currentFrame,
              width: widget.width,
              height: widget.height,
              fit: BoxFit.fill,
              // 保持像素精确渲染
              filterQuality: FilterQuality.low,
            )
          : const Center(
              child: Text(
                '弹幕渲染区域',
                style: TextStyle(color: Colors.grey),
              ),
            ),
    );
  }
}