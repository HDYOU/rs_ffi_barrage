# rs_ffi_barrage

**基于 Rust + C ABI FFI 的高性能全平台 Flutter 弹幕插件。**

rs_ffi_barrage 是一个使用 Rust 编写底层引擎、通过 C ABI FFI 向 Flutter 暴露接口的弹幕插件。它完整复刻了 DanmakuFlameMaster 风格弹幕引擎的核心逻辑，同时利用 Rust 的零成本抽象和无锁并发能力，在性能上实现了显著超越。

**自 Flutter 3.38 起，本项目采用 Dart Hooks 系统（原 native-assets）自动编译 Rust 原生代码，无需编写平台特定的构建文件（CMake/Podspec/build.gradle），支持所有平台。**

---

## 功能特性

- **完整弹幕轨道系统**
  - 4种弹幕轨道：正向滚动、顶部固定、底部固定、逆向滚动
  - 轨道碰撞避让 + 拥挤策略（排队/丢弃）
  - 弹幕对象内存池复用，消除 GC 卡顿

- **精准时间控制**
  - 毫秒级精准时间控制：0.25~4.0 倍速、Seek、暂停/恢复
  - 弹幕过滤：关键词黑名单、类型开关、短时长过滤

- **纯 CPU 软件渲染**
  - 输出 RGBA8888 像素缓冲区，无需 GPU 上下文
  - crossbeam 无锁队列，Dart 异步批量推送
  - SIMD 向量化加速坐标与像素混合

- **Emoji 贴图管理**
  - Flutter RGBA 位图注入
  - 本地文件加载
  - 网络 URL 远程下载 + LRU 缓存

- **四大文字特效**
  - 描边特效（Stroke）
  - 立体阴影特效（Shadow）
  - 霓虹发光特效（Neon）
  - 七彩渐变特效（Rainbow）

- **跨平台支持（Dart Hooks 自动编译）**
  - Android (arm64-v8a / armeabi-v7a / x86_64 / x86)
  - iOS (arm64 / x86_64 模拟器)
  - Windows (x86_64 / ARM64)
  - macOS (Apple Silicon / Intel)
  - Linux (x86_64 / aarch64)

---

## Dart Hooks 系统（Flutter 3.38+）

### 什么是 Dart Hooks？

Dart Hooks（原名 native-assets）是 Flutter 3.38 正式支持的新构建系统，用于自动编译和打包原生代码。它通过 `hook/build.dart` 脚本替代了传统的平台特定构建文件：

| 传统方式 | Dart Hooks |
|---------|-----------|
| Android: CMakeLists.txt + build.gradle | ✗ 不需要 |
| iOS: .podspec | ✗ 不需要 |
| Windows/macOS/Linux: CMakeLists.txt | ✗ 不需要 |
| 手动交叉编译 | 自动处理 |
| 手动 dlopen 加载动态库 | @Native 自动解析 |

### 核心优势

1. **零平台特定代码**：一个 `build.dart` 脚本处理所有平台
2. **自动工具链管理**：`native_doctor` 自动安装 Rust/NDK
3. **声明式 FFI**：使用 `@Native` 注解，无需手动 `DynamicLibrary.open`
4. **跨平台一致性**：同一份 Dart 代码支持所有平台

### 项目 Hooks 结构

```
flutter_plugin/
├── hook/
│   └── build.dart           # Dart Hooks 构建脚本
├── native_manifest.yaml     # 工具链版本配置
├── pubspec.yaml             # Hooks 依赖声明
└── lib/src/
    └── ffi_bindings.dart    # @Native 注解声明
```

---

## 与 DanmakuFlameMaster 功能对照表

| 功能 | DanmakuFlameMaster | rs_ffi_barrage |
|------|:---:|:---:|
| 滚动弹幕 | ✅ | ✅ |
| 顶部固定弹幕 | ✅ | ✅ |
| 底部固定弹幕 | ✅ | ✅ |
| 逆向滚动弹幕 | ✅ | ✅ |
| 轨道碰撞检测 | ✅ | ✅ |
| 弹幕生命周期管理 | ✅ | ✅ |
| 时间轴控制（倍速/seek） | ✅ | ✅ |
| 弹幕过滤 | ✅ | ✅ |
| CPU 渲染 | ✅ | ✅ |
| 内存池复用 | ❌ | ✅ |
| Emoji 贴图注册 | ❌ | ✅ |
| 文字描边特效 | ❌ | ✅ |
| 立体阴影特效 | ❌ | ✅ |
| 霓虹发光特效 | ❌ | ✅ |
| 七彩渐变特效 | ❌ | ✅ |
| Rust FFI 跨平台 | ❌ | ✅ |
| SIMD 加速 | ❌ | ✅ |
| Dart Hooks 自动编译 | ❌ | ✅ |

---

## Rust 性能优化亮点

rs_ffi_barrage 充分利用 Rust 语言特性进行极致性能优化：

| 优化项 | 说明 |
|--------|------|
| **无锁并发队列** | 使用 `crossbeam` channel 实现生产者-消费者无锁通信，Dart 侧批量异步拉取渲染帧 |
| **parking_lot 轻量读写锁** | 替代标准库 `Mutex/RwLock`，在竞争较低场景下减少 5~10 倍锁开销 |
| **bytemuck 零拷贝** | Rust 结构体与 C 结构体内存布局完全对齐，FFI 边界零拷贝传递 |
| **SIMD 向量化加速** | 坐标变换、像素 Alpha 混合、颜色空间转换使用 `std::simd` 或 `wide` crate 并行处理 |
| **LTO 全优化** | Release 配置开启 `lto = "fat"`，跨 crate 内联优化 |
| **strip 瘦身** | 剥离调试符号，减小 .so/.dylib/.dll 文件体积（可减少 50% 以上） |
| **panic=abort** | FFI 边界设置 `panic = "abort"`，避免 unwinding 跨语言边界导致未定义行为 |
| **LRU 内存缓存** | 贴图数据使用 `lru` crate 缓存，热数据常驻内存 |
| **延迟解码** | 网络/本地图片仅缓存压缩数据，渲染时按需解码，降低 IO 与内存开销 |

---

## 项目架构

```
rs_ffi_barrage/
├── rust/                          # Rust 底层 cdylib 内核
│   ├── Cargo.toml                 # Rust 项目配置
│   ├── cbindgen.toml              # cbindgen 配置（自动生成 C 头文件）
│   ├── src/
│   │   ├── lib.rs                 # FFI C 接口层（#[no_mangle] extern "C" 导出）
│   │   ├── core.rs                # Core 全局调度器（引擎生命周期管理）
│   │   ├── track.rs               # 轨道管理器（4种轨道 + 碰撞检测）
│   │   ├── barrage.rs             # 弹幕对象 & 内存池（对象复用）
│   │   ├── emoji.rs               # Emoji 图文资源管理器（注册/查询/缓存）
│   │   ├── text_effect.rs         # 文字特效渲染模块（描边/阴影/霓虹/七彩）
│   │   ├── filter.rs              # 过滤引擎（黑名单/类型/时长过滤）
│   │   ├── timeline.rs            # 时间轴控制器（倍速/seek/暂停/恢复）
│   │   ├── renderer.rs            # CPU RGBA 渲染管线（图文混排）
│   │   ├── network.rs             # 网络 HTTP 客户端（远程贴图下载）
│   │   └── utils.rs               # 底层工具库（SIMD 辅助/颜色转换）
│   ├── tests/                     # 集成测试
│   │   └── integration_tests.rs
│   └── benches/                   # 性能基准测试（criterion）
│       └── barrage_bench.rs
├── flutter_plugin/                # Flutter 插件工程
│   ├── hook/                      # Dart Hooks 构建系统
│   │   └── build.dart             # 构建脚本（自动编译 Rust）
│   ├── native_manifest.yaml       # 工具链版本配置
│   ├── pubspec.yaml               # Hooks 依赖声明
│   ├── lib/
│   │   └── src/
│   │       ├── ffi_bindings.dart       # @Native 注解 FFI 绑定
│   │       ├── barrage_engine.dart     # 弹幕引擎 Dart 封装
│   │       ├── barrage_view.dart       # Flutter Widget 渲染组件
│   │       ├── emoji_manager.dart      # Emoji 管理器（三类注册接口）
│   │       └── text_effect_config.dart # 文字特效配置模型
│   └── example/                   # Flutter 演示工程
├── c_demo/                        # C 语言 FFI Demo
│   ├── main.c                     # 核心接口验证程序
│   ├── CMakeLists.txt             # CMake 构建配置
│   └── build.sh                   # 一键构建脚本
├── scripts/                       # 构建与推送脚本
│   └── push_to_github.sh          # Git 一键推送脚本
├── .gitignore
└── README.md                      # 本文件
```

---

## 环境准备

### 1. 安装 Rust 工具链

```bash
# macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
# 下载并运行 https://rustup.rs/
```

### 2. 安装必要的 Rust targets

```bash
# Android
rustup target add aarch64-linux-android
rustup target add arm-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android

# iOS
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim

# macOS
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin

# Windows
rustup target add x86_64-pc-windows-msvc
rustup target add aarch64-pc-windows-msvc

# Linux
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-gnu
```

### 3. 使用 native_doctor 自动安装（推荐）

```bash
# 安装 native_doctor
dart pub global activate native_doctor

# 在项目目录运行，自动安装所有依赖
cd flutter_plugin
dart pub global run native_doctor
```

