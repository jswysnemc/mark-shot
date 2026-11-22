/**
  【Nix打包】【共用依赖】按 pkgs 返回编译依赖、动态库搜索目录和隔离测试字体来源
*/
{ pkgs }:
rec {
  nativeBuildInputs = with pkgs; [
    pkg-config
    rustPlatform.bindgenHook
    makeWrapper
    python3
    cargo-about
    fontconfig
  ];
  nativeCheckInputs = with pkgs; [ dbus ];
  buildInputs = with pkgs; [
    fontconfig
    freetype
    openssl
    libxkbcommon
    libxcb
    pipewire
    wayland
  ];
  mediaBuildInputs = with pkgs; [
    libva
    libdrm
    libgbm
    libpulseaudio
  ];
  runtimeLibraries = with pkgs; [
    libpulseaudio
    stdenv.cc.cc.lib
  ];
  runtimeLibraryPath = pkgs.lib.concatStringsSep ":" [
    "/run/opengl-driver/lib"
    (pkgs.lib.makeLibraryPath runtimeLibraries)
  ];
  systemTestFontConfig = pkgs.makeFontsConf {
    fontDirectories = with pkgs; [
      dejavu_fonts
      noto-fonts-cjk-sans
    ];
    includes = [ "${pkgs.fontconfig.out}/etc/fonts/conf.d" ];
    impureFontDirectories = [ ];
  };
  testFontConfig = pkgs.writeText "mark-shot-fonts.conf" ''
    <?xml version="1.0"?>
    <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
    <fontconfig>
      <include>${systemTestFontConfig}</include>
      <alias binding="strong">
        <family>DejaVu LGC Sans</family>
        <prefer><family>DejaVu Sans</family></prefer>
      </alias>
    </fontconfig>
  '';
  referenceFontArchive = pkgs.fetchurl {
    url = "https://github.com/notofonts/latin-greek-cyrillic/releases/download/NotoSans-v2.015/NotoSans-v2.015.zip";
    sha256 = "0c34df072a3fa7efbb7cbf34950e1f971a4447cffe365d3a359e2d4089b958f5";
  };
}
