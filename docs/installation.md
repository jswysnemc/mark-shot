# Installation

**English** | [简体中文](zh-CN/installation.md)

The current native prerelease is
[0.2.0-preview.7](https://github.com/jswysnemc/mark-shot/releases/tag/native-v0.2.0-preview.7).
The 0.1.x packages contain the earlier Qt implementation, preserved on `qt-old`.
This prerelease does not update stable AUR or PPA channels.

## Prebuilt downloads

Choose a package from the release page for your distribution and architecture. It provides
8 DEB packages, RPM binaries and source packages, Arch packages, AppImage, Flatpak,
Linux portable archives and a Windows x86_64 archive. Prebuilt packages include the recording
library and license notices; Rust is only needed for source builds.

For the Linux x86_64 portable archive, keep the extracted directory intact:

```sh
tar -xzf mark-shot-native-0.2.0-preview.7-linux-x86_64.tar.gz
./mark-shot-native-0.2.0-preview.7-linux-x86_64/mark-shot
```

Use the `linux-aarch64` archive on ARM64. On Windows, extract the complete ZIP and run
`mark-shot.exe`; do not separate it from the bundled media and resource files.

## Verify downloads

Expand **Files and SHA-256** on the release page and compare the complete digest with your file.
Checksum files and internal JSON metadata are not separate release assets.

```sh
sha256sum mark-shot-native-0.2.0-preview.7-linux-x86_64.tar.gz
```

On Windows, use PowerShell:

```powershell
Get-FileHash .\mark-shot-native-0.2.0-preview.7-windows-x86_64-msvc.zip -Algorithm SHA256
```

Release filenames use `.` instead of `~` to comply with GitHub asset naming. Debian and RPM
package versions still retain their prerelease ordering inside the package.

## Requirements

The requirements and commands below apply to source builds.

- Linux on x86_64 or aarch64, or Windows on x86_64
- Rust 1.95 or newer. The repository pins a toolchain in `rust-toolchain.toml`; `rustup` installs
  it automatically on the first build.
- The development packages listed below

## Build dependencies

`scripts/packaging/install_dependencies.sh` installs everything on Debian, Ubuntu, Fedora and Arch
Linux. It calls the system package manager through `sudo`:

```sh
bash scripts/packaging/install_dependencies.sh
```

To install the packages yourself:

**Debian and Ubuntu**

```sh
sudo apt install build-essential clang libclang-dev pkg-config git python3 curl \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libpipewire-0.3-dev \
  libssl-dev libfontconfig-dev fontconfig fonts-noto-core fonts-noto-cjk nasm \
  libva-dev libdrm-dev libgbm-dev libpulse-dev xz-utils zstd
```

**Fedora**

```sh
sudo dnf install gcc gcc-c++ make clang clang-devel pkgconf-pkg-config git python3 curl \
  wayland-devel libxkbcommon-devel libxkbcommon-x11-devel libxcb-devel pipewire-devel \
  openssl-devel fontconfig-devel google-noto-sans-fonts google-noto-sans-cjk-fonts nasm \
  libva-devel libdrm-devel mesa-libgbm-devel pulseaudio-libs-devel xz zstd
```

**Arch Linux**

```sh
sudo pacman -S --needed base-devel clang pkgconf git python curl wayland libxkbcommon \
  libxkbcommon-x11 libxcb pipewire openssl fontconfig noto-fonts noto-fonts-cjk nasm \
  libva libdrm mesa libpulse xz zstd
```

## Build

```sh
git clone https://github.com/jswysnemc/mark-shot.git
cd mark-shot
cargo build --release --locked
./target/release/mark-shot --version
```

The result is a single executable, `target/release/mark-shot`. Screenshots, annotation, pinning,
history and code scanning work with this file alone.

### Recording library

Screen recording loads a separate media library, which `cargo build` does not produce. Build it
once and place it in a `media` directory next to the executable:

```sh
bash scripts/media/build-libraries.sh linux
mkdir -p target/release/media
cp -r target/media-linux/. target/release/media/
```

The script downloads fixed FFmpeg and OpenH264 source archives, verifies their checksums and builds
one private shared library. No recording program is installed or started.

### OCR build options

The default build loads ONNX Runtime at run time. The library and the language models are
downloaded only after you click the download buttons in the OCR panel.

| Command | Result |
| --- | --- |
| `cargo build --release --locked` | Default. The inference library is downloaded from the OCR panel |
| `cargo build --release --locked --no-default-features --features bundled-ocr` | The inference library is embedded. Models are still downloaded on request |
| `cargo build --release --locked --no-default-features --features ocr-gpu` | Embedded library with WebGPU acceleration |

`ocr-dynamic` (the default) and `bundled-ocr` are mutually exclusive, so do not use
`--all-features`.

### Windows

Build with the MSVC toolchain:

```sh
cargo build --release --locked --target x86_64-pc-windows-msvc
```

The optional media library for MKV and GIF export is cross-compiled on Linux with
`bash scripts/media/build-libraries.sh windows`, which needs a MinGW toolchain. Place the result in
a `media` folder next to `mark-shot.exe`.

## Runtime dependencies

A source build links against the system libraries below. Packages declare them for you.

| Purpose | Libraries |
| --- | --- |
| Core | glibc, fontconfig, libxkbcommon, libxkbcommon-x11, libxcb, OpenSSL |
| Capture and recording | PipeWire, libpulse, libva, libdrm, Mesa (GBM) |
| Optional | `xdg-desktop-portal` for portal capture and Wayland global shortcuts; Noto fonts for CJK text |

Mark Shot does not use Qt, GTK or a web view, and it does not start external capture, clipboard or
encoding tools such as `grim`, `wl-copy`, `xclip` or `ffmpeg`. The only helper it calls is
`fc-match` from fontconfig, to match system fonts for text annotations.

## Packages

The release workflows build the following packages when a `native-v*` tag is published. Each
Linux package contains the executable, the media library and the license notices.

| Format | Targets | File name |
| --- | --- | --- |
| DEB | Debian 12 and 13, Ubuntu 24.04 and 26.04; amd64 and arm64 | `mark-shot_<version>_<arch><suffix>.deb` |
| RPM | Fedora; x86_64 and aarch64 | `mark-shot-<version>-1.<dist>.<arch>.rpm` |
| Arch package | x86_64 and aarch64 | `mark-shot-bin-<version>-1-<arch>.pkg.tar.zst` |
| AUR | `mark-shot` (source) and `mark-shot-bin` (prebuilt) | |
| AppImage | x86_64 | `mark-shot-native-<version>-linux-x86_64.AppImage` |
| Flatpak bundle | x86_64 | `mark-shot-native-<version>-linux-x86_64.flatpak` |
| Portable archive | Linux x86_64 and aarch64 | `mark-shot-native-<version>-linux-<arch>.tar.gz` |
| Windows archive | x86_64 MSVC | `mark-shot-native-<version>-windows-x86_64-msvc.zip` |

Installed packages use this layout:

```text
/usr/bin/mark-shot                      symlink to the executable
/usr/lib/mark-shot/mark-shot            executable
/usr/lib/mark-shot/media/               media library, its sources and licenses
/usr/share/applications/io.github.jswysnemc.MarkShot.desktop
/usr/share/icons/hicolor/scalable/apps/io.github.jswysnemc.MarkShot.svg
/usr/share/licenses/mark-shot/
```

[Development](development.md#packaging) explains how to produce this layout locally.

## After installing

1. Run `mark-shot` once to confirm that capture works on your desktop.
2. Bind it to a key. See [Desktop integration](desktop-integration.md).
3. Open `mark-shot --settings` to choose the theme, the save folder and the finish action.
