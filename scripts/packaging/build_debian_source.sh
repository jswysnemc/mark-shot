#!/usr/bin/env bash
# 【原生打包】【源码构建】先验证全部系统依赖和实际 Rust 版本，再构建源码或二进制
set -euo pipefail
source_root="$(realpath "${1:?source root required}")"
kind="${2:?source or binary required}"
python3 "$(dirname "$0")/debian_build_dependencies.py" --root "$source_root"
cd "$source_root"
# 【原生打包】【工具链核验】Rustup 工具链不登记在 dpkg，前置校验替代重复的软件包数据库检查
case "$kind" in
  source) dpkg-buildpackage -S -sa -us -uc --no-check-builddeps ;;
  binary) dpkg-buildpackage -b -us -uc --no-check-builddeps ;;
  *) exit 1 ;;
esac
