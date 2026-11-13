# Mark Shot documentation

**English** | [简体中文](zh-CN/README.md)

These pages describe the native Rust version of Mark Shot (0.2.0).

## For users

| Document | Read it when you want to |
| --- | --- |
| [Installation](installation.md) | Build Mark Shot, add the recording library, or learn which packages exist |
| [Usage](usage.md) | Take a screenshot, finish it, pin it, browse history, scan codes, upload, or use the command line |
| [Annotation tools](annotation-tools.md) | Learn what each tool does and how to edit existing annotations |
| [Shortcuts](shortcuts.md) | Look up or change a key binding |
| [Configuration](configuration.md) | Edit the configuration file or find where data is stored |
| [OCR and translation](ocr-and-translation.md) | Set up text recognition and a translation service |
| [Screen recording](recording.md) | Record the screen as MP4, MKV or GIF |
| [Desktop integration](desktop-integration.md) | Bind a key in your compositor, enable the tray icon or the global shortcut |
| [Platform support](platform-support.md) | Check what works on your desktop |

## For contributors

| Document | Content |
| --- | --- |
| [Development](development.md) | Repository layout, architecture, checks, packaging |

## Conventions

- `mark-shot` in a command means the built or installed executable. From a source checkout, use
  `./target/release/mark-shot`.
- Paths such as `~/.config/mark-shot/` are the Linux defaults. [Configuration](configuration.md)
  lists the Windows equivalents.
- Key names follow the configuration file: `Ctrl+Shift+S`, `Return`, `Escape`.
