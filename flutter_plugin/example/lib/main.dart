/// rs_ffi_barrage 插件演示工程主入口
///
/// 包含四个演示场景：
/// 1. 贴图查询演示 — 调用 getEmojiBitmapFromFlutter 查询并展示贴图
/// 2. 三类 Emoji 注册演示 — Flutter位图 / 本地Assets / 远程URL
/// 3. 文字特效演示 — 描边/阴影/霓虹/彩虹/全特效
/// 4. 混合渲染演示 — 文字 + 贴图 + 特效混合

import 'dart:math';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show rootBundle;
import 'package:http/http.dart' as http;
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

void main() {
  runApp(const BarrageDemoApp());
}

/// 演示应用根组件
class BarrageDemoApp extends StatelessWidget {
  const BarrageDemoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'rs_ffi_barrage 演示',
      theme: ThemeData(
        colorSchemeSeed: Colors.blue,
        useMaterial3: true,
        brightness: Brightness.light,
      ),
      home: const DemoHomePage(),
      debugShowCheckedModeBanner: false,
    );
  }
}

/// 演示主页 —— 使用 TabBar 切换四个场景
class DemoHomePage extends StatefulWidget {
  const DemoHomePage({super.key});

  @override
  State<DemoHomePage> createState() => _DemoHomePageState();
}

class _DemoHomePageState extends State<DemoHomePage>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 4, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('rs_ffi_barrage 演示'),
        bottom: TabBar(
          controller: _tabController,
          isScrollable: true,
          tabs: const [
            Tab(text: '贴图查询'),
            Tab(text: 'Emoji注册'),
            Tab(text: '文字特效'),
            Tab(text: '混合渲染'),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: const [
          EmojiQueryDemoScene(),      // 场景1：贴图查询演示
          EmojiRegisterDemoScene(),   // 场景2：三类Emoji注册演示
          TextEffectDemoScene(),      // 场景3：文字特效演示
          MixedRenderDemoScene(),     // 场景4：混合渲染演示
        ],
      ),
    );
  }
}

// ============================================================================
// 场景1：贴图查询演示
// ============================================================================

/// 场景1 —— 贴图查询演示
///
/// 调用 getEmojiBitmapFromFlutter("666") 和 getEmojiBitmapFromFlutter("好")
/// 获取贴图 RGBA 数据，使用 rgbaBytesToImage 转为 ui.Image 后，
/// 通过 RawImage 展示二次渲染结果，同时显示查询状态和贴图宽高信息。
class EmojiQueryDemoScene extends StatefulWidget {
  const EmojiQueryDemoScene({super.key});

  @override
  State<EmojiQueryDemoScene> createState() => _EmojiQueryDemoSceneState();
}

class _EmojiQueryDemoSceneState extends State<EmojiQueryDemoScene> {
  /// 引擎实例
  BarrageEngine? _engine;

  /// 查询结果：贴图ID 对应的 ui.Image
  final Map<String, ui.Image?> _queriedImages = {};

  /// 查询结果：贴图ID 对应的宽高信息
  final Map<String, String> _imageInfo = {};

  /// 查询状态：贴图ID 对应的状态文本
  final Map<String, String> _queryStatus = {};

  /// 是否正在查询
  bool _isQuerying = false;

  /// 待查询的贴图标识列表
  final List<String> _emojiTexts = ['666', '好'];

  @override
  void initState() {
    super.initState();
    _initEngine();
  }

  @override
  void dispose() {
    // 释放所有查询到的 ui.Image
    for (final image in _queriedImages.values) {
      image?.dispose();
    }
    // 释放引擎资源
    _engine?.dispose();
    super.dispose();
  }

  /// 初始化引擎（需 Rust 动态库支持）
  Future<void> _initEngine() async {
    try {
      _engine = BarrageEngine(maxTracks: 10, poolCapacity: 200);
    } catch (e) {
      print('引擎初始化失败: $e（在无 Rust 库的环境中属于正常情况）');
    }
  }

