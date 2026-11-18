#!/usr/bin/env bash
# 【原生构建】【Linux发行包】编译本架构程序与可替换媒体库，拒绝 Qt 和外部编码程序依赖
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_TARGET_DIR="$root/target"
# 【原生构建】【链接配置】剥离发行版继承的 LTO 参数，避免 cc crate 产出 GCC slim-LTO 目标文件
# 发行版打包配置（如 Arch/Manjaro 的 makepkg.conf）默认在 CFLAGS 中追加 -flto=auto。
# libspa-sys 等 crate 通过 cc crate 编译 C 源码，会继承该参数并产出只含 .gnu.lto_* 段的
# slim-LTO 目标文件；rustc 1.90 起 x86_64-unknown-linux-gnu 默认使用 rust-lld，
# 它不加载 GCC LTO 插件，导致这些静态库的符号在链接期全部丢失。
# 追加在末尾可覆盖先前的 -flto，与 scripts/media/build-libraries.sh 的处理保持一致。
export CFLAGS="${CFLAGS:-} -fno-lto"
export CXXFLAGS="${CXXFLAGS:-} -fno-lto"
export LDFLAGS="${LDFLAGS:-} -fno-lto"
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
