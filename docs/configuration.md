# Configuration

**English** | [简体中文](zh-CN/configuration.md)

Most options can be changed in the settings window. Everything is stored in one TOML file that
can also be edited by hand.

## Settings window

Open it with `mark-shot --settings`, `Ctrl+,` in the editor, or the settings button on the
toolbar.

| Page | Content |
| --- | --- |
| Appearance | Theme, language, tray icon |
| Capture | Finish action, double-click and middle-click actions, frozen displays, initial tool, window and region detection |
| Recording | Format, quality, encoder, frame rate, audio, save folder |
| Toolbar | Toolbar position and size, buttons of the main and action toolbars |
| Shortcuts | Every editor key binding and the global capture shortcut |
| OCR | Recognition engine, run mode, installed models |
| Translation | Target language, translation service and its credentials |
| Storage | Save folder, file name pattern, capture history |
| Pins & frames | Pinned image border, export frame with live preview |
| Integrations | Upload |
| Advanced | Logging, about |

`Ctrl+F` searches all settings. A field is saved when you press `Enter` or leave it; `Esc`
discards the edit. The footer shows when a change takes effect and the path of the configuration
file. Comments and unknown keys in the file are preserved.

## Configuration file

| Platform | Default path |
| --- | --- |
| Linux | `~/.config/mark-shot/config.toml` (follows `XDG_CONFIG_HOME`) |
| Windows | `%APPDATA%\mark-shot\config.toml` |

`--config FILE` selects another file for one run.

The first start writes only a minimal file. A key that is absent uses the built-in default, so
deleting a key restores its default. Print the complete annotated reference with:

```sh
mark-shot --print-default-config
```

If there is no `config.toml` but a `config.json` from the Qt version exists in the same folder,
Mark Shot reads that file and keeps writing to it in the same format.

## Common options

### Theme and language

```toml
[ui]
theme = "light"      # light | dark | system
language = ""        # empty or "system", "english", "chinese"
```

`Ctrl+Shift+L` in the editor switches between light and dark and saves the choice.

### Finish action and overlay

```toml
[general]
to = "cs"            # c copy, s save, p pin, o stdout; see the usage guide
overlay = "auto"     # auto | window | layer
```

`window` shows the editor as a fullscreen window, `layer` draws it above everything else, and
`auto` uses the layer on niri and COSMIC and a window elsewhere.

### Capture behaviour

```toml
[capture]
freeze_scope = "all-screens"     # all-screens | cursor-screen
hide_own_windows = true          # hide pins and other Mark Shot windows while capturing
kde_kwin_screenshot = true       # false uses the desktop portal on KDE
double_click_action = "copy"     # none | copy | save | save-as | pin | cancel
middle_click_action = "none"
toolbar_position = "selection"   # selection | top | bottom | left | right

[startup]
include_cursor = false

[region_detection]
enabled = true

[window_detection]
enabled = true
```

### Save location

```toml
[save]
directory = "screenshots"            # inside the system pictures folder
month_format = "%Y-%m"               # subfolder per month
filename_format = "%Y-%m-%d_%H-%M"
```

### Pinned images

```toml
[pinned]
always_on_top = true
border_enabled = true
border_color = "#5EEAD4"
border_width = 2.0
```

### Capture history

```toml
[history]
enabled = true
limit = 50               # 1 to 200
max_size_mib = 256
index_annotations = false
directory = ""           # empty uses the default data folder
```

`index_annotations` stores annotation text in the PNG metadata of history records. That text can
include content that was cropped out or covered in the final image, so it is off by default.

### Upload

The upload button sends a multipart POST request with the PNG. There is no default host.

```toml
[upload]
command = ""             # empty uses the built-in HTTP upload
timeout_ms = 60000

[upload.env]
MARK_SHOT_UPLOAD_URL = "https://image-host.example/upload"
MARK_SHOT_UPLOAD_FIELD = "image"
MARK_SHOT_UPLOAD_API_KEY = "token"
MARK_SHOT_UPLOAD_AUTH_HEADER = "Authorization"
MARK_SHOT_UPLOAD_AUTH_SCHEME = "Bearer"
MARK_SHOT_UPLOAD_URL_PATH = "data.url"
```