  /// 执行贴图查询
  Future<void> _performQuery(String emojiText) async {
    if (_engine == null) {
      setState(() {
        _queryStatus[emojiText] = '引擎未初始化';
      });
      return;
    }

    setState(() {
      _isQuerying = true;
      _queryStatus[emojiText] = '查询中...';
    });

    try {
      // 步骤1：调用 FFI 两步查询获取 RGBA 像素数据
      // 第一步：rs_barrage_get_emoji_info 查询宽高和大小
      // 第二步：rs_barrage_copy_emoji_bitmap 拷贝像素
      final rgbaData = _engine!.emojiManager.getEmojiBitmapFromFlutter(emojiText);

      if (rgbaData == null) {
        setState(() {
          _queryStatus[emojiText] = '未找到贴图（返回 null）';
          _imageInfo[emojiText] = '无';
          _isQuerying = false;
        });
        return;
      }

      // 从数据推断宽高信息（RGBA8888 格式，每像素4字节）
      // 注意：实际宽高应在应用层维护，此处演示推算逻辑
      final totalPixels = rgbaData.length ~/ 4;
      // 假设为正方形，取近似边长
      final approxSize = _guessImageSize(totalPixels);

      setState(() {
        _queryStatus[emojiText] = '找到贴图！数据大小: ${rgbaData.length} 字节';
        _imageInfo[emojiText] = '${approxSize.width}x${approxSize.height} (估算)';
      });

      // 步骤2：将 RGBA 数据转为 ui.Image 用于展示
      final image = await rgbaBytesToImage(rgbaData, approxSize.width, approxSize.height);

      if (mounted) {
        setState(() {
          if (image != null) {
            _queriedImages[emojiText] = image;
            // 更新精确宽高信息
            _imageInfo[emojiText] = '${image.width}x${image.height}';
          } else {
            _queryStatus[emojiText] = 'RGBA转Image失败';
          }
          _isQuerying = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _queryStatus[emojiText] = '查询出错: $e';
          _isQuerying = false;
        });
      }
    }
  }

  /// 根据像素数估算图片尺寸（正方形近似）
  _ImageSize _guessImageSize(int totalPixels) {
    if (totalPixels <= 0) return _ImageSize(0, 0);
    final side = sqrt(totalPixels).round();
    return _ImageSize(side, side);
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // 说明卡片
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '场景1：贴图查询演示',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                const Text(
                  '调用 getEmojiBitmapFromFlutter() 执行两步 FFI 调用'
                  '（查信息 → 拷像素），获取贴图 RGBA 数据并渲染展示。',
                  style: TextStyle(color: Colors.grey),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        // 查询按钮
        Center(
          child: ElevatedButton.icon(
            onPressed: _isQuerying
                ? null
                : () {
                    for (final text in _emojiTexts) {
                      _performQuery(text);
                    }
                  },
            icon: const Icon(Icons.search),
            label: const Text('查询贴图（"666" 和 "好"）'),
          ),
        ),
        const SizedBox(height: 16),

        // 查询结果展示
        for (final emojiText in _emojiTexts) ...[
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // 贴图标识
                  Text(
                    '贴图标识: "$emojiText"',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 8),

                  // 查询状态
                  Row(
                    children: [
                      const Text('查询状态：'),
                      Text(
                        _queryStatus[emojiText] ?? '未查询',
                        style: TextStyle(
                          color: (_queryStatus[emojiText]?.contains('找到') ?? false)
                              ? Colors.green
                              : Colors.grey,
                        ),
                      ),
                    ],
                  ),

                  // 宽高信息
                  if (_imageInfo.containsKey(emojiText))
                    Padding(
                      padding: const EdgeInsets.only(top: 4),
                      child: Text('宽高信息：${_imageInfo[emojiText]}'),
                    ),

                  const SizedBox(height: 12),

                  // 渲染结果
                  if (_queriedImages.containsKey(emojiText) &&
                      _queriedImages[emojiText] != null)
                    Center(
                      child: RawImage(
                        image: _queriedImages[emojiText],
                        width: 100,
                        height: 100,
                        fit: BoxFit.contain,
                      ),
                    )
                  else
                    Container(
                      width: double.infinity,
                      height: 80,
                      decoration: BoxDecoration(
                        color: Colors.grey[200],
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Center(
                        child: Text('暂无贴图数据', style: TextStyle(color: Colors.grey)),
                      ),
                    ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 8),
        ],
      ],
    );
  }
}

/// 简单尺寸辅助类
class _ImageSize {
  final int width;
  final int height;
  const _ImageSize(this.width, this.height);
}

