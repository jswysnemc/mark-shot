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

SHLIB_DEPS="$(dpkg-shlibdeps -O -e"$PACKAGE_DIR/usr/bin/mark-shot" | sed 's/^shlibs:Depends=//')"
MANUAL_DEPS="python3"
RECOMMENDS="python3-venv, xdg-desktop-portal, pipewire, qt6-wayland, grim, wl-clipboard, xclip"
SUGGESTS="gnome-shell, tesseract-ocr, tesseract-ocr-chi-sim"
if [[ -n "$SHLIB_DEPS" ]]; then
    DEPENDS="${SHLIB_DEPS}, ${MANUAL_DEPS}"
else
    DEPENDS="$MANUAL_DEPS"
fi

if [[ "$COMPAT_CHECK" -eq 1 ]]; then
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
} > "$CONTROL_DIR/control"

fakeroot dpkg-deb --build --root-owner-group "$PACKAGE_DIR" "$ASSET"
sha256sum "$ASSET" > "${ASSET}.sha256"

echo "Built ${ASSET}"
echo "asset=${ASSET}"
echo "checksum=${ASSET}.sha256"
