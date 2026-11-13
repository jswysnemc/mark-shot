#!/usr/bin/env bash
# 【原生打包】【版权收集】固定工具与运行库通知摘要，输出当前架构实际依赖的版权资料
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
destination="${1:-$root/target/packaging-licenses}"
arch="$(uname -m)"
case "$arch" in
  x86_64) digest=3ad1e94ea1e63d69d1961f4fb15ccbbe9575d906111f96d2cb4dce6df48f71b4 ;;
  aarch64|arm64) arch=aarch64; digest=eb4d9ff6ebd2518152a8308729431a04b3836e71d2bd5ead7aea69f91a71162e ;;
  *) printf '【原生打包】【版权收集】不支持的架构: %s\n' "$arch" >&2; exit 1 ;;
esac
mkdir -p "$destination" "$root/target/packaging-tools/cargo-about"
if [[ -f "$root/reference-fonts/OFL.txt" ]]; then
  cp "$root/reference-fonts/OFL.txt" "$destination/LICENSE-REFERENCE-FONTS.txt"
fi
archive="$root/target/packaging-tools/cargo-about.tar.gz"
python3 "$root/scripts/ci-download.py" \
  "https://github.com/EmbarkStudios/cargo-about/releases/download/0.9.3/cargo-about-0.9.3-$arch-unknown-linux-musl.tar.gz" \
  "$archive" "$digest"
tar -xzf "$archive" --strip-components=1 -C "$root/target/packaging-tools/cargo-about"
"$root/target/packaging-tools/cargo-about/cargo-about" generate --locked --workspace \
  --target "$arch-unknown-linux-gnu" --fail --config "$root/release/about.toml" \
  --output-file "$destination/DEPENDENCIES.txt" "$root/release/license-notices.hbs"
python3 "$root/scripts/ci-download.py" \
  https://raw.githubusercontent.com/microsoft/onnxruntime/da9b5e364c465de65c49d91e696cd6485270757f/LICENSE \
  "$destination/LICENSE-ONNXRUNTIME.txt" 2f07c72751aed99790b8a4869cf2311df85a860b22ded05fa22803587a48922c
python3 "$root/scripts/ci-download.py" \
  https://raw.githubusercontent.com/microsoft/onnxruntime/da9b5e364c465de65c49d91e696cd6485270757f/ThirdPartyNotices.txt \
  "$destination/NOTICES-ONNXRUNTIME.txt" 0e07b95f3a8d6230037707c5c4a2b554d12c4cb67369669ac255635528ffcee2
