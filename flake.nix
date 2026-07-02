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

        mark-shot = pkgs.stdenv.mkDerivation {
          pname = "mark-shot";
          version = "0.1.32";

          src = self;

          nativeBuildInputs = with pkgs; [
            cmake
            ninja
            pkg-config
            qt6.wrapQtAppsHook
          ];

          buildInputs = qtModules ++ [
            pkgs.kdePackages.layer-shell-qt
            pkgs.libportal
            pkgs.pipewire
            pkgs.libxcb
          ];

          cmakeFlags = [
            "-DCMAKE_BUILD_TYPE=Release"
            "-DMARK_SHOT_WITH_LIBPORTAL=ON"
          ];
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
            pkgs.kdePackages.layer-shell-qt
            pkgs.pipewire
          ];

          shellHook = ''
            export Qt6_DIR="${pkgs.qt6.qtbase}/lib/cmake/Qt6"
            export CMAKE_PREFIX_PATH="${pkgs.lib.concatStringsSep ":" qtModules}"
          '';
        };
      });
}
