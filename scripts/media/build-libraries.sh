#!/usr/bin/env bash
# 【媒体构建】【进程内库】固定来源并构建单个私有动态库，不生成录制或转换可执行程序
set -euo pipefail
platform="${1:-linux}"
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
build_root="${MARK_SHOT_MEDIA_BUILD:-$task_root/target/media-build-$platform}"
destination="${MARK_SHOT_MEDIA_DIST:-$task_root/target/media-$platform}"
prefix="$build_root/prefix"
sources="$build_root/sources"
jobs="${MARK_SHOT_BUILD_JOBS:-2}"
mkdir -p "$sources" "$prefix" "$destination/licenses" "$build_root/ffmpeg"
source_bundle=false
if [[ -f "$sources/SOURCE-SHA256SUMS" ]]; then
  (cd "$sources"; sha256sum --check --status SOURCE-SHA256SUMS)
  source_bundle=true
fi

# 【媒体构建】【归档复用】按地址、路径和摘要校验缓存，只有缺失或不匹配才下载
fetch() {
  if [[ -f "$2" ]] && printf '%s  %s\n' "$3" "$2" | sha256sum --check --status; then return; fi
  python3 "$task_root/scripts/ci-download.py" "$1" "$2" "$3"
}

# 1. 【媒体构建】【来源固定】每次构建都核对归档摘要，不复用旧 GPL 媒体库
if [[ "$source_bundle" == false ]]; then
fetch https://ffmpeg.org/releases/ffmpeg-9.0.tar.xz \
  "$build_root/ffmpeg.tar.xz" 7f607a00dd0d28a729d5a4811205812eef01cf6ef6155025febb6f36a9062d52
tar -xf "$build_root/ffmpeg.tar.xz" -C "$sources"
fi
flags=(--disable-debug --disable-doc --disable-autodetect --disable-everything --disable-programs
  --disable-network --disable-avdevice --disable-shared --enable-static --enable-pic
  --enable-avfilter --enable-swscale --enable-swresample
  --enable-decoder=h264,aac --enable-parser=h264,aac --enable-encoder=gif,aac
  --enable-demuxer=mov,matroska --enable-muxer=mp4,matroska,gif
  --enable-protocol=file --enable-bsf=extract_extradata,h264_mp4toannexb
  --enable-filter=buffer,buffersink,scale,fps,format,split,palettegen,paletteuse
  --prefix="$prefix" --pkg-config-flags=--static)
compiler=cc
stripper=strip
library=libmarkshot-media.so
link_flags=(-Wl,-z,defs -Wl,-Bsymbolic -Wl,--exclude-libs,ALL)
native_libs=()
if [[ "$platform" == linux ]]; then
  native_libs=(-lgbm -ldl)
  if [[ "$source_bundle" == false ]]; then
  fetch https://codeload.github.com/cisco/openh264/tar.gz/refs/tags/v2.6.0 \
    "$build_root/openh264.tar.gz" 558544ad358283a7ab2930d69a9ceddf913f4a51ee9bf1bfb9e377322af81a69
  tar -xf "$build_root/openh264.tar.gz" -C "$sources"
  fi
  make -C "$sources/openh264-2.6.0" -j"$jobs" libraries BUILDTYPE=Release ENABLEPIC=Yes
  make -C "$sources/openh264-2.6.0" install-static PREFIX="$prefix"
  cp "$sources/openh264-2.6.0/LICENSE" "$destination/licenses/OPENH264.txt"
  if [[ "$source_bundle" == false ]]; then
  fetch https://github.com/FFmpeg/nv-codec-headers/archive/refs/tags/n13.0.19.0.tar.gz \
    "$build_root/nv-codec-headers.tar.gz" 86d15d1a7c0ac73a0eafdfc57bebfeba7da8264595bf531cf4d8db1c22940116
  tar -xf "$build_root/nv-codec-headers.tar.gz" -C "$sources"
  fi
  make -C "$sources/nv-codec-headers-n13.0.19.0" PREFIX="$prefix" install
  flags+=(--enable-libopenh264 --enable-encoder=libopenh264,h264_vaapi,h264_nvenc --enable-vaapi
    --enable-ffnvcodec --enable-nvenc --enable-cuda)
elif [[ "$platform" == windows ]]; then
  compiler=x86_64-w64-mingw32-gcc
  stripper=x86_64-w64-mingw32-strip
  library=markshot-media.dll
  flags+=(--enable-cross-compile --target-os=mingw32 --arch=x86_64 --cross-prefix=x86_64-w64-mingw32- --pkg-config=pkg-config)
  link_flags=(-static-libgcc -Wl,--exclude-all-symbols)
