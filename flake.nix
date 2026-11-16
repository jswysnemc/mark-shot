/**
  【Nix打包】【构建入口】保留程序与开发环境输出，按固定工具链构建原生 Linux 双架构版本
*/
{
  description = "Mark Shot - native Rust screenshot, annotation and recording tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  /**
    【Nix打包】【平台输出】按 self 和固定输入返回 Linux 软件包、开发环境及安装验证
  */
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
        sources = import ./packaging/nix/source.nix {
          inherit (pkgs) lib;
          root = ./.;
        };
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
        sourceRevision =
          if self ? dirtyRev then
            self.dirtyRev
          else if self ? rev then
            self.rev
          else
            "local-working-tree";
        dependencies = import ./packaging/nix/dependencies.nix { inherit pkgs; };
        mediaSources = import ./packaging/nix/media-sources.nix { inherit (pkgs) fetchurl; };
        mediaLibrary = pkgs.callPackage ./packaging/nix/media-library.nix {
          source = sources.media;
          inherit mediaSources dependencies;
        };
        mark-shot = pkgs.callPackage ./packaging/nix/package.nix {
          source = sources.application;
          inherit
            version
            sourceRevision
            rustPlatform
            dependencies
            mediaLibrary
            ;
        };
      in
      {
        packages = {
          default = mark-shot;
          inherit mark-shot;
        };
        apps.default = flake-utils.lib.mkApp { drv = mark-shot; };
        devShells.default = import ./packaging/nix/dev-shell.nix {
          inherit
            pkgs
            toolchain
            dependencies
            mediaLibrary
            ;
        };
        checks = import ./packaging/nix/checks.nix {
          inherit pkgs version mark-shot;
        };
        formatter = pkgs.nixfmt;
      }
    );
}
