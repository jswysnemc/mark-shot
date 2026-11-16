/**
  【Nix媒体】【原生构建】按固定源码及依赖构建进程内媒体库，返回含对应源码、许可和摘要的软件包
*/
{
  lib,
  stdenv,
  python3,
  pkg-config,
  nasm,
  patchelf,
  source,
  mediaSources,
  dependencies,
}:
stdenv.mkDerivation {
  pname = "mark-shot-media";
  version = "0.1.0";
  src = source;
  nativeBuildInputs = [
    python3
    pkg-config
    nasm
    patchelf
  ];
  buildInputs = dependencies.mediaBuildInputs;
  dontConfigure = true;

  buildPhase = ''
    runHook preBuild
    # 1. 【Nix媒体】【离线准备】只使用 Nix 预取归档，沿用项目下载摘要和源码清单校验
    mkdir -p target/media-source-cache
    cp ${mediaSources.ffmpeg} target/media-source-cache/ffmpeg.tar.xz
    cp ${mediaSources.openh264} target/media-source-cache/openh264.tar.gz
    cp ${mediaSources.nv-codec-headers} target/media-source-cache/nv-codec-headers.tar.gz
    python scripts/packaging/fetch_media_sources.py \
      --destination "$TMPDIR/mark-shot-media-sources" --cache target/media-source-cache --root .
    export MARK_SHOT_MEDIA_SOURCES="$TMPDIR/mark-shot-media-sources"
    export MARK_SHOT_BUILD_JOBS="$NIX_BUILD_CORES"
    # 2. 【Nix媒体】【共用构建】复用当前原生库编译参数，保留 LGPL 配置及完整源码交付
    bash scripts/media/build-libraries.sh linux
    runHook postBuild
  '';
  installPhase = ''
    runHook preInstall
    mkdir -p "$out/lib/mark-shot/media"
    cp -a target/media-linux/. "$out/lib/mark-shot/media/"
    runHook postInstall
  '';
  postFixup = ''
    # 3. 【Nix媒体】【动态依赖】保留音频库和图形驱动搜索路径，不依赖启动入口提供环境变量
    library="$out/lib/mark-shot/media/libmarkshot-media.so"
    patchelf --set-rpath "$(patchelf --print-rpath "$library"):${dependencies.runtimeLibraryPath}" "$library"
    # 4. 【Nix媒体】【最终摘要】Nix 修正运行路径后仅刷新动态库摘要，原始源码摘要必须一致
    python scripts/packaging/media_checksums.py --directory "$out/lib/mark-shot/media" --refresh
  '';
  meta = {
    description = "In-process recording and export library for Mark Shot";
    license = with lib.licenses; [
      lgpl21Plus
      bsd2
    ];
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
  };
}
