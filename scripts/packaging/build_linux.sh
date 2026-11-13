#!/usr/bin/env bash
# 【原生构建】【Linux发行包】编译本架构程序与可替换媒体库，拒绝 Qt 和外部编码程序依赖
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_TARGET_DIR="$root/target"
offline=()
configuration=()
if [[ "${1:-}" == --offline ]]; then
  offline=(--offline)
  export CARGO_NET_OFFLINE=true
  test -f "$root/media/SOURCE-SHA256SUMS"
  export MARK_SHOT_MEDIA_SOURCES="$root/media"
  test -d "$root/vendor"
  configuration=(--config 'source.crates-io.replace-with="vendored-sources"'
    --config "source.vendored-sources.directory=\"$root/vendor\"")
fi
cargo build --release --bin mark-shot --locked "${offline[@]}" "${configuration[@]}" -j "${MARK_SHOT_BUILD_JOBS:-2}"
bash scripts/media/build-libraries.sh linux > target/packaging-media.log 2>&1 || {
  tail -100 target/packaging-media.log >&2
  exit 1
}
target/release/mark-shot --help > target/packaging-help.txt
ldd target/release/mark-shot > target/packaging-linkage.txt
if grep -Ei 'not found|libQt|libgtk|webkit|webview|libavcodec|onnxruntime|webgpu_dawn' target/packaging-linkage.txt; then
  printf '【原生构建】【链接校验】主程序出现缺失或不允许的依赖\n' >&2
  exit 1
fi
mkdir -p target/packaging-licenses
if [[ -f packaging/licenses/DEPENDENCIES.txt ]]; then
  cp packaging/licenses/*.txt target/packaging-licenses/
elif [[ -f debian/dependency-licenses/DEPENDENCIES.txt ]]; then
  cp debian/dependency-licenses/*.txt target/packaging-licenses/
elif [[ "${#offline[@]}" == 0 ]]; then
  bash scripts/packaging/collect_licenses.sh
else
  printf '【原生构建】【离线版权】源码包缺少预先收集的依赖版权资料\n' >&2
  exit 1
fi
mkdir -p target/packaging-media-evidence
MARK_SHOT_MEDIA_LIBRARY="$root/target/media-linux/libmarkshot-media.so" \
  MARK_SHOT_MEDIA_EVIDENCE="$root/target/packaging-media-evidence" \
  cargo test -p shot-media --locked "${offline[@]}" "${configuration[@]}" -- --ignored --nocapture
