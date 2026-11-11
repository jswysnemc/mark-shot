#!/usr/bin/env bash
# 【录制构建】【独立组件】构建裁剪媒体工具与 Linux GPU 录制器，产物包括源码和许可证
set -euo pipefail

platform="${1:-linux}"
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
build_root="${MARK_SHOT_MEDIA_BUILD:-$task_root/target/recording-build-$platform}"
destination="${MARK_SHOT_MEDIA_DIST:-$task_root/target/recording-components-$platform}"
prefix="$build_root/prefix"
sources="$build_root/sources"
jobs="${MARK_SHOT_BUILD_JOBS:-2}"
mkdir -p "$sources" "$prefix" "$destination/licenses"

# 1. 【录制构建】【来源固定】归档逐项验证摘要，Git 源码校验完整提交
python3 "$task_root/scripts/ci-download.py" \
  https://ffmpeg.org/releases/ffmpeg-9.0.tar.xz "$build_root/ffmpeg.tar.xz" \
  7f607a00dd0d28a729d5a4811205812eef01cf6ef6155025febb6f36a9062d52
tar -xf "$build_root/ffmpeg.tar.xz" -C "$sources"
ffmpeg_source="$sources/ffmpeg-9.0"
flags=(--disable-debug --disable-doc --disable-autodetect --disable-everything
  --disable-network --disable-avdevice --disable-shared --enable-static --enable-pic
  --enable-ffmpeg --enable-ffprobe --enable-avfilter --enable-swscale --enable-swresample
  --enable-decoder=h264,aac --enable-parser=h264,aac
  --enable-demuxer=mov,matroska,concat --enable-muxer=mp4,matroska,gif,null,rawvideo
  --enable-protocol=file,pipe --enable-bsf=extract_extradata,h264_mp4toannexb
  --enable-filter=buffer,buffersink,scale,fps,format,split,palettegen,paletteuse,abuffer,abuffersink,amix,aresample,aformat,anull,null,hwupload,scale_vaapi
  --prefix="$prefix")

if [[ "$platform" == windows ]]; then
  flags+=(--enable-cross-compile --target-os=mingw32 --arch=x86_64
    --cross-prefix=x86_64-w64-mingw32- --enable-encoder=gif,wrapped_avframe,pcm_s16le,rawvideo --pkg-config=pkg-config
    --extra-ldflags=-static)
elif [[ "$platform" == linux ]]; then
  python3 "$task_root/scripts/ci-download.py" \
    https://code.videolan.org/videolan/x264/-/archive/b35605ace3ddf7c1a5d67a2eb553f034aef41d55/x264-b35605ace3ddf7c1a5d67a2eb553f034aef41d55.tar.bz2 \
    "$build_root/x264.tar.bz2" 6eeb82934e69fd51e043bd8c5b0d152839638d1ce7aa4eea65a3fedcf83ff224
  tar -xf "$build_root/x264.tar.bz2" -C "$sources"
  x264_source="$sources/x264-b35605ace3ddf7c1a5d67a2eb553f034aef41d55"
  mkdir -p "$build_root/x264"
  (cd "$build_root/x264"; "$x264_source/configure" --prefix="$prefix" --enable-static --enable-pic --disable-cli --disable-opencl; make -j"$jobs"; make install)
  python3 "$task_root/scripts/ci-download.py" \
    https://github.com/FFmpeg/nv-codec-headers/archive/refs/tags/n13.0.19.0.tar.gz \
    "$build_root/nv-codec-headers.tar.gz" 86d15d1a7c0ac73a0eafdfc57bebfeba7da8264595bf531cf4d8db1c22940116
  tar -xf "$build_root/nv-codec-headers.tar.gz" -C "$sources"
  make -C "$sources/nv-codec-headers-n13.0.19.0" PREFIX="$prefix" install
  export PKG_CONFIG_PATH="$prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
  flags+=(--enable-encoder=gif,wrapped_avframe,pcm_s16le,rawvideo,aac,libx264,h264_vaapi,h264_nvenc
    --enable-gpl --enable-version3 --enable-libx264 --enable-vaapi
    --enable-ffnvcodec --enable-nvenc --enable-cuda --pkg-config-flags=--static)
else
  echo "Unsupported platform: $platform" >&2
  exit 1
fi

# 2. 【录制构建】【最小媒体工具】只编译本地 H.264/AAC、重封装与 GIF 所需组件
mkdir -p "$build_root/ffmpeg"
(cd "$build_root/ffmpeg"; "$ffmpeg_source/configure" "${flags[@]}"; make -j"$jobs"; make install)
if [[ "$platform" == windows ]]; then
  cp "$prefix/bin/ffmpeg.exe" "$prefix/bin/ffprobe.exe" "$destination/"
  x86_64-w64-mingw32-strip "$destination/ffmpeg.exe" "$destination/ffprobe.exe"
