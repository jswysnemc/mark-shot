#!/usr/bin/env bash
# 【Windows构建】【OCR依赖】准备旧 ABI 的并存 DLL，不替换 MSYS2 当前头文件、导入库或运行时
# 参数 1：兼容运行时的独立前缀；结果放入该目录的 bin 子目录
set -euo pipefail

prefix="${1:?Expected a private runtime prefix}"
package="mingw-w64-ucrt-x86_64-abseil-cpp-20260526.0-1-any.pkg.tar.zst"
checksum="89803dbb88b7e1a18169fd2ecefa2b85de3d99fcd5d6899b737e4451532b691f"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

# 1. 【Windows构建】【可信下载】MSYS2 ONNX 依赖仍引用 2605 ABI，不能把 2608 DLL 改名替代
curl --fail --location --retry 3 --connect-timeout 20 --max-time 180 \
    "https://repo.msys2.org/mingw/ucrt64/$package" -o "$temporary/$package"
printf '%s  %s\n' "$checksum" "$temporary/$package" | sha256sum --check --status

# 2. 【Windows构建】【隔离提取】仅复制带版本号的 DLL，当前工具链及其他 ABI 均保持不变
(cd "$temporary" && cmake -E tar xf "$package")
test -f "$temporary/ucrt64/bin/libabsl_hash-2605.0.0.dll"
mkdir -p "$prefix/bin"
cp "$temporary"/ucrt64/bin/libabsl_*-2605.0.0.dll "$prefix/bin/"
printf 'Prepared Abseil 2605 compatibility runtime in %s\n' "$prefix/bin"