// ============================================================================
// 场景2：三类 Emoji 注册演示
// ============================================================================

/// 场景2 —— 三类 Emoji 注册演示
///
/// 包含三个子 Tab：
/// - Tab1: Flutter 动态位图注册 — 从网络加载图片，转为 RGBA 注册
/// - Tab2: 本地 Assets 文件注册 — 从 assets/ 目录加载注册
/// - Tab3: 远程网络 URL 注册 — 输入 URL 注册贴图
class EmojiRegisterDemoScene extends StatefulWidget {
  const EmojiRegisterDemoScene({super.key});

  @override
  State<EmojiRegisterDemoScene> createState() => _EmojiRegisterDemoSceneState();
}

class _EmojiRegisterDemoSceneState extends State<EmojiRegisterDemoScene>
    with SingleTickerProviderStateMixin {
  /// 引擎实例
  BarrageEngine? _engine;

  /// Tab 控制器
  late TabController _subTabController;

  /// 注册结果文本
  String _registerResult = '等待操作...';

  @override
  void initState() {
    super.initState();
    _subTabController = TabController(length: 3, vsync: this);
    try {
      _engine = BarrageEngine(maxTracks: 5, poolCapacity: 100);
    } catch (e) {
      print('引擎初始化失败: $e');
    }
  }

  @override
  void dispose() {
    _subTabController.dispose();
    _engine?.dispose();
    super.dispose();
  }

  /// 更新注册结果
  void _setResult(String msg) {
    if (mounted) {
      setState(() => _registerResult = msg);
    }
  }

  /// Tab1：从网络下载图片，转为 RGBA 注册
  Future<void> _registerFromNetworkUrl() async {
    _setResult('正在从网络下载图片...');
    try {
      // 使用一个公开的测试图片
      final url = 'https://picsum.photos/64/64'; // 随机 64x64 图片
      final response = await http.get(Uri.parse(url));
      if (response.statusCode != 200) {
        _setResult('网络请求失败: HTTP ${response.statusCode}');
        return;
      }

      // 将下载的字节数据解码为 ui.Image
      final codec = await ui.instantiateImageCodec(response.bodyBytes);
      final frameInfo = await codec.getNextFrame();
      final image = frameInfo.image;

      // 将 ui.Image 转为 RGBA Uint8List
      final rgbaData = await imageToRgbaBytes(image);
      if (rgbaData == null) {
        _setResult('图片转 RGBA 失败');
        return;
      }

      // 调用引擎注册
      if (_engine == null) {
        _setResult('引擎未初始化');
        return;
      }

      final result = _engine!.emojiManager.registerEmojiFromFlutterBitmap(
        'net_emoji',
        rgbaData,
        image.width,
        image.height,
      );

      _setResult(
        result == 0
            ? '注册成功！标识: net_emoji, 尺寸: ${image.width}x${image.height}'
            : '注册失败，返回码: $result',
      );

      // 释放图片资源
      image.dispose();
      codec.dispose();
    } catch (e) {
      _setResult('网络注册出错: $e');
    }
  }

  /// Tab2：从 Assets 文件注册
  Future<void> _registerFromAsset() async {
    _setResult('正在从 Assets 加载图片...');
    try {
      // 加载 assets 中的图片
      final byteData = await rootBundle.load('assets/emoji_heart.png');
      final codec = await ui.instantiateImageCodec(byteData.buffer.asUint8List());
      final frameInfo = await codec.getNextFrame();
      final image = frameInfo.image;

      // 转为 RGBA
      final rgbaData = await imageToRgbaBytes(image);
      if (rgbaData == null) {
        _setResult('Assets 图片转 RGBA 失败');
        return;
      }

      if (_engine == null) {
        _setResult('引擎未初始化');
        return;
      }

      final result = _engine!.emojiManager.registerEmojiFromFlutterBitmap(
        'heart',
        rgbaData,
        image.width,
        image.height,
      );

      _setResult(
        result == 0
            ? '注册成功！标识: heart, 尺寸: ${image.width}x${image.height}'
            : '注册失败，返回码: $result',
      );

      image.dispose();
      codec.dispose();
    } catch (e) {
      _setResult('Assets 注册出错 (请确保 assets/emoji_heart.png 存在): $e');
    }
  }

  /// Tab3：从远程 URL 注册（引擎内部下载）
  void _registerFromUrlInput(String url) {
    if (url.isEmpty) {
      _setResult('请输入有效的 URL');
      return;
    }

    if (_engine == null) {
      _setResult('引擎未初始化');
      return;
    }

    _setResult('正在调用引擎注册 URL: $url');
    final result = _engine!.emojiManager.registerEmojiFromUrl('url_emoji', url);
    _setResult(result == 0 ? 'URL 注册成功！标识: url_emoji' : 'URL 注册失败，返回码: $result');
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // 说明卡片
        Card(
          margin: const EdgeInsets.all(16),
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '场景2：三类 Emoji 注册演示',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                const Text(
                  '三种注册方式：Flutter动态位图 / 本地Assets文件 / 远程网络URL',
                  style: TextStyle(color: Colors.grey),
                ),
              ],
            ),
          ),
        ),

        // 子 TabBar
        TabBar(
          controller: _subTabController,
          tabs: const [
            Tab(text: 'Flutter位图'),
            Tab(text: '本地Assets'),
            Tab(text: '远程URL'),
          ],
        ),

        // 注册结果展示
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Text(
            _registerResult,
            style: TextStyle(
              color: _registerResult.contains('成功') ? Colors.green : Colors.blueGrey,
            ),
          ),
        ),
        // 使用 Expanded 让 TabBarView 占满剩余空间
        Expanded(
          child: TabBarView(
            controller: _subTabController,
            children: [
              // Tab1: Flutter动态位图注册
              _buildTabFlutterBitmap(),
              // Tab2: 本地Assets注册
              _buildTabAsset(),
              // Tab3: 远程URL注册
              _buildTabUrl(),
            ],
          ),
        ),
      ],
    );
  }

  /// Tab1 内容：Flutter 动态位图注册
  Widget _buildTabFlutterBitmap() {
    return Center(
      child: ElevatedButton.icon(
        onPressed: _registerFromNetworkUrl,
        icon: const Icon(Icons.download),
        label: const Text('从网络下载图片并注册'),
      ),
    );
  }

  /// Tab2 内容：本地 Assets 注册
  Widget _buildTabAsset() {
    return Center(
      child: ElevatedButton.icon(
        onPressed: _registerFromAsset,
        icon: const Icon(Icons.folder),
        label: const Text('从 assets/emoji_heart.png 注册'),
      ),
    );
  }

  /// Tab3 内容：远程 URL 注册
  Widget _buildTabUrl() {
    final urlController = TextEditingController(
      text: 'https://picsum.photos/64/64',
    );
    return Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          TextField(
            controller: urlController,
            decoration: const InputDecoration(
              labelText: '图片 URL',
              hintText: '输入远程图片地址',
              border: OutlineInputBorder(),
              prefixIcon: Icon(Icons.link),
            ),
          ),
          const SizedBox(height: 16),
          ElevatedButton.icon(
            onPressed: () => _registerFromUrlInput(urlController.text),
            icon: const Icon(Icons.cloud_upload),
            label: const Text('注册此 URL 贴图'),
          ),
        ],
      ),
    );
  }
}

