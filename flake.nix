{
  description = "Mark Shot - a Qt6 screenshot & annotation tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        inherit (pkgs) lib;

        qtModules = with pkgs; [
          qt6.qtbase
          qt6.qtdeclarative
          qt6.qttools
          qt6.qtshadertools
          qt6.qtsvg
          qt6.qtmultimedia
          qt6.qtwayland
          qt6.qt5compat
          qt6.qtimageformats
          qt6.qtcharts
        ];

        # Runtime helpers invoked by the binary/scripts but not linked into the ELF.
        # Without grim, niri/wlroots sessions fall back to xdg-desktop-portal and can
        # take several seconds while the portal path also disturbs tiled layouts.
        runtimeBinaries = with pkgs; [
          grim
          wl-clipboard
          python3
        ];

        # layer-shell-qt is only linked by the optional dlopened plugin
        # (libmark-shot-layer-shell.so). wrapQtAppsHook therefore cannot discover it
        # from the main mark-shot ELF and must be told about the Wayland shell
        # integration plugin + library path explicitly; otherwise configureOverlay
        # fails and niri tiles a regular window, squeezing other columns.
        layerShellQt = pkgs.kdePackages.layer-shell-qt;
        qtPluginPrefix = pkgs.qt6.qtbase.qtPluginPrefix or "lib/qt-6/plugins";

        mark-shot = pkgs.stdenv.mkDerivation {
          pname = "mark-shot";
          version = "0.1.42";

          src = self;

          nativeBuildInputs = with pkgs; [
            cmake
            ninja
            pkg-config
            qt6.wrapQtAppsHook
          ];

          buildInputs = qtModules ++ [
            pkgs.ffmpeg
            layerShellQt
            pkgs.libportal
            pkgs.pipewire
            pkgs.libxcb
          ];

          cmakeFlags = [
            "-DCMAKE_BUILD_TYPE=Release"
            "-DMARK_SHOT_REQUIRE_FFMPEG=ON"
            "-DMARK_SHOT_WITH_LIBPORTAL=ON"
            "-DMARK_SHOT_REQUIRE_LAYER_SHELL=ON"
          ];

          qtWrapperArgs = [
            "--prefix PATH : ${lib.makeBinPath runtimeBinaries}"
            "--prefix QT_PLUGIN_PATH : ${layerShellQt}/${qtPluginPrefix}"
            "--prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [ layerShellQt ]}"
          ];

          meta = with lib; {
            description = "Qt 6 Wayland screenshot selection and annotation tool";
            homepage = "https://github.com/jswysnemc/mark-shot";
            license = licenses.mit;
            mainProgram = "mark-shot";
            platforms = platforms.linux;
          };
        };
      in
      {
        packages = rec {
          default = mark-shot;
          inherit mark-shot;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cmake
            ninja
            pkg-config
            gcc
          ];

          buildInputs = qtModules ++ [
            pkgs.ffmpeg
            layerShellQt
            pkgs.pipewire
            pkgs.libportal
            pkgs.libxcb
          ] ++ runtimeBinaries;

          shellHook = ''
            export Qt6_DIR="${pkgs.qt6.qtbase}/lib/cmake/Qt6"
            export CMAKE_PREFIX_PATH="${lib.concatStringsSep ":" qtModules}"
            export PATH="${lib.makeBinPath runtimeBinaries}:$PATH"
            export QT_PLUGIN_PATH="${layerShellQt}/${qtPluginPrefix}''${QT_PLUGIN_PATH:+:$QT_PLUGIN_PATH}"
          '';
        };
      });
}
