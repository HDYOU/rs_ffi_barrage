/// build.rs — cbindgen 自动生成 C 头文件
///
/// 在构建时调用 cbindgen 读取 crate 根目录的 cbindgen.toml 配置，
/// 扫描所有 #[no_mangle] extern "C" 函数，生成 rs_barrage.h 头文件。
/// 该头文件供 Flutter/Dart 通过 FFI (dart:ffi) 直接调用。

fn main() {
    // 告知 cargo，当 cbindgen.toml 或 src/lib.rs 变化时重新运行 build.rs
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");

    // 尝试从 crate 根目录解析配置并生成头文件
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let config = cbindgen::Config::from_file("cbindgen.toml")
        .unwrap_or_else(|e| {
            // 若配置文件读取失败，使用默认配置并记录警告
            eprintln!("[build.rs] 警告: 读取 cbindgen.toml 失败 ({}), 使用默认配置", e);
            cbindgen::Config::default()
        });

    match cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file("rs_barrage.h");
            println!("[build.rs] 成功生成 C 头文件: rs_barrage.h");
        }
        Err(e) => {
            // 头文件生成失败不阻断编译，仅输出警告
            eprintln!("[build.rs] 警告: cbindgen 头文件生成失败 ({})", e);
            eprintln!("[build.rs] 编译将继续，但您需要手动生成头文件");
        }
    }
}