// ============================================================================
// 场景3：文字特效演示
// ============================================================================

/// 场景3 —— 文字特效演示
///
/// 展示五种预设特效的弹幕示例：
/// - 描边效果
/// - 立体阴影效果（浮雕阴影）
/// - 霓虹发光效果
/// - 彩虹渐变效果
/// - 全特效混合（彩虹渐变 + 描边 + 阴影 + 霓虹）
class TextEffectDemoScene extends StatefulWidget {
  const TextEffectDemoScene({super.key});

  @override
  State<TextEffectDemoScene> createState() => _TextEffectDemoSceneState();
}

class _TextEffectDemoSceneState extends State<TextEffectDemoScene> {
  /// 引擎实例
  BarrageEngine? _engine;

  /// 最后一次发送结果
  String _lastResult = '点击按钮发送特效弹幕';

  @override
  void initState() {
    super.initState();
    try {
      _engine = BarrageEngine(maxTracks: 10, poolCapacity: 200);
    } catch (e) {
      print('引擎初始化失败: $e');
    }
  }

  @override
  void dispose() {
    _engine?.dispose();
    super.dispose();
  }

  /// 发送一条特效弹幕
  void _sendEffectBarrage(String text, TextEffectConfig config, String effectName) {
    if (_engine == null) {
      setState(() => _lastResult = '引擎未初始化');
      return;
    }

    final result = _engine!.sendBarrage(
      text,
      trackType: 3, // 滚动弹幕
      color: 0xFFFFFFFF,
      fontSize: 28.0,
      alpha: 1.0,
      durationMs: 8000,
      effectConfig: config,
    );

    setState(() {
      _lastResult = result == 0 ? '$effectName 发送成功！' : '$effectName 发送失败，返回码: $result';
    });
  }

