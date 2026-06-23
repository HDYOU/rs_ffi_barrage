// =============================================================================
// rs_ffi_barrage — Dart Hooks 构建脚本
//
// 使用 Dart Hooks 系统（Flutter 3.38+ 正式支持）自动编译 Rust 原生代码。
// 此脚本在 Flutter/Dart 构建过程中自动运行，无需手动编写平台特定的构建文件。
//
// 依赖：
// - native_toolchain_rust: Rust 编译工具链
// - hooks: Dart Hooks 核心库
// - code_assets: 代码资产定义
//
// 参考：
// https://docs.flutter.dev/platform-integration/bind-native-code
// https://pub.dev/packages/native_toolchain_rust
// =============================================================================

import 'dart:io';

import 'package:hooks/hooks.dart';
import 'package:code_assets/code_assets.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

/// 构建钩子入口点
///
/// Flutter SDK 在构建过程中自动调用此脚本。
/// 脚本使用 RustBuilder 编译 rust/ 目录下的 Rust cdylib 代码，
/// 并将生成的动态库自动打包到应用程序中。
void main(List<String> args) async {
  try {
    await build(args, (BuildInput input, BuildOutputBuilder output) async {
      // ── 配置 Rust 构建器 ─────────────────────────────────────────────────────
      // RustBuilder 会自动：
      // 1. 检测目标平台和架构
      // 2. 选择正确的 Rust target (如 aarch64-linux-android, x86_64-apple-darwin)
      // 3. 调用 cargo 编译 cdylib
      // 4. 将生成的动态库添加到输出资产中
      
      final rustBuilder = RustBuilder(
        // 包名：与 pubspec.yaml 中的 name 一致
        package: 'rs_ffi_barrage',
        
        // Rust crate 路径：相对于包根目录
        // Rust 代码位于 rust/ 目录，Cargo.toml 在 rust/Cargo.toml
        cratePath: 'rust',
        
        // 构建配置输入
        buildConfig: input,
      );
      
      // ── 执行 Rust 编译 ───────────────────────────────────────────────────────
      // RustBuilder.run() 会：
      // - 检查 rustup 是否安装
      // - 确保正确的 Rust target 已安装
      // - 执行 cargo build --release --target <target>
      // - 输出 CodeAsset 到 output.assets.code
      await rustBuilder.run(output: output);
      
      // ── 构建日志 ─────────────────────────────────────────────────────────────
      // 打印构建信息用于调试
      final targetOS = input.targetOS;
      final targetArchitecture = input.targetArchitecture;
      print('[rs_ffi_barrage] 构建完成:');
      print('  - 目标平台: ${targetOS.name}');
      print('  - 目标架构: ${targetArchitecture.name}');
      print('  - 输出资产数: ${output.assets.code.length}');
      
      // ── 资产验证 ─────────────────────────────────────────────────────────────
      // 确保至少有一个代码资产被生成
      if (output.assets.code.isEmpty) {
        throw StateError(
          'Rust 构建未产生任何代码资产。请检查 rust/Cargo.toml 配置:\n'
          '  - crate-type 必须包含 "cdylib"\n'
          '  - name 必须为 "rs_ffi_barrage"'
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
    // 提供清晰的错误信息，指导用户解决问题
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
    print('     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh');
    print('');
    print('  2. 安装必要的 Rust targets:');
    print('     rustup target add aarch64-linux-android');
    print('     rustup target add arm-linux-androideabi');
    print('     rustup target add x86_64-linux-android');
    print('     rustup target add i686-linux-android');
    print('     rustup target add aarch64-apple-ios');
    print('     rustup target add x86_64-apple-ios');
    print('     rustup target add aarch64-apple-ios-sim');
    print('     rustup target add aarch64-apple-darwin');
    print('     rustup target add x86_64-apple-darwin');
    print('     rustup target add x86_64-pc-windows-msvc');
    print('     rustup target add aarch64-pc-windows-msvc');
    print('');
    print('  3. 或运行 native_doctor 自动安装:');
    print('     dart pub global activate native_doctor');
    print('     dart pub global run native_doctor');
    print('');
    exit(1);
  }
}