// =============================================================================
// rs_ffi_barrage — Dart Hooks 构建脚本
//
// 使用 Dart Hooks 系统（Flutter 3.41+ 正式支持）自动编译 Rust 原生代码。
// 此脚本在 Flutter/Dart 构建过程中自动运行，无需手动编写平台特定的构建文件。
//
// 依赖：
// - native_toolchain_rust ^1.0.4: Rust 编译工具链
// - hooks: Dart Hooks 核心库
//
// 参考：
// https://docs.flutter.dev/platform-integration/bind-native-code
// https://github.com/GregoryConrad/native_toolchain_rust
// =============================================================================

import 'dart:io';

import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

/// 构建钩子入口点
///
/// Flutter SDK 在构建过程中自动调用此脚本。
/// 脚本使用 RustBuilder 编译 rust/ 目录下的 Rust cdylib 代码，
/// 并将生成的动态库自动打包到应用程序中。
void main(List<String> args) async {
  try {
    await build(args, (input, output) async {
      // ── 配置 Rust 构建器 ─────────────────────────────────────────────────────
      // RustBuilder 会自动：
      // 1. 检测目标平台和架构
      // 2. 选择正确的 Rust target (如 aarch64-linux-android, x86_64-apple-darwin)
      // 3. 调用 cargo 编译 cdylib
      // 4. 将生成的动态库添加到输出资产中
      //
      // native_toolchain_rust ^1.0.4 新 API：
      // - assetName: 指定 FFI 绑定文件路径
      // - 默认查找 rust/ 或 native/ 目录下的 Cargo.toml
      
      await RustBuilder(
        // 资产名称：指向 FFI 绑定文件
        // 格式：'src/<文件名>.dart'
        assetName: 'src/ffi_bindings.dart',
        
        // 可选配置：
        // - cratePath: 指定 Rust crate 路径（默认查找 rust/ 或 native/）
        // - cargoFeatures: 启用 Cargo features
        // - rustToolchainVersion: 指定 Rust 版本
      ).run(input: input, output: output);
      
      // ── 构建日志 ─────────────────────────────────────────────────────────────
      print('[rs_ffi_barrage] 构建完成:');
      print('  - 目标平台: ${input.targetOS.name}');
      print('  - 目标架构: ${input.targetArchitecture.name}');
      print('  - 输出资产数: ${output.assets.code.length}');
      
      // ── 资产验证 ─────────────────────────────────────────────────────────────
      if (output.assets.code.isEmpty) {
        throw StateError(
          'Rust 构建未产生任何代码资产。请检查 rust/Cargo.toml 配置:\n'
          '  - crate-type 必须包含 "cdylib" 和 "staticlib"\n'
          '  - [lib] name 必须配置正确'
        );
      }
      
      // 打印每个资产的详细信息
      for (final asset in output.assets.code) {
        print('  - 资产: ${asset.name}');
        print('    链接模式: ${asset.linkMode}');
        if (asset.file != null) {
          print('    文件: ${asset.file}');
        }
      }
    });
  } catch (e, stackTrace) {
    // ── 错误处理 ───────────────────────────────────────────────────────────────
    print('============================================');
    print('  [rs_ffi_barrage] 构建失败');
    print('============================================');
    print('');
    print('错误: $e');
    print('');
    print('堆栈跟踪:');
    print(stackTrace);
    print('');
    print('可能的解决方案:');
    print('  1. 确保已安装 Rust 工具链:');
    print('     curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh');
    print('');
    print('  2. 确保 rust-toolchain.toml 配置正确:');
    print('     [toolchain]');
    print('     channel = "1.90.0"');
    print('     targets = [');
    print('       "aarch64-linux-android",');
    print('       "x86_64-linux-android",');
    print('       "aarch64-apple-ios",');
    print('       "aarch64-apple-ios-sim",');
    print('       "x86_64-apple-ios",');
    print('       "aarch64-apple-darwin",');
    print('       "x86_64-apple-darwin",');
    print('       "x86_64-pc-windows-msvc",');
    print('       "aarch64-pc-windows-msvc",');
    print('       "x86_64-unknown-linux-gnu",');
    print('       "aarch64-unknown-linux-gnu",');
    print('     ]');
    print('');
    print('  3. 确保 Cargo.toml 配置正确:');
    print('     [lib]');
    print('     crate-type = ["staticlib", "cdylib"]');
    print('');
    exit(1);
  }
}