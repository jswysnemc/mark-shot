#!/usr/bin/env bash
# Build a .deb from an already-configured CMake build tree.
# Intended for CI (release-deb.yml) and local Debian/Ubuntu containers.
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: build-deb-package.sh --build-dir DIR --version VER --arch ARCH
                           --target NAME [--asset-suffix SUFFIX] [--compat-check]

Environment:
  PWD must be the repository root (so LICENSE/README are available).

Notes:
  - Shared-library Depends are generated from the main binary and all
    installed mark-shot *.so modules (layer-shell + provider plugins).
  - --compat-check rejects dependencies that are too new for Debian 12 /
    Deepin baselines (t64, Qt 6.9+, LayerShellQt, etc.).
EOF
}

BUILD_DIR=""
VERSION=""
DEB_ARCH=""
TARGET=""
ASSET_SUFFIX=""
COMPAT_CHECK=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --build-dir) BUILD_DIR="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        --arch) DEB_ARCH="$2"; shift 2 ;;
        --target) TARGET="$2"; shift 2 ;;
        --asset-suffix) ASSET_SUFFIX="$2"; shift 2 ;;
        --compat-check) COMPAT_CHECK=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 1 ;;
    esac
done

if [[ -z "$BUILD_DIR" || -z "$VERSION" || -z "$DEB_ARCH" || -z "$TARGET" ]]; then
    usage >&2
    exit 1
fi

HOST_ARCH="$(dpkg --print-architecture)"
if [[ "$HOST_ARCH" != "$DEB_ARCH" ]]; then
    echo "Runner architecture $HOST_ARCH does not match package architecture $DEB_ARCH" >&2
    exit 1
fi

PACKAGE_DIR="$PWD/deb-root"
INSTALL_DIR="$PACKAGE_DIR/usr"
CONTROL_DIR="$PACKAGE_DIR/DEBIAN"
ASSET="mark-shot_${VERSION}_${DEB_ARCH}${ASSET_SUFFIX}.deb"

rm -rf "$PACKAGE_DIR"
cmake --install "$BUILD_DIR" --prefix "$INSTALL_DIR"
install -Dm644 LICENSE "$PACKAGE_DIR/usr/share/doc/mark-shot/copyright"
install -Dm644 README.md "$PACKAGE_DIR/usr/share/doc/mark-shot/README.md"
install -Dm644 README.zh-CN.md "$PACKAGE_DIR/usr/share/doc/mark-shot/README.zh-CN.md"
install -d "$CONTROL_DIR"

mkdir -p debian
cat > debian/control <<'EOF'
Source: mark-shot
Section: graphics
Priority: optional
Maintainer: jswysnemc <snemc@qq.com>
Rules-Requires-Root: no
Standards-Version: 4.7.0

Package: mark-shot
Architecture: any
Depends: ${shlibs:Depends}
Description: Qt 6 screenshot selection and annotation tool
 Mark Shot captures screenshots and annotates image regions.
EOF

# 1. 收集主程序与全部 ELF 插件/模块，避免 OCR/layer-shell 的 so 依赖漏出
SHLIB_ARGS=()
if [[ -x "$PACKAGE_DIR/usr/bin/mark-shot" ]]; then
    SHLIB_ARGS+=(-e"$PACKAGE_DIR/usr/bin/mark-shot")
fi
while IFS= read -r -d '' so_path; do
    if file -b "$so_path" | grep -q 'ELF'; then
        SHLIB_ARGS+=(-e"$so_path")
    fi
done < <(find "$PACKAGE_DIR/usr" -type f -name '*.so' -print0 2>/dev/null)

if [[ ${#SHLIB_ARGS[@]} -eq 0 ]]; then
    echo "No ELF binaries found under $PACKAGE_DIR/usr for dpkg-shlibdeps" >&2
    exit 1
fi

# 2. 合并所有 so 的共享库依赖
SHLIB_DEPS="$(dpkg-shlibdeps -O "${SHLIB_ARGS[@]}" | sed 's/^shlibs:Depends=//')"
MANUAL_DEPS="python3"
RECOMMENDS="python3-venv, xdg-desktop-portal, pipewire, qt6-wayland, grim, wl-clipboard, xclip"
SUGGESTS="gnome-shell, tesseract-ocr, tesseract-ocr-chi-sim, python3-rapidocr, python3-zxing-cpp, python3-pil"

# 3. 按已安装模块补充建议依赖说明
PLUGIN_NOTES=()
if [[ -e "$PACKAGE_DIR/usr/lib/mark-shot/libmark-shot-layer-shell.so" ]]; then
    PLUGIN_NOTES+=("includes layer-shell plugin")
fi
if [[ -e "$PACKAGE_DIR/usr/lib/mark-shot/plugins/libmark-shot-ocr-rapid.so" ]]; then
    PLUGIN_NOTES+=("includes ocr-rapid plugin")
fi
if [[ -e "$PACKAGE_DIR/usr/lib/mark-shot/plugins/libmark-shot-code-scan-zxing.so" ]]; then
    PLUGIN_NOTES+=("includes code-scan-zxing plugin")
fi
if [[ -e "$PACKAGE_DIR/usr/lib/mark-shot/plugins/libmark-shot-translate-openai.so" ]]; then
    PLUGIN_NOTES+=("includes translate-openai plugin")
fi

if [[ -n "$SHLIB_DEPS" ]]; then
    DEPENDS="${SHLIB_DEPS}, ${MANUAL_DEPS}"
else
    DEPENDS="$MANUAL_DEPS"
fi

if [[ "$COMPAT_CHECK" -eq 1 ]]; then
    # Debian 12 / Deepin 基线：拒绝 t64、过新 Qt、LayerShellQt 等
    if echo "$DEPENDS" | grep -Eq 't64|libstdc\+\+6 \(>= 14|libqt6[^,]+ \(>= 6\.(9|10)|liblayershellqtinterface|wl-clipboard'; then
        echo "Generated dependencies are too new for the Debian/Deepin compatible package:" >&2
        echo "$DEPENDS" >&2
        exit 1
    fi
fi

{
    echo "Package: mark-shot"
    echo "Version: ${VERSION}"
    echo "Section: graphics"
    echo "Priority: optional"
    echo "Architecture: ${DEB_ARCH}"
    echo "Maintainer: jswysnemc <snemc@qq.com>"
    echo "Depends: ${DEPENDS}"
    echo "Recommends: ${RECOMMENDS}"
    echo "Suggests: ${SUGGESTS}"
    echo "Description: Qt 6 screenshot selection and annotation tool"
    echo " Mark Shot captures screenshots, annotates image regions, pins floating"
    echo " image stickers, and provides OCR and translation helpers for pinned"
    echo " image text."
    echo " ."
    echo " Build target: ${TARGET}"
    if [[ ${#PLUGIN_NOTES[@]} -gt 0 ]]; then
        echo " ."
        echo " Modules: $(IFS=', '; echo "${PLUGIN_NOTES[*]}")"
    fi
} > "$CONTROL_DIR/control"

fakeroot dpkg-deb --build --root-owner-group "$PACKAGE_DIR" "$ASSET"
sha256sum "$ASSET" > "${ASSET}.sha256"

echo "Built ${ASSET}"
echo "asset=${ASSET}"
echo "checksum=${ASSET}.sha256"
echo "depends=${DEPENDS}"
