#!/bin/bash
# =============================================================================
# rs_ffi_barrage — C Demo 一键构建脚本
#
# 先编译 Rust cdylib，再编译 C Demo 并运行。
# 支持 Linux / macOS / Windows (MSYS2/MinGW)。
# =============================================================================

set -e

echo "============================================"
echo "  rs_ffi_barrage — C Demo 一键构建"
echo "============================================"

# 获取脚本所在目录的上级目录（项目根目录）
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "[1/4] 编译 Rust cdylib (Release)..."
cd "${PROJECT_ROOT}/rust"
cargo build --release
echo "  Rust 编译完成"

# ── 检测平台 ──────────────────────────────────────────────────────────────────
OS_TYPE="$(uname -s)"
case "${OS_TYPE}" in
    Linux*)   PLATFORM="linux"  ;;
    Darwin*)  PLATFORM="macos"  ;;
    MINGW*|MSYS*) PLATFORM="windows" ;;
    *)        echo "[错误] 不支持的操作系统: ${OS_TYPE}"; exit 1 ;;
esac
echo "  检测到平台: ${PLATFORM}"

echo "[2/4] 创建 C Demo 构建目录..."
BUILD_DIR="${SCRIPT_DIR}/build"
mkdir -p "${BUILD_DIR}"

echo "[3/4] 运行 CMake 构建..."
cd "${BUILD_DIR}"
cmake "${SCRIPT_DIR}" -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release

echo "[4/4] 运行 C Demo..."
echo ""

if [ "${PLATFORM}" = "windows" ]; then
    # Windows 需要确保 DLL 在 PATH 中
    RUST_DLL_DIR="${PROJECT_ROOT}/rust/target/release"
    export PATH="${RUST_DLL_DIR}:${PATH}"
    ./Release/rs_ffi_barrage_demo.exe
else
    ./rs_ffi_barrage_demo
fi

echo ""
echo "============================================"
echo "  C Demo 执行完毕！"
echo "============================================"