- `MARK_SHOT_UPLOAD_URL_PATH` is the dot-separated path of the image URL in the JSON response.
  Without it, `url`, `data.url`, `data.link`, `link` and `image.url` are tried in turn.
- Keys starting with `MARK_SHOT_UPLOAD_HEADER_` add request headers; keys starting with
  `MARK_SHOT_UPLOAD_FIELD_` add form fields.
- With `command` set, Mark Shot runs that command instead. `{image}` is replaced by the path of a
  temporary PNG, or the path is appended when there is no placeholder. The command prints the URL
  on its first output line, or a JSON object with a `url` field.

The same variables can be set in the process environment instead of the file.

### Notifications and logging

```toml
[notifications]
enabled = true
preview = true           # show the screenshot in the notification

[log]
level = "info"           # off | error | info
max_file_size_mb = 1
```

## All sections

| Section | Purpose |
| --- | --- |
| `[ui]` | Theme and language |
| `[general]` | Default finish action, overlay type |
| `[capture]`, `[startup]` | Capture behaviour, toolbar position |
| `[region_detection]`, `[window_detection]` | Automatic region and window candidates |
| `[save]`, `[export.image_frame]` | Save location, export frame |
| `[pinned]`, `[history]`, `[notifications]` | Pinned images, history, notifications |
| `[desktop]` | Tray icon and global shortcut, see [Desktop integration](desktop-integration.md) |
| `[recording]` | See [Screen recording](recording.md) |
| `[ocr]`, `[translation]` | See [OCR and translation](ocr-and-translation.md) |
| `[keys]` | See [Shortcuts](shortcuts.md) |
| `[toolbar_layout]`, `[toolbar]` | Toolbar buttons, icon and font size |
| `[tools]` and `[tools.*]` | Initial colour, width and style of each tool |
| `[upload]`, `[log]` | Upload, logging |
| `[theme]`, `[selection]`, `[annotations.*]`, `[input]`, `[animation]` and others | Advanced appearance and interaction tuning; the defaults rarely need changing |

`[tools]` only sets the values a tool starts with. Once a tool has been used, the last used values
in the state folder take precedence.

## Environment variables

| Variable | Effect |
| --- | --- |
| `MARK_SHOT_LANG` | Interface language; overrides `[ui].language` |
| `MARK_SHOT_BACKEND` | `auto`, `wayland` or `x11`. Forces the display protocol on Linux |
| `MARK_SHOT_CAPTURE` | `kde`, `image-copy`, `screencopy` or `portal`. Forces the Wayland capture method |
| `MARK_SHOT_LOG` | Log level: `error`, `warn`, `info`, `debug` or `trace` |
| `MARK_SHOT_CONFIG_DIR` | Folder containing a Qt-style `config.json` to use |
| `MARK_SHOT_MEDIA_LIBRARY` | Absolute path of the recording library, for development builds |
| `MARK_SHOT_UPLOAD_*` | Upload settings, see above |

A forced backend that cannot connect reports an error; Mark Shot does not silently fall back to
another display server.

## Data locations

| Data | Linux path |
| --- | --- |
| Configuration | `~/.config/mark-shot/` |
| Last used tool values, log file | `~/.local/state/mark-shot/` |
| OCR models | `~/.local/share/mark-shot/models/` |
| OCR inference library | `~/.local/share/mark-shot/runtime/` |
| Capture history | `~/.local/share/mark-shot/capture-history/` |
| Screenshots | `~/Pictures/screenshots/` |
| Recordings | `Mark Shot` inside the system videos folder |

The Linux paths follow the XDG base directory variables. On Windows, the configuration, the OCR
models and the inference library are stored under `%APPDATA%\mark-shot\`.