else
  echo "Unsupported media platform: $platform" >&2
  exit 1
fi
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig"
# 2. 【媒体构建】【最小依赖】一次链接共用媒体代码，库以 LGPL 配置构建，不启用 GPL 或 nonfree
(cd "$build_root/ffmpeg"; "$sources/ffmpeg-9.0/configure" "${flags[@]}"; make -j"$jobs"; make install)
if grep -Eq '^#define CONFIG_(GPL|NONFREE) 1' "$build_root/ffmpeg/config.h"; then
  echo 'Unsupported media license configuration' >&2
  exit 1
fi
read -r -a media_cflags <<< "$(pkg-config --cflags libavcodec libavformat libavutil libavfilter libswscale libswresample)"
read -r -a media_libs <<< "$(pkg-config --static --libs libavformat libavcodec libavfilter libswscale libswresample libavutil)"
"$compiler" -shared -fPIC -O2 -fvisibility=hidden -Wall -Wextra -Werror \
  "${media_cflags[@]}" "$task_root"/crates/shot-media/native/*.c "${link_flags[@]}" \
  -o "$build_root/$library" "${media_libs[@]}" "${native_libs[@]}"
"$stripper" "$build_root/$library"
cp "$build_root/$library" "$destination/$library"
cp "$sources/ffmpeg-9.0/COPYING.LGPLv2.1" "$destination/licenses/FFMPEG.txt"
cp "$build_root/ffmpeg/config.h" "$destination/FFMPEG-CONFIG.h"
cp "$build_root/ffmpeg/ffbuild/config.mak" "$destination/FFMPEG-CONFIG.mak"
# 3. 【媒体构建】【可重建交付】保留仓库目录结构与原始归档，源码包不夹带对象文件和已编译库
bundle="$(mktemp -d "$build_root/source-bundle.XXXXXX")"
trap 'rm -rf "$bundle"' EXIT
bundle_sources="$bundle/target/media-build-$platform/sources"
mkdir -p "$bundle/crates/shot-media" "$bundle/scripts/media" "$bundle_sources"
cp -a "$task_root/crates/shot-media/native" "$bundle/crates/shot-media/"
cp "$task_root/scripts/media/build-libraries.sh" "$bundle/scripts/media/"
cp "$task_root/scripts/ci-download.py" "$bundle/scripts/"
if [[ "$source_bundle" == true ]]; then
  # 4. 【媒体构建】【重建源码】只重新打包清单内的原始文件，不混入本次编译生成物
  (cd "$sources"; sed 's/^[^ ]*  //' SOURCE-SHA256SUMS | tar -cf - -T -) | tar -xf - -C "$bundle_sources"
else
  tar -xf "$build_root/ffmpeg.tar.xz" -C "$bundle_sources"
  if [[ "$platform" == linux ]]; then
    tar -xf "$build_root/openh264.tar.gz" -C "$bundle_sources" --exclude='openh264-*/res'
    tar -xf "$build_root/nv-codec-headers.tar.gz" -C "$bundle_sources"
  fi
fi
(cd "$bundle_sources"; find . -type f ! -name SOURCE-SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > SOURCE-SHA256SUMS)
printf 'Build from this directory: bash scripts/media/build-libraries.sh %s\nRequired tools: C/C++ compiler, make, nasm, pkg-config, Python 3. Linux also needs libva/libdrm/libgbm/PulseAudio development headers (libva-dev libdrm-dev libgbm-dev libpulse-dev on Ubuntu). Windows cross build needs MinGW-w64.\nOpenH264 test media (res/) omitted; all library source and build files are included unchanged. Full upstream URL and digest are in the build script.\n' "$platform" > "$bundle/BUILD.txt"
tar -cJf "$destination/MEDIA-SOURCES.tar.xz" -C "$bundle" .
cat > "$destination/NOTICES.txt" <<'NOTICE'
Mark Shot uses FFmpeg libraries under LGPL-2.1-or-later in a replaceable shared library.
FFmpeg: 9.0. GPL and nonfree components are disabled. No command-line media programs are included.
Linux software H.264: OpenH264 2.6.0 (BSD-2-Clause).
The matching sources, wrapper and build configuration are included in MEDIA-SOURCES.tar.xz.
System graphics drivers and desktop protocol libraries remain operating-system dependencies.
NOTICE
(cd "$destination"; sha256sum "$library" MEDIA-SOURCES.tar.xz > SHA256SUMS)
if [[ "$platform" == linux ]]; then
  ldd "$destination/$library" > "$destination/LINUX-RUNTIME.txt"
  if grep -E 'not found|libavcodec|libavformat|libavfilter|libx264' "$destination/LINUX-RUNTIME.txt"; then exit 1; fi
fi
