# Nix

**English** | [简体中文](zh-CN/nix.md)

The Flake builds the native Rust application on `x86_64-linux` and `aarch64-linux`.
The application version comes from `Cargo.toml`; the compiler and components follow
`rust-toolchain.toml`. `flake.lock` pins nixpkgs, flake-utils and rust-overlay.

## Build and run

From the repository root:

```sh
nix build
nix build .#mark-shot
nix run . -- --settings
```

Both build commands produce the same package. `result/bin/mark-shot` starts the application.
The package includes the executable, desktop entry, icon, dependency license notices and a
replaceable in-process recording library with its matching sources.

The recording library uses the project's minimal FFmpeg/OpenH264 build. Nix retrieves the
source archives by fixed hashes before the build, and Cargo dependencies come from `Cargo.lock`.
Build phases use these local inputs. The default OCR feature remains dynamic: install the
runtime and language models through the application's OCR controls.

The running system supplies the desktop portal, audio service and graphics drivers. The package
provides library search paths for PulseAudio, the C++ runtime and `/run/opengl-driver/lib`.
These paths also apply when background services start the real executable directly. The application
uses the packaged `fc-match` by its full path, so font matching also works without it on the user's `PATH`.

## Development and verification

```sh
nix develop
cargo build --release --locked
./target/release/mark-shot --settings
```

The development shell provides the fixed Rust compiler, bindgen, native headers and a built
media library through `MARK_SHOT_MEDIA_LIBRARY`.

```sh
nix flake check
nix flake check --all-systems --no-build
nix fmt -- flake.nix packaging/nix/*.nix
```

The build runs workspace tests using private configuration directories and byte-pinned reference
fonts, followed by actual software encoding and export tests. Installed-package checks validate
CLI startup, desktop files, media ABI and checksums. The dedicated Nix workflow uses native
x86_64 and ARM64 runners.

The private application directory contains `SOURCE-COMMIT`. Git-based Flake inputs record their
revision, with a `-dirty` suffix for a modified working tree. A local path input without Git
metadata is marked `local-working-tree`.

## NixOS package selection

Add this repository as a Flake input and select the output for your system:

```nix
inputs.mark-shot.url = "github:jswysnemc/mark-shot";

environment.systemPackages = [
  inputs.mark-shot.packages.${pkgs.stdenv.hostPlatform.system}.default
];
```
