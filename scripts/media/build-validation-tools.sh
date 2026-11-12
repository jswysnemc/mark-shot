#!/usr/bin/env bash
# 【媒体验证】【独立解码器】仅供 CI 验收，产物不进入应用发布包
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
build_root="$task_root/target/media-validation-build-windows"
source_root="$task_root/target/media-build-windows/sources/ffmpeg-9.0"
destination="$task_root/target/media-validation-windows"
mkdir -p "$build_root" "$destination"
cd "$build_root"
"$source_root/configure" --disable-debug --disable-doc --disable-autodetect --disable-everything \
  --disable-network --disable-avdevice --disable-shared --enable-static \
  --enable-cross-compile --target-os=mingw32 --arch=x86_64 --cross-prefix=x86_64-w64-mingw32- \
  --enable-ffmpeg --enable-ffprobe --enable-avfilter --enable-swscale --enable-swresample \
  --enable-decoder=h264,aac,gif --enable-parser=h264,aac --enable-demuxer=mov,matroska,gif \
  --enable-muxer=null,rawvideo --enable-protocol=file,pipe \
  --enable-encoder=wrapped_avframe,pcm_s16le,rawvideo \
  --enable-filter=buffer,buffersink,scale,format,abuffer,abuffersink,aresample,aformat,anull,null \
  --extra-ldflags=-static
make -j"${MARK_SHOT_BUILD_JOBS:-2}"
cp ffmpeg.exe ffprobe.exe "$destination/"
x86_64-w64-mingw32-strip "$destination/ffmpeg.exe" "$destination/ffprobe.exe"