`native_doctor` 会读取 `native_manifest.yaml` 配置，自动安装：
- 正确版本的 Android NDK
- Rust 工具链和所有必要的 targets

---

## Flutter 集成步骤

### 1. 在 pubspec.yaml 中添加依赖

```yaml
dependencies:
  flutter:
    sdk: flutter
  rs_ffi_barrage:
    git:
      url: https://github.com/HDYOU/rs_ffi_barrage
      path: flutter_plugin
```

### 2. 运行获取依赖

```bash
flutter pub get
```

首次运行时，Dart Hooks 会自动编译 Rust 代码。

### 3. 初始化引擎

```dart
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

final BarrageEngine engine = BarrageEngine();

void initBarrage() {
  // Dart Hooks 版本无需 await initialize()
  // 引擎在构造时自动初始化
  print('弹幕引擎已初始化');
}
```

### 4. 使用 BarrageView 渲染

```dart
import 'package:flutter/material.dart';
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

class MyVideoPlayer extends StatefulWidget {
  @override
  State<MyVideoPlayer> createState() => _MyVideoPlayerState();
}

class _MyVideoPlayerState extends State<MyVideoPlayer> {
  final BarrageEngine _engine = BarrageEngine();

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        // 视频播放器...
        Positioned.fill(
          child: BarrageView(
            engine: _engine,
            width: 1080,
            height: 720,
          ),
        ),
      ],
    );
  }

  @override
  void dispose() {
    _engine.dispose();
    super.dispose();
  }
}
```

### 5. 发送弹幕

```dart
// 发送一条简单的正向滚动弹幕
_engine.sendBarrage(
  text: 'Hello Flutter!',
  trackType: 0,              // 0=正向滚动, 1=顶部, 2=底部, 3=逆向
  color: 0xFFFFFFFF,         // 白色
  fontSize: 24.0,
  alpha: 0.9,
  durationMs: 5000,
);

// 发送一条带特效的弹幕
_engine.sendBarrage(
  text: '特效弹幕!',
  trackType: 0,
  color: 0xFFFF00FF,
  fontSize: 28.0,
  alpha: 1.0,
  durationMs: 6000,
  effectConfig: TextEffectConfig.outlineOnly(),
);
```

---

## getEmojiBitmapFromFlutter 接口使用案例

```dart
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

final engine = BarrageEngine();

// 查询贴图（通过表情名称获取 RGBA 位图数据）
// 注意：入参仅为纯文本标识，不包含中括号
Uint8List? bitmap = engine.emojiManager.getEmojiBitmapFromFlutter("666");

if (bitmap != null) {
  // 获取到贴图数据，可以二次渲染
  // bitmap 为 RGBA8888 原始像素数据
  final image = await rgbaBytesToImage(bitmap, width, height);
  // 在自定义 Widget 中展示
} else {
  print("未找到贴图 [666]");
}
```

---

## 三类 Emoji 注册 Dart 代码示例

### 方式一：Flutter RGBA 位图注入

```dart
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/painting.dart';
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

Future<void> registerFromBitmap(EmojiManager manager) async {
  // 从 Flutter 资源或网络加载图片
  final ByteData data = await rootBundle.load('assets/emojis/666.png');
  final ui.Image image = await decodeImageFromList(data.buffer.asUint8List());

  // 转换为 RGBA 字节数据
  final byteData = await image.toByteData(format: ui.ImageByteFormat.rawRgba);
  final Uint8List rgbaData = byteData!.buffer.asUint8List();

  // 注册到引擎
  manager.registerEmojiFromFlutterBitmap(
    emojiId: '666',
    rgbaData: rgbaData,
    width: image.width,
    height: image.height,
  );
  print('贴图 [666] 注册成功 (${image.width}x${image.height})');
}
```

### 方式二：本地文件

```dart
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

Future<void> registerFromFile(EmojiManager manager) async {
  manager.registerEmojiFromLocalPath(
    emojiId: 'laugh',
    filePath: '/path/to/emojis/laugh.png',
  );
  print('贴图 [laugh] 已从文件注册');
}
```

### 方式三：网络 URL

```dart
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

Future<void> registerFromUrl(EmojiManager manager) async {
  manager.registerEmojiFromUrl(
    emojiId: 'cool',
    url: 'https://example.com/emojis/cool.png',
  );
  print('贴图 [cool] 已从网络 URL 注册');
}
```

---

## 四大文字特效使用代码

### 描边特效 (Stroke)

```dart
import 'package:rs_ffi_barrage/rs_ffi_barrage.dart';

final config = TextEffectConfig.outlineOnly();
// 或自定义：
// final config = TextEffectConfig(
//   outlineEnabled: true,
//   outlineWidth: 2.0,
//   outlineColorRgba: 0xFF0000FF,
//   outlineSoftEdge: true,
// );

engine.sendBarrage(
  text: '描边特效',
  effectConfig: config,
);
```

