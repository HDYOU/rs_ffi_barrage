# =============================================================================
# rs_ffi_barrage — C Demo CMake 构建配置
#
# 编译 C 语言 FFI Demo 程序，链接 librs_ffi_barrage 动态库。
# 用于脱离 Flutter 环境直接验证 Rust FFI 核心接口。
# =============================================================================

cmake_minimum_required(VERSION 3.10)
project(rs_ffi_barrage_demo VERSION 0.1.0 LANGUAGES C)

# ── C 标准 ────────────────────────────────────────────────────────────────────
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# ── Rust 输出路径 ─────────────────────────────────────────────────────────────
# 指向 Rust cdylib 编译输出目录
set(RUST_TARGET_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../rust/target/release")

# ── 可执行文件 ────────────────────────────────────────────────────────────────
add_executable(rs_ffi_barrage_demo
    main.c
)

# ── 头文件路径 ────────────────────────────────────────────────────────────────
# cbindgen 生成的 rs_barrage.h 在此目录
target_include_directories(rs_ffi_barrage_demo PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../rust
)

# ── 链接 Rust 动态库 ──────────────────────────────────────────────────────────
# Linux / macOS 链接方式
if(APPLE)
    target_link_libraries(rs_ffi_barrage_demo PRIVATE
        ${RUST_TARGET_DIR}/librs_ffi_barrage.dylib
    )
elseif(UNIX)
    target_link_libraries(rs_ffi_barrage_demo PRIVATE
        ${RUST_TARGET_DIR}/librs_ffi_barrage.so
    )
elseif(WIN32)
    target_link_libraries(rs_ffi_barrage_demo PRIVATE
        ${RUST_TARGET_DIR}/rs_ffi_barrage.dll
    )
endif()

# ── 运行时库路径设置 ──────────────────────────────────────────────────────────
# 使可执行文件能够找到动态库
if(APPLE)
    set_target_properties(rs_ffi_barrage_demo PROPERTIES
        BUILD_RPATH "${RUST_TARGET_DIR}"
        INSTALL_RPATH "${RUST_TARGET_DIR}"
    )
elseif(UNIX)
    set_target_properties(rs_ffi_barrage_demo PROPERTIES
        BUILD_RPATH "${RUST_TARGET_DIR}"
        INSTALL_RPATH "${RUST_TARGET_DIR}"
    )
endif()

message(STATUS "C Demo 构建目标: rs_ffi_barrage_demo")
message(STATUS "Rust 库路径: ${RUST_TARGET_DIR}")