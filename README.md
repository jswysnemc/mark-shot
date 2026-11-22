<div align="center">
  <img src="assets/hicolor/scalable/apps/mark-shot.svg" alt="Mark Shot logo" width="112" />
  <h1>Mark Shot</h1>
  <p>Native screenshot, annotation and screen recording for Linux and Windows.</p>
  <p><strong>English</strong> | <a href="README.zh-CN.md">简体中文</a></p>
  <p>
    <a href="https://github.com/jswysnemc/mark-shot/releases">
      <img src="https://img.shields.io/github/v/release/jswysnemc/mark-shot?color=6da0f2&labelColor=4a5054&label=release&style=flat-square&logo=github" alt="Release" />
    </a>
    <a href="https://gitter.im/mark-shot/community">
      <img src="https://img.shields.io/badge/gitter-join%20chat-46bc99?labelColor=4a5054&style=flat-square&logo=gitter" alt="Gitter" />
    </a>
    <img src="https://img.shields.io/badge/language-Rust-dfb56c?labelColor=4a5054&style=flat-square&logo=rust" alt="Language: Rust" />
    <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows-28c0e7?labelColor=4a5054&style=flat-square" alt="Platform: Linux, Windows" />
    <img src="https://img.shields.io/badge/display-Wayland%20%7C%20X11-9979d9?labelColor=4a5054&style=flat-square" alt="Display: Wayland, X11" />
    <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-92d076?labelColor=4a5054&style=flat-square" alt="License: MIT OR Apache-2.0" />
  </p>
</div>

Mark Shot freezes the screen, lets you select a region, annotate it, and then copy, save, pin,
recognize, translate or upload the result. It is written in Rust and draws its own interface in
software, so it does not depend on Qt, GTK or a web view.