### 立体阴影特效 (Shadow)

```dart
final config = TextEffectConfig.embossShadow();

engine.sendBarrage(
  text: '阴影特效',
  effectConfig: config,
);
```

### 霓虹发光特效 (Neon)

```dart
final config = TextEffectConfig.neonGlow();

engine.sendBarrage(
  text: '霓虹特效',
  effectConfig: config,
);
```

### 七彩渐变特效 (Rainbow)

```dart
final config = TextEffectConfig.rainbowGradient();

engine.sendBarrage(
  text: '七彩渐变',
  effectConfig: config,
);
```

### 全特效叠加

```dart
final config = TextEffectConfig.allEffects();

engine.sendBarrage(
  text: '全特效弹幕',
  effectConfig: config,
);
```

---

## 构建与测试命令

```bash
# ── Rust 单元测试 ──────────────────────────────────────────────────────────
cd rust && cargo test

# ── Rust 性能基准测试（criterion） ──────────────────────────────────────────
cd rust && cargo bench

# ── Rust Release 编译 ──────────────────────────────────────────────────────
cd rust && cargo build --release

# ── Flutter 构建（Dart Hooks 自动编译 Rust） ────────────────────────────────
cd flutter_plugin/example

# Android
flutter build apk

# iOS
flutter build ios

# Windows
flutter build windows

# macOS
flutter build macos

# Linux
flutter build linux

# ── 运行 native_doctor（自动安装工具链） ────────────────────────────────────
dart pub global activate native_doctor
dart pub global run native_doctor

# ── C Demo 编译与运行 ──────────────────────────────────────────────────────
cd c_demo && bash build.sh
```

---

## hook/build.dart 工作原理

```dart
// flutter_plugin/hook/build.dart
import 'package:hooks/hooks.dart';
import 'package:code_assets/code_assets.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

void main(List<String> args) async {
  await build(args, (BuildInput input, BuildOutputBuilder output) async {
    final rustBuilder = RustBuilder(
      package: 'rs_ffi_barrage',
      cratePath: 'rust',
      buildConfig: input,
    );
    await rustBuilder.run(output: output);
  });
}
```

构建流程：
1. Flutter 构建时自动调用 `hook/build.dart`
2. `RustBuilder` 检测目标平台和架构
3. 选择正确的 Rust target（如 `aarch64-linux-android`）
4. 执行 `cargo build --release --target <target>`
5. 输出 `CodeAsset` 到 `output.assets.code`
6. Flutter 打包动态库到应用程序

---

## ffi_bindings.dart @Native 注解

```dart
// 使用 @ffi.DefaultAsset 指定资产位置
@ffi.DefaultAsset('package:rs_ffi_barrage/rs_ffi_barrage')
library rs_ffi_barrage_bindings;

import 'dart:ffi' as ffi;

// @Native 注解声明 FFI 函数，无需手动 DynamicLibrary.open
@ffi.Native<ffi.Pointer<ffi.Void> Function(ffi.Int32, ffi.Int32)>()
external ffi.Pointer<ffi.Void> rs_barrage_create(int max_tracks, int pool_capacity);

@ffi.Native<ffi.Int32 Function(ffi.Pointer<ffi.Void>, ffi.Int32)>()
external int rs_barrage_update(ffi.Pointer<ffi.Void> engine, int delta_ms);
```

关键点：
- `@ffi.DefaultAsset` 指定动态库位置（格式：`package:<包名>/<crate名>`）
- `@Native` 注解声明函数签名
- `external` 关键字标记为外部函数
- 构建系统自动解析符号，无需手动 `lookupFunction`

---

## 平台权限配置

### Android

在 `AndroidManifest.xml` 中添加：

```xml
<manifest>
    <!-- 网络贴图下载需要 -->
    <uses-permission android:name="android.permission.INTERNET" />
</manifest>
```

### iOS

在 `Info.plist` 中配置 ATS 白名单（如需要加载 HTTP 图片）：

```xml
<key>NSAppTransportSecurity</key>
<dict>
    <key>NSAllowsArbitraryLoads</key>
    <false/>
    <key>NSExceptionDomains</key>
    <dict>
        <key>example.com</key>
        <dict>
            <key>NSExceptionAllowsInsecureHTTPLoads</key>
            <true/>
        </dict>
    </dict>
</dict>
```

---

## 参考资料

- [Flutter 官方文档：Bind to native code using FFI](https://docs.flutter.dev/platform-integration/bind-native-code)
- [Dart Hooks 文档](https://dart.dev/tools/hooks)
- [native_toolchain_rust 包](https://pub.dev/packages/native_toolchain_rust)
- [hooks 包](https://pub.dev/packages/hooks)
- [code_assets 包](https://pub.dev/packages/code_assets)

---

## License

MIT

Copyright (c) 2024 HDYOU