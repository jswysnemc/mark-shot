#!/usr/bin/env bash
# 【原生源码】【离线依赖】只准备 Linux Rust 依赖及媒体源码，不夹带预编译动态库
set -euo pipefail
source_root="${1:?source directory required}"
repository="$(cd "$(dirname "$0")/../.." && pwd)"
source_root="$(realpath "$source_root")"
test -f "$source_root/Cargo.lock"
test ! -e "$source_root/vendor"
before="$(sha256sum "$source_root/Cargo.lock")"
(cd "$source_root"; cargo vendor-filterer --platform='*-unknown-linux-gnu' --versioned-dirs \
  --exclude-crate-path='libloading-0.8.9#tests/nagisa*.dll' \
  --exclude-crate-path='libloading-0.9.0#tests/nagisa*.dll' \
  --exclude-crate-path='system-deps-7.0.8#src/tests/lib' \
  --exclude-crate-path='vcpkg-0.2.15#test-data' \
  --exclude-crate-path='ring-0.17.14#pregenerated/*.o' vendor)
if [[ "$(sha256sum "$source_root/Cargo.lock")" != "$before" ]]; then
  printf '【原生源码】【依赖锁定】准备依赖时 Cargo.lock 发生变化\n' >&2
  exit 1
fi
if find "$source_root/vendor" -type f \( -name '*.a' -o -name '*.lib' -o -name '*.dll' -o -name '*.so' -o -name '*.so.*' -o -name '*.o' -o -name '*.exe' \) -print -quit | grep -q .; then
  printf '【原生源码】【依赖校验】依赖目录含预编译库\n' >&2
  exit 1
fi
python3 "$repository/scripts/packaging/fetch_media_sources.py" \
  --root "$repository" --destination "$source_root/media" --cache "$repository/target/media-source-cache"
python3 "$repository/scripts/ci-reference-fonts.py" \
  --download-only --root "$source_root/reference-fonts"
(cd "$source_root"; bash "$repository/scripts/packaging/collect_licenses.sh" "$source_root/packaging/licenses")
cp "$source_root/reference-fonts/OFL.txt" "$source_root/packaging/licenses/LICENSE-REFERENCE-FONTS.txt"