> **Status.** `main` contains the native Rust rewrite.
> The earlier 0.1.x Qt implementation is preserved on the
> [`qt-old`](https://github.com/jswysnemc/mark-shot/tree/qt-old) branch.
> Linux on Wayland and X11 is the primary target. The Windows build runs, but
> has not been fully verified on real Windows machines yet.

## Features

| Area | What you get |
| --- | --- |
| Capture | Region, window, monitor and full-desktop capture, delayed capture, scrolling screenshots, headless capture with a JSON report |
| Annotation | Pen, highlighter, line, arrow, rectangle, ellipse, shape markers, numbering, text, mosaic and blur, magnifier, ruler, laser pointer |
| Editing | Move, resize and rotate annotations, multi-selection, undo and redo, canvas zoom and pan, export frame with padding and shadow |
| Pinning | Pin a capture, an image file or the clipboard as an always-on-top window, preserving clipboard HTML/RTF formatting: readable local images are rendered, network/unreadable images become labelled placeholders, image paragraphs become aligned rows, and `align`/centre layouts are kept (embedded RTF images and Office vector objects are not included), with OCR text selection and translation |
| OCR | Local PP-OCRv5 recognition, editable result window, table export to TSV, CSV, Markdown and HTML |
| Translation | Bilibili Index-Translate, OpenAI, Anthropic, Gemini, Tencent, Baidu, Youdao, or a custom command |
| Recording | MP4, MKV and GIF with pause and resume, system audio or microphone, hardware encoding where available |
| Extras | QR code and barcode scanning, capture history, image upload, tray icon, global shortcut, light and dark themes |

## Platform support

| Platform | Capture | Notes |
| --- | --- | --- |
| Wayland: niri, Sway and other wlroots compositors | ext-image-copy-capture, wlr-screencopy | Window snapping on niri and Sway |
| Wayland: KDE Plasma | KWin ScreenShot2, portal fallback | |
| Wayland: GNOME | Desktop portal with PipeWire | The portal asks for permission on first use |
| X11 | XGetImage with RandR | Window snapping through EWMH |
| Windows | GDI | Experimental |

Details and known limits are in [Platform support](docs/platform-support.md).

On GNOME Wayland, install the optional pin extension from this checkout with
`bash scripts/install-gnome-pin-extension.sh`, log out and back in, then run
`gnome-extensions enable mark-shot-pin@jswysnemc`. It raises only `mark-shot-pin` windows
once when they appear. You can turn off Always on Top in GNOME's window menu afterward;
exclusive fullscreen windows may still cover pins. Other Wayland compositors need
wlr-layer-shell for client-controlled pinning.

## Install

### 1. Package managers (recommended)

#### Arch Linux (AUR)

Mark Shot is available in the Arch User Repository (AUR) as `mark-shot-bin` (prebuilt binary, recommended) and `mark-shot` (built from source) as mutually exclusive options:

```sh
# Option A (recommended): install prebuilt binary
yay -S mark-shot-bin
# or with paru
paru -S mark-shot-bin
```

To build locally from source instead:

```sh
# Option B (alternative): build from source via AUR
yay -S mark-shot
```

Note: Package versions in the AUR are maintained independently and may lag behind GitHub Releases. You can verify the available version with `yay -Si mark-shot-bin`. If you need the latest release immediately, download `mark-shot-bin-*.pkg.tar.zst` from GitHub Releases and install with `sudo pacman -U`.

#### Debian / Ubuntu

Download the `.deb` package matching your distribution version and architecture from [Releases](https://github.com/jswysnemc/mark-shot/releases). For example, on Ubuntu 24.04 (x86_64):

```sh
sudo apt install ./mark-shot_0.2.4-1_amd64.ubuntu24.04.deb
```

Replace the filename with the package matching your distribution version and architecture.

#### Fedora 43

RPM packages are currently built for Fedora 43. Download the package for your architecture from [Releases](https://github.com/jswysnemc/mark-shot/releases), then install:

```sh
sudo dnf install ./mark-shot-0.2.4-1.fc43.x86_64.rpm
```

Replace the filename with the package matching your architecture.

#### Nix / NixOS

Run directly via Flakes or integrate into your configuration:

```sh
nix run github:jswysnemc/mark-shot
```

### 2. Universal packages and portable archives

- **AppImage**: Download `mark-shot-native-*-linux-x86_64.AppImage`, make it executable, and run:
  ```sh
  chmod +x mark-shot-native-*-linux-x86_64.AppImage
  ./mark-shot-native-*-linux-x86_64.AppImage
  ```
- **Flatpak**: Download the `.flatpak` bundle and install with `flatpak install mark-shot-native-*-linux-x86_64.flatpak`.
- **Portable archive**: Download the Linux `.tar.gz` or Windows `.zip` archive from [Releases](https://github.com/jswysnemc/mark-shot/releases), extract, and run.
- **Windows installer**: Download `mark-shot-native-*-windows-x86_64-msvc-setup.exe` and follow the setup wizard.

The release page contains a collapsed file list with SHA-256 values. See [Installation](docs/installation.md) for download verification and detailed format specifications.

### 3. Build from source

To build from source, Rust 1.95 or newer is required; `rustup` picks the pinned toolchain from
`rust-toolchain.toml` automatically.

```sh
git clone https://github.com/jswysnemc/mark-shot.git
cd mark-shot
bash scripts/packaging/install_dependencies.sh   # Debian, Ubuntu, Fedora or Arch
cargo build --release --locked
./target/release/mark-shot --help
```

Prebuilt packages include the recording media library. Source builds need to build it separately;
see [Installation](docs/installation.md) for that step and per-distribution dependencies.

## Quick start

```sh
mark-shot                    # select a region, then annotate it
mark-shot -r -t c            # drag a region and copy it immediately
mark-shot -m -f -t s         # save the monitor under the pointer
mark-shot --scroll           # scrolling screenshot
mark-shot --ocr              # recognize text in a region
mark-shot --record           # open the recording controller
mark-shot --pin-clipboard    # pin the clipboard image, rich text or text; local images render, network images become placeholders
mark-shot picture.png        # annotate an existing image
mark-shot --settings         # open preferences
```

Bind `mark-shot` to a key in your compositor, or enable the built-in global shortcut and tray icon
in the settings window. In the editor, `Enter` finishes, `Ctrl+C` copies, `Ctrl+S` saves, `Ctrl+P`
pins and `Esc` cancels.

## Documentation

| Document | Content |
| --- | --- |
| [Installation](docs/installation.md) | Build dependencies, building, the media library, packages |
| [Nix](docs/nix.md) | Native package, development shell and Flake checks |
| [Usage](docs/usage.md) | Capture modes, finishing a capture, pinning, history, scanning, upload, command line |
| [Annotation tools](docs/annotation-tools.md) | Every tool, selection editing and the export frame |
| [Shortcuts](docs/shortcuts.md) | Default key bindings and how to change them |
| [Configuration](docs/configuration.md) | Configuration file, settings window, environment variables, data locations |
| [OCR and translation](docs/ocr-and-translation.md) | Models, runtime, result window, translation services |
| [Screen recording](docs/recording.md) | Formats, audio, control commands, platform paths |
| [Desktop integration](docs/desktop-integration.md) | Compositor key bindings, global shortcut, tray, background service |
| [Platform support](docs/platform-support.md) | Per-desktop capabilities and limits |
| [Interface fonts](doc/ui-fonts.md) | Fontconfig matching and font rendering on Linux |
| [Development](docs/development.md) | Repository layout, architecture, tests, packaging |

Chinese versions are in [docs/zh-CN](docs/zh-CN/README.md).

## Contributing

Bug reports and pull requests are welcome in the
[issue tracker](https://github.com/jswysnemc/mark-shot/issues). Include the desktop environment,
the display protocol and the output of `mark-shot --version` in bug reports.
[Development](docs/development.md) describes the checks a change is expected to pass.

## Links

- [LINUX DO](https://linux.do/)

## License

Mark Shot is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. The native Wayland interface, the capture backends and parts of the editor are based on
LumineCapture, which uses the same dual license. Bundled fonts are covered by the SIL Open Font
License ([Inter](assets/fonts/LICENSE-OFL.txt),
[Mark Shot Numerals](assets/fonts/LICENSE-NUMERALS.txt)); toolbar icons are covered by the
[MIT license](assets/icons/mark-shot/LICENSE).