  /// 特效配置列表
  List<_EffectDemoItem> get _effectItems => [
    _EffectDemoItem(
      name: '描边效果',
      description: '白色描边，宽度 3.0，柔边开启',
      text: '【描边】Hello 弹幕!',
      config: TextEffectConfig.outlineOnly(),
    ),
    _EffectDemoItem(
      name: '立体阴影效果',
      description: '6层浮雕阴影，立体感强',
      text: '【阴影】立体弹幕!',
      config: TextEffectConfig.embossShadow(),
    ),
    _EffectDemoItem(
      name: '霓虹发光效果',
      description: '单色蓝辉光，半径 8.0，强度 0.7',
      text: '【霓虹】Glow 弹幕!',
      config: TextEffectConfig.neonGlow(),
    ),
    _EffectDemoItem(
      name: '彩虹渐变效果',
      description: '红橙黄绿青蓝紫 7色 90度线性渐变',
      text: '【彩虹】Colorful 弹幕!',
      config: TextEffectConfig.rainbowGradient(),
    ),
    _EffectDemoItem(
      name: '全特效混合',
      description: '彩虹渐变 + 描边 + 阴影 + 霓虹光',
      text: '【全特效】Super 弹幕!',
      config: TextEffectConfig.allEffects(),
    ),
  ];

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // 说明卡片
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '场景3：文字特效演示',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                const Text(
                  '使用 TextEffectConfig 预设创建各种特效弹幕。'
                  '点击按钮通过 FFI 发送到引擎，目前为演示 UI 展示。',
                  style: TextStyle(color: Colors.grey),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        // 最近结果
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Colors.blue[50],
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text('最近操作: $_lastResult'),
        ),
        const SizedBox(height: 16),

        // 特效列表
        for (final item in _effectItems) ...[
          Card(
            child: ListTile(
              title: Text(item.name),
              subtitle: Text(item.description),
              trailing: ElevatedButton(
                onPressed: () => _sendEffectBarrage(item.text, item.config, item.name),
                child: const Text('发送'),
              ),
            ),
          ),
          const SizedBox(height: 8),
        ],
      ],
    );
  }
}

/// 特效演示项数据
class _EffectDemoItem {
  final String name;
  final String description;
  final String text;
  final TextEffectConfig config;

  const _EffectDemoItem({
    required this.name,
    required this.description,
    required this.text,
    required this.config,
  });
}

// ============================================================================
// 场景4：混合渲染演示
// ============================================================================

/// 场景4 —— 混合渲染演示
///
/// 弹幕包含文字 + [666] 贴图引用 + 特效的混合渲染。
/// 展示 BarrageView 组件实时渲染效果。
class MixedRenderDemoScene extends StatefulWidget {
  const MixedRenderDemoScene({super.key});

  @override
  State<MixedRenderDemoScene> createState() => _MixedRenderDemoSceneState();
}

class _MixedRenderDemoSceneState extends State<MixedRenderDemoScene> {
  /// 引擎实例
  BarrageEngine? _engine;

  /// 是否已启动渲染
  bool _isRendering = false;

  /// 渲染日志
  final List<String> _logs = [];

  @override
  void initState() {
    super.initState();
    try {
      _engine = BarrageEngine(maxTracks: 10, poolCapacity: 300);
      _addLog('引擎初始化成功');
    } catch (e) {
      _addLog('引擎初始化失败: $e（在无 Rust 库环境中属于正常表现）');
    }
  }

  @override
  void dispose() {
    _engine?.dispose();
    super.dispose();
  }

  void _addLog(String msg) {
    if (mounted) {
      setState(() => _logs.insert(0, '[${DateTime.now().second}s] $msg'));
    }
  }

