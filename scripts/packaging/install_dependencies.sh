#!/usr/bin/env bash
# 【原生构建】【系统依赖】只安装原生 Rust、媒体库和发行包工具所需的系统依赖
set -euo pipefail
packaging="${1:-build}"
privilege=()
if [[ "$EUID" != 0 ]]; then privilege=(sudo); fi
# shellcheck source=/dev/null
source /etc/os-release
case "$ID" in
  ubuntu|debian)
    export DEBIAN_FRONTEND=noninteractive
    "${privilege[@]}" apt-get update
    packages=(build-essential clang libclang-dev pkg-config ca-certificates curl git python3
      libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libpipewire-0.3-dev
      libssl-dev libfontconfig-dev fontconfig fonts-noto-core fonts-noto-cjk nasm
      libva-dev libdrm-dev libgbm-dev libpulse-dev xz-utils zstd)
    if [[ "$packaging" == packaging ]]; then
      packages+=(dpkg-dev desktop-file-utils file patchelf appstream fakeroot debhelper
        devscripts equivs lintian gh)
    fi
    "${privilege[@]}" apt-get install -y --no-install-recommends "${packages[@]}"
    ;;
  fedora)
    packages=(gcc gcc-c++ make clang clang-devel pkgconf-pkg-config ca-certificates curl git python3
      wayland-devel libxkbcommon-devel libxkbcommon-x11-devel libxcb-devel pipewire-devel openssl-devel fontconfig-devel
      google-noto-sans-fonts google-noto-sans-cjk-fonts nasm libva-devel libdrm-devel mesa-libgbm-devel
      pulseaudio-libs-devel xz zstd)
    if [[ "$packaging" == packaging ]]; then
      packages+=(rpm-build desktop-file-utils file patchelf appstream)
    fi
    "${privilege[@]}" dnf -y --setopt=install_weak_deps=False install "${packages[@]}"
    ;;
  arch)
    packages=(base-devel clang pkgconf ca-certificates curl git python wayland libxkbcommon
      libxkbcommon-x11 libxcb pipewire openssl fontconfig noto-fonts noto-fonts-cjk nasm
      libva libdrm mesa libpulse xz zstd)
    if [[ "$packaging" == packaging ]]; then
      packages+=(desktop-file-utils file patchelf appstream)
    fi
    "${privilege[@]}" pacman -Syu --noconfirm --needed "${packages[@]}"
    ;;
  *) printf '【原生构建】【系统依赖】不支持的发行版: %s\n' "$ID" >&2; exit 1 ;;
esac