else
  cp "$prefix/bin/ffmpeg" "$prefix/bin/ffprobe" "$destination/"
  gsr_revision=f97d1e392816503282d20a0804c3ce3fd2943767
  vulkan_revision=88487f687f3100ebe1d3559a8a34d971ee85b977
  wf_revision=c5de47440e8e81c92befb696cb69819cdb3bfe8a
  for entry in "gpu-screen-recorder|https://repo.dec05eba.com/gpu-screen-recorder|$gsr_revision" "Vulkan-Headers|https://github.com/KhronosGroup/Vulkan-Headers.git|$vulkan_revision" "wf-recorder|https://github.com/ammen99/wf-recorder.git|$wf_revision"; do
    IFS='|' read -r name address revision <<< "$entry"
    if [[ ! -d "$sources/$name/.git" ]]; then git init "$sources/$name"; git -C "$sources/$name" remote add origin "$address"; fi
    git -C "$sources/$name" fetch --depth 1 origin "$revision"
    git -C "$sources/$name" checkout --detach FETCH_HEAD
    [[ "$(git -C "$sources/$name" rev-parse HEAD)" == "$revision" ]]
  done
  python3 "$task_root/scripts/recording/patch-gsr.py" "$sources/gpu-screen-recorder"
  # 3. 【录制构建】【静态媒体依赖】媒体库静态链接，桌面协议和驱动使用系统 ABI
  python3 - "$sources/gpu-screen-recorder/meson.build" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text()
for name in ['libavcodec', 'libavformat', 'libavutil', 'libswresample', 'libavfilter']:
    text = text.replace(f"dependency('{name}')", f"dependency('{name}', static : true)")
path.write_text(text)
PY
  export C_INCLUDE_PATH="$sources/Vulkan-Headers/include${C_INCLUDE_PATH:+:$C_INCLUDE_PATH}"
  meson setup "$build_root/gsr" "$sources/gpu-screen-recorder" --buildtype=release \
    -Dsystemd=false -Dcapabilities=false -Dnvidia_suspend_fix=false -Dapp_audio=false
  ninja -C "$build_root/gsr" -j "$jobs" gpu-screen-recorder
  cp "$build_root/gsr/gpu-screen-recorder" "$destination/"
  python3 "$task_root/scripts/recording/patch-wf.py" "$sources/wf-recorder"
  python3 - "$sources/wf-recorder/meson.build" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text()
for name in ['libavcodec', 'libavformat', 'libavutil', 'libswresample', 'libavfilter']:
    text = text.replace(f"dependency('{name}')", f"dependency('{name}', static : true)")
path.write_text(text)
PY
  meson setup "$build_root/wf" "$sources/wf-recorder" --buildtype=release -Dpipewire=disabled
  ninja -C "$build_root/wf" -j "$jobs"
  cp "$build_root/wf/wf-recorder" "$destination/"
  strip "$destination/wf-recorder"
  cp "$sources/wf-recorder/LICENSE" "$destination/licenses/WF-RECORDER.txt"
  strip "$destination/gpu-screen-recorder" "$destination/ffmpeg" "$destination/ffprobe"
  cp "$sources/gpu-screen-recorder/LICENSE" "$destination/licenses/GPU-SCREEN-RECORDER.txt"
  cp "$x264_source/COPYING" "$destination/licenses/X264.txt"
  python3 - "$sources/nv-codec-headers-n13.0.19.0/include/ffnvcodec" "$destination/licenses/NV-CODEC-HEADERS.txt" <<'PY'
from pathlib import Path
import sys
headers = sorted(Path(sys.argv[1]).glob('*.h'))
notices = [f"{header.name}\n{header.read_text().split('*/', 1)[0]}*/\n" for header in headers]
Path(sys.argv[2]).write_text('\n'.join(notices))
PY
  ldd "$destination/gpu-screen-recorder" > "$destination/LINUX-RUNTIME.txt"
  if grep -E 'not found|libavcodec|libavformat|libavfilter' "$destination/LINUX-RUNTIME.txt"; then exit 1; fi
fi

# 4. 【录制构建】【源码与许可】分发完整对应源码、补丁和构建参数，保留可重复构建入口
cp "$ffmpeg_source"/COPYING* "$destination/licenses/"
cp "$build_root/ffmpeg/config.h" "$destination/FFMPEG-CONFIG.h"
mkdir -p "$sources/mark-shot-build/scripts/recording"
cp "$task_root/scripts/recording/"*.sh "$task_root/scripts/recording/"*.py "$task_root/scripts/recording/"*.h "$task_root/scripts/recording/"*.hpp "$sources/mark-shot-build/scripts/recording/"
cp "$task_root/scripts/ci-download.py" "$sources/mark-shot-build/scripts/"
tar --exclude=.git -C "$sources" -cJf "$destination/COMPONENT-SOURCES.tar.xz" .
cat > "$destination/NOTICES.txt" <<'EOF'
Recording components are separate executables, not linked into Mark Shot.
FFmpeg 9.0: https://ffmpeg.org/ ; see COPYING files and FFMPEG-CONFIG.h.
Linux GPU Screen Recorder: GPL-3.0-only, source f97d1e392816503282d20a0804c3ce3fd2943767.
Linux x264: GPL-2.0-or-later; FFmpeg and wf-recorder media executables use GPL-3.0-or-later.
GPU Screen Recorder and its combined executable use GPL-3.0-only.
wf-recorder: MIT source c5de47440e8e81c92befb696cb69819cdb3bfe8a, linked with GPL media libraries.
Windows media export tools: LGPL-2.1-or-later; H.264/AAC capture uses Windows system APIs.
Corresponding source, integration modifications and build scripts are included in COMPONENT-SOURCES.tar.xz.
The Linux integration adds Portal GPU cropping and forces unprivileged X11 capture.
The wf-recorder integration adds pause control, capture pacing and a four-frame buffer limit.
No KMS helper, capabilities, systemd unit or NVIDIA configuration is installed.
EOF
(cd "$destination"; find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS)