  /// 发送混合弹幕
  void _sendMixedBarrage() {
    if (_engine == null) {
      _addLog('引擎未初始化，无法发送');
      return;
    }

    // 使用全特效 + 贴图引用的混合弹幕
    final result = _engine!.sendBarrage(
      '🔥 混合弹幕 [666] 太厉害了! ✨',
      trackType: 3,
      color: 0xFFFFFF00,
      fontSize: 24.0,
      alpha: 1.0,
      durationMs: 10000,
      effectConfig: TextEffectConfig.allEffects(),
    );

    _addLog(
      result == 0 ? '混合弹幕发送成功！' : '发送失败，返回码: $result',
    );
  }

  /// 发送简单弹幕
  void _sendSimpleBarrage() {
    if (_engine == null) {
      _addLog('引擎未初始化，无法发送');
      return;
    }

    final result = _engine!.sendBarrage(
      '普通弹幕测试文字',
      trackType: 0,
      color: 0xFFFFFFFF,
      fontSize: 20.0,
    );

    _addLog(
      result == 0 ? '简单弹幕发送成功！' : '发送失败，返回码: $result',
    );
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // 说明卡片
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '场景4：混合渲染演示',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                const Text(
                  '弹幕包含文字 + [666] 贴图 + 全特效的混合渲染。'
                  '使用 BarrageView 实时展示引擎渲染输出。',
                  style: TextStyle(color: Colors.grey),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        // 渲染区域
        Container(
          width: double.infinity,
          height: 200,
          decoration: BoxDecoration(
            color: Colors.black,
            borderRadius: BorderRadius.circular(12),
          ),
          child: _engine != null && _engine!.isActive
              ? ClipRRect(
                  borderRadius: BorderRadius.circular(12),
                  child: BarrageView(
                    engine: _engine!,
                    width: MediaQuery.of(context).size.width - 32,
                    height: 200,
                    fps: 30,
                    backgroundColor: 0xFF1A1A2E,
                  ),
                )
              : const Center(
                  child: Text(
                    '引擎未初始化\n渲染区域（预览）',
                    style: TextStyle(color: Colors.grey),
                    textAlign: TextAlign.center,
                  ),
                ),
        ),
        const SizedBox(height: 16),

        // 控制按钮
        Row(
          children: [
            Expanded(
              child: ElevatedButton.icon(
                onPressed: _sendMixedBarrage,
                icon: const Icon(Icons.auto_awesome),
                label: const Text('发送混合弹幕'),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: OutlinedButton.icon(
                onPressed: _sendSimpleBarrage,
                icon: const Icon(Icons.text_fields),
                label: const Text('发送普通弹幕'),
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),

        // 引擎控制
        Row(
          children: [
            Expanded(
              child: TextButton.icon(
                onPressed: () {
                  _engine?.clear();
                  _addLog('清空弹幕');
                },
                icon: const Icon(Icons.clear_all),
                label: const Text('清空'),
              ),
            ),
            Expanded(
              child: TextButton.icon(
                onPressed: () {
                  _engine?.setSpeed(0.5);
                  _addLog('速度设为 0.5x');
                },
                icon: const Icon(Icons.slow_motion_video),
                label: const Text('0.5x'),
              ),
            ),
            Expanded(
              child: TextButton.icon(
                onPressed: () {
                  _engine?.setSpeed(2.0);
                  _addLog('速度设为 2.0x');
                },
                icon: const Icon(Icons.fast_forward),
                label: const Text('2.0x'),
              ),
            ),
            Expanded(
              child: TextButton.icon(
                onPressed: () {
                  _engine?.setSpeed(1.0);
                  _addLog('速度恢复 1.0x');
                },
                icon: const Icon(Icons.speed),
                label: const Text('1.0x'),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),

        // 操作日志
        Text(
          '操作日志',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        Container(
          height: 200,
          decoration: BoxDecoration(
            color: Colors.grey[100],
            borderRadius: BorderRadius.circular(8),
          ),
          child: ListView.builder(
            reverse: true,
            itemCount: _logs.length,
            itemBuilder: (context, index) {
              return Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                child: Text(
                  _logs[index],
                  style: const TextStyle(fontSize: 12, fontFamily: 'monospace'),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}