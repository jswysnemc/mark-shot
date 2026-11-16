/**
  【Nix开发】【开发环境】按固定工具链、编译依赖与媒体库返回可直接编译和录制的原生开发环境
*/
{
  pkgs,
  toolchain,
  dependencies,
  mediaLibrary,
}:
pkgs.mkShell {
  nativeBuildInputs =
    dependencies.nativeBuildInputs
    ++ dependencies.nativeCheckInputs
    ++ [
      toolchain
      pkgs.nasm
      pkgs.git
      pkgs.nixfmt
    ];
  buildInputs = dependencies.buildInputs ++ dependencies.mediaBuildInputs;
  MARK_SHOT_MEDIA_LIBRARY = "${mediaLibrary}/lib/mark-shot/media/libmarkshot-media.so";
  LD_LIBRARY_PATH = dependencies.runtimeLibraryPath;
}
