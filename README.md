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

> **Status.** This branch is the native rewrite (version 0.2.0, in development). The 0.1.x
> packages on the [Releases](https://github.com/jswysnemc/mark-shot/releases) page are the earlier
> Qt implementation. Linux on Wayland and X11 is the primary target. The Windows build runs, but
> has not been fully verified on real Windows machines yet.

## Features

| Area | What you get |
| --- | --- |
| Capture | Region, window, monitor and full-desktop capture, delayed capture, scrolling screenshots, headless capture with a JSON report |
| Annotation | Pen, highlighter, line, arrow, rectangle, ellipse, shape markers, numbering, text, mosaic and blur, magnifier, ruler, laser pointer |
| Editing | Move, resize and rotate annotations, multi-selection, undo and redo, canvas zoom and pan, export frame with padding and shadow |
| Pinning | Pin a capture, an image file or the clipboard as an always-on-top window, with OCR text selection and translation |
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

## Install

No native release has been published yet, so build from source for now. Rust 1.95 or newer is
required; `rustup` picks the pinned toolchain from `rust-toolchain.toml` automatically.

```sh
git clone https://github.com/jswysnemc/mark-shot.git
cd mark-shot
bash scripts/packaging/install_dependencies.sh   # Debian, Ubuntu, Fedora or Arch
cargo build --release --locked
./target/release/mark-shot --help
```

Screen recording additionally needs the bundled media library. See
[Installation](docs/installation.md) for that step, for per-distribution dependencies and for the
package formats produced by the release workflows.

## Quick start

```sh
mark-shot                    # select a region, then annotate it
mark-shot -r -t c            # drag a region and copy it immediately
mark-shot -m -f -t s         # save the monitor under the pointer
mark-shot --scroll           # scrolling screenshot
mark-shot --ocr              # recognize text in a region
mark-shot --record           # open the recording controller
mark-shot --pin-clipboard    # pin the clipboard image or text
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
| [Usage](docs/usage.md) | Capture modes, finishing a capture, pinning, history, scanning, upload, command line |
| [Annotation tools](docs/annotation-tools.md) | Every tool, selection editing and the export frame |
| [Shortcuts](docs/shortcuts.md) | Default key bindings and how to change them |
| [Configuration](docs/configuration.md) | Configuration file, settings window, environment variables, data locations |
| [OCR and translation](docs/ocr-and-translation.md) | Models, runtime, result window, translation services |
| [Screen recording](docs/recording.md) | Formats, audio, control commands, platform paths |
| [Desktop integration](docs/desktop-integration.md) | Compositor key bindings, global shortcut, tray, background service |
| [Platform support](docs/platform-support.md) | Per-desktop capabilities and limits |
| [Development](docs/development.md) | Repository layout, architecture, tests, packaging |

Chinese versions are in [docs/zh-CN](docs/zh-CN/README.md).

## Contributing

Bug reports and pull requests are welcome in the
[issue tracker](https://github.com/jswysnemc/mark-shot/issues). Include the desktop environment,
the display protocol and the output of `mark-shot --version` in bug reports.
[Development](docs/development.md) describes the checks a change is expected to pass.

## License

Mark Shot is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. The native Wayland interface, the capture backends and parts of the editor are based on
LumineCapture, which uses the same dual license. Bundled fonts are covered by the SIL Open Font
License ([Inter](assets/fonts/LICENSE-OFL.txt),
[Mark Shot Numerals](assets/fonts/LICENSE-NUMERALS.txt)); toolbar icons are covered by the
[MIT license](assets/icons/mark-shot/LICENSE).
