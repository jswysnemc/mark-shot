# Configuration

## Application Config

Mark Shot reads application settings from `~/.config/mark-shot/config.json` on Linux and the Qt application config directory on other platforms. If the file does not exist, Mark Shot creates a default `config.json` on first startup. Pinned windows use the OCR and translation settings in the same file. The default OCR helper prefers `rapidocr` and can fall back to `tesseract`; the translation helper calls an OpenAI-compatible `/chat/completions` endpoint.

<details>
<summary>Application Config JSON Example & Options Details (Click to expand)</summary>

```json
{
  "env": {
    "QT_FONT_DPI": 96
  },
  "ui": {
    "language": "system",
    "theme": "system"
  },
  "debug": {
    "enabled": false,
    "logPath": ""
  },
  "annotation": {
    "defaultTool": "move",
    "fullscreenDefaultTool": "laser",
    "defaultColor": "#FF4D4D"
  },
  "save": {
    "pathTemplate": "{pictures}/mark-shot/mark-shot-{datetime}.png"
  },
  "export": {
    "imageFrame": {
      "enabled": false,
      "padding": 112,
      "cornerRadius": 18,
      "shadowRadius": 72,
      "shadowOffsetY": 28,
      "shadowOpacity": 0.32
    }
  },
  "capture": {
    "wayland": {
      "kde": {
        "kwinScreenshot": {
          "enabled": true
        }
      }
    }
  },
  "shortcuts": {
    "tools": {
      "pen": "P",
      "rectangle": "R"
    },
    "actions": {
      "copy": "Ctrl+C",
      "save": "Ctrl+S",
      "pin": "Ctrl+P",
      "upload": "Ctrl+U"
    },
    "startup": {
      "colorPicker": "C",
      "ruler": "R",
      "codeScanner": "Q",
      "displayCapture": "D"
    }
  },
  "windows": {
    "tray": {
      "enabled": true
    },
    "hotkeys": {
      "capture": "Ctrl+Alt+S"
    }
  },
  "codeScan": {
    "command": "",
    "timeoutMs": 15000
  },
  "upload": {
    "command": "",
    "timeoutMs": 60000,
    "env": {
      "MARK_SHOT_UPLOAD_URL": "",
      "MARK_SHOT_UPLOAD_FIELD": "image",
      "MARK_SHOT_UPLOAD_API_KEY": "",
      "MARK_SHOT_UPLOAD_AUTH_SCHEME": "Bearer",
      "MARK_SHOT_UPLOAD_URL_PATH": "",
      "MARK_SHOT_UPLOAD_DELETE_URL_PATH": ""
    }
  },
  "pinnedWindow": {
    "autoOcr": false,
    "alwaysOnTop": true,
    "border": true,
    "borderColor": "#5EEAD4",
    "borderWidth": 2
  },
  "scrollCapture": {
    "frame": 5,
    "previewGap": 5,
    "hidePreviewDuringCapture": false
  },
  "windowDetection": {
    "enabled": true,
    "command": "mark-shot-window-detection-niri",
    "env": {
      "MARK_SHOT_NIRI_PANEL_EDGE": "top",
      "MARK_SHOT_NIRI_OFFSET_Y": 0
    },
    "timeoutMs": 1000
  },
  "ocr": {
    "enabled": true,
    "backend": "rapidocr",
    "command": "",
    "timeoutMs": 30000
  },
  "translation": {
    "autoAfterOcr": false,
    "targetLanguage": "Simplified Chinese",
    "apiBase": "https://api.openai.com/v1",
    "apiKeyEnv": "OPENAI_API_KEY",
    "apiKey": "",
    "model": "gpt-4o-mini",
    "temperature": 0.2,
    "timeoutMs": 60000,
    "timeoutSeconds": 60,
    "systemPrompt": "",
    "command": ""
  }
}
```

| Configuration Key | Data Type | Default Value | Description |
| :--- | :---: | :---: | :--- |
| `env` | Object | `{}` | Environment variables applied to the process before `QApplication` creation (e.g., `"QT_FONT_DPI": 96` to normalize high-DPI scaling). Alias: `environment`. |
| `ui.language` | String | `"system"` | Interface language. Supported values: `system` (follow system locale), `english`, `chinese`. Also accepts `en`/`zh`/`zh_cn`/`cn` variants. Supersedes the legacy root-level `language` key. Configurable from the General settings page. |
| `ui.theme` | String | `"system"` | Interface theme. Supported values: `system` (follow the Qt/desktop color scheme), `dark`, and `light`. The General settings page writes this value and the settings dialog applies it immediately. Supersedes the legacy root-level `theme` key. |
| `capture.freezeScope` | String | `"all-screens"` | Scope of displays frozen during region screenshot session. Only effective in multi-monitor setups when not capturing all outputs explicitly. Supported values: `all-screens` (freeze all monitors) and `cursor-screen` (freeze only the monitor containing the mouse cursor). Aliases: `freezeScope`, `freezeDisplayScope`, etc. |
| `capture.wayland.kde.kwinScreenshot.enabled` | Boolean | `true` | Whether to enable KWin `org.kde.KWin.ScreenShot2` restricted D-Bus interface screenshot capture on KDE Wayland. If disabled, fallback to standard Portal capture. |
| `debug.enabled` | Boolean | `false` | Enables debug logging on Linux and Windows. CLI `--debug` / `--no-debug` override this value; `DEBUG=1` still enables logging unless `--no-debug` is set. |
| `debug.logPath` | String | system temp `mark-shot-scroll.log` | Debug log destination. CLI `--debug-log` overrides this value; `MARK_SHOT_DEBUG_LOG` remains supported when no config or CLI path is set. |
| `annotation.defaultTool` | String | `"move"` | The default annotation tool active after selecting a region. Supported values: `move`, `select`, `pen`, `line`, `highlighter`, `rectangle`, `ellipse`, `arrow`, `text`, `number`, `mosaic`, `magnifier`, `laser`. Overridden by CLI `--default-tool`. |
| `annotation.fullscreenDefaultTool` | String | `"laser"` | The default tool active in fullscreen annotation mode. Overridden by CLI `--fullscreen-default-tool`. If configured as `move` in fullscreen, the program defaults to `select`. |
| `annotation.defaultColor` | String | `"#FF4D4D"` | Initial annotation color. Supports `#RRGGBB` (opaque) or `#RRGGBBAA` (with alpha). Overridden by CLI `--default-color`. |
| `save.pathTemplate` | String | `"{pictures}/mark-shot/mark-shot-{datetime}.png"` | Default PNG path used by Save and as the initial Save As filename. Parent directories are created before saving. Aliases include `save.path`, `save.location`, root `savePathTemplate`, and directory-only `save.directory`. |
| `save.directoryTemplate` | String | `""` | Directory-only save template. If set, filename automatically uses `mark-shot-{datetime}.png`. Aliases: `save.directory`, `save.dir`, `save.folder`. |
| `recording.storage.videoDirectory` | String | `"{pictures}/mark-shot/videos"` | Default output directory for MP4 recordings. Aliases include `recording.storage.videos`, `recording.storage.videoDir`, and `recording.output.videoDirectory`. |
| `recording.storage.gifDirectory` | String | `"{pictures}/mark-shot/gifs"` | Default output directory for GIF recordings. Aliases include `recording.storage.gifs`, `recording.storage.gifDir`, and `recording.output.gifDirectory`. |
| `export.imageFrame` | Boolean/Object | `false` | Optional Mac-style image frame for user-facing exports. Object form supports `enabled`, `padding` (`0`-`256`, default `112`), `cornerRadius` (`0`-`128`, default `18`), `shadowRadius` (`0`-`128`, default `72`), `shadowOffsetY` (`0`-`128`, default `28`), and `shadowOpacity` (`0.0`-`1.0`, default `0.32`). Applies to Save, Save As, Copy, Upload, Open With, and extension-command images; OCR, code scan, pinned windows, quick display capture, and scrolling capture keep the raw image. Set `enabled` to `true` to enable the framed export. |
| `shortcuts` | Object | - | Customizable keyboard shortcuts. Alias: `hotkeys` (or under `annotation.shortcuts`/`annotation.hotkeys`). See details below. |
| `windows.tray.enabled` | Boolean | `true` on Windows, `false` elsewhere | Starts tray mode automatically. The key name is kept for compatibility. Use `mark-shot --tray` to start tray mode without changing config, or `mark-shot --capture` to force one-shot capture when autostart is enabled. |
| `windows.hotkeys.capture` | String | `"Ctrl+Alt+S"` | Global hotkey for region capture while tray mode is running. Windows uses RegisterHotKey; supported Linux desktops use the desktop portal. Aliases include `hotkey`, `captureHotkey`, and `screenshot`. |
| `windows.hotkeys.fullscreen` | String | `""` | Optional global hotkey for fullscreen annotation capture while tray mode is running. Alias: `fullscreenHotkey`. The generated default config only writes the region capture hotkey. |
| `colorPicker.history` | Array | `[]` | Recent colors picked by the startup Color Picker tool. Stored as `#RRGGBBAA` strings, capped at 7 entries. Updated automatically whenever a color is confirmed in the color panel. |
| `codeScan.command` | String | `""` | Custom QR/barcode scanner command. Supports `{image}`, `{imagePath}`, and `{imageUrl}` placeholders; if none is present, Mark Shot appends the temporary PNG path. The command must print the same JSON shape as `mark-shot-code-scan`. Aliases: `codeScanner.command`, `barcodeScanner.command`, `barcode.command`. |
| `codeScan.timeoutMs` | Number | `15000` | Timeout for the code scanner command. Environment variable `MARK_SHOT_CODE_SCAN_TIMEOUT_MS` can override it. |
| `upload.command` | String | `""` | Custom image-host upload command. Supports `{image}`, `{imagePath}`, and `{imageUrl}` placeholders; if none is present, Mark Shot appends the temporary PNG path. The command must print a URL (JSON `{"url": "..."}` or plain text starting with `http`). Aliases: `imageUpload.command`, `uploader.command`, `imageHost.command`. Environment variable `MARK_SHOT_UPLOAD_COMMAND` can override it. |
| `upload.timeoutMs` | Number | `60000` | Timeout for the upload command. Environment variable `MARK_SHOT_UPLOAD_TIMEOUT_MS` can override it. |
| `upload.env` | Object | `{}` | Environment variables passed to the upload helper (built-in `mark-shot-upload` or custom `command`). Merges over the system environment. Aliases: `environment`, `envVars`, `variables`. See below for supported keys. |
| `pinnedWindow.autoOcr` | Boolean | `false` | Controls whether a pinned sticker window starts OCR text recognition in the background immediately on creation. If disabled, OCR runs on demand when Copy Image Text or Translate is chosen. Alias: `pinned`, `pin`. |
| `pinnedWindow.alwaysOnTop` | Boolean | `true` | Controls whether pinned sticker windows stay above normal windows. The pinned-window context menu can toggle this value and writes it back to `config.json`. Aliases: `stayOnTop`, `topmost`, `above`. On GNOME Wayland, the bundled helper extension is used when available. |
| `pinnedWindow.border` | Boolean/Object | `true` | Outer border configuration for pinned sticker windows. Can be a boolean, or an object containing `enabled` (bool), `color` (name/hex/RGBA object), and `width` (float, `1.0` to `12.0`). Also flat configs like `borderEnabled`, `borderColor`, and `borderWidth` are supported. |
| `scrollCapture.frame` | Boolean/Number/Object | `5` | Outer frame offset for scrolling capture. A number sets the pixel gap between the captured region and the frame; `false` disables the frame. Object form supports `enabled` and `gap`. Aliases: `captureFrame`, `border`, `outline`, plus flat `frameEnabled`/`frameGap`. |
| `scrollCapture.previewGap` | Number/Object | `5` | Pixel gap between the outer frame and the scrolling preview panel. The panel is placed around the frame using the first available non-overlapping position. Aliases: `previewDistance`, `previewOffset`, `panelGap`; object form supports `gap`. |
| `scrollCapture.hidePreviewDuringCapture` | Boolean | `false` | Hides the scrolling preview panel while capture is running even when the panel would fit, showing the floating drag handle instead. Pausing still reveals the preview panel. Aliases: `hidePreviewWhileCapturing`, `hidePanelDuringCapture`, `hideUiDuringCapture`; nested `scrollCapture.preview.hideWhileCapturing` is also supported. |
| `ocr.enabled` | Boolean | `true` | Controls whether OCR features are available. Does not enable pinned-window background OCR by itself. |
| `ocr.resultPanel` | Boolean/Object | `true` | Controls whether the main selection OCR flow opens an editable result window. Object form supports `enabled`, `show`, `visible`, or `use`. Aliases include `resultWindow`, `ocrResultPanel`, and `ocrResultWindow`. Environment variables `MARK_SHOT_OCR_RESULT_PANEL` and `MARK_SHOT_OCR_RESULT_WINDOW` override this config. |
| `translation.autoAfterOcr` | Boolean | `false` | Controls whether translation starts automatically after a successful pinned-window OCR result. If enabled, choosing Translate later displays the cached translation instantly. |
| `windowDetection.enabled` | Boolean | `true` | Controls window boundary recognition. Set to `false` to disable both built-in X11 window detection and configured external detection scripts. |
| `windowDetection.env` | Object | `{}` | Environment variables passed to the window boundary detection script. Alias: `environment`. <br>• **Niri Script**: Automatically reads DMS bar, dock, frame, and frame-exclusion settings. It also supports `MARK_SHOT_NIRI_PANEL_EDGE` (`top`/`bottom`/`left`/`right`/`none`) and pixel offsets `MARK_SHOT_NIRI_OFFSET_X/Y/WIDTH/HEIGHT`.<br>• **Hyprland Script**: Supports `MARK_SHOT_HYPRLAND_INCLUDE_INACTIVE` (`1`/`0`) and pixel offsets `MARK_SHOT_HYPRLAND_OFFSET_X/Y/WIDTH/HEIGHT`.<br>• **GNOME Script**: Supports pixel offsets `MARK_SHOT_GNOME_OFFSET_X/Y/WIDTH/HEIGHT`. |

##### Save Path Placeholders
Path values: `{home}` (user home), `{pictures}` (pictures directory), `{desktop}` (desktop directory), `{downloads}` (downloads directory), `{config}` (config directory), `{data}` (data directory); time values `{timestamp}`, `{timestamp.ms}`, `{yyyy}`, `{yy}`, `{MM}`, `{M}`, `{dd}`, `{d}`, `{HH}`, `{hh}`, `{mm}`, `{ss}`, `{zzz}`, `{date}`, `{time}`, `{datetime}`, and `{datetime:FORMAT}` such as `{datetime:yyyy-MM-dd_HH-mm-ss-zzz}`; geometry values `{selection.x}`, `{selection.y}`, `{selection.width}`, `{selection.height}`, `{selection.right}`, `{selection.bottom}`, `{selection.geometry}`, and the same `{source.*}` fields for the capture source; image/output values `{image.width}`, `{image.height}`, `{name}`, and `{ext}`. Relative expanded paths are resolved below the default pictures `mark-shot` directory, missing `.png` suffixes are appended, and unknown placeholders make the template fall back to the default path.

##### Keyboard Shortcut Config Details

The `shortcuts` node supports the following sub-nodes:
- **`tools`** (alias: `tool`, `toolShortcuts`): Keyboard shortcuts for switching tools (`move`, `select`, `pen`, `line`, `highlighter`, `rectangle`, `ellipse`, `arrow`, `text`, `number`, `mosaic`, `magnifier`, `laser`).
- **`actions`** (alias: `action`, `actionShortcuts`): Keyboard shortcuts for global actions (`copy`, `save`, `pin`, `upload`, `undo`, `redo`, `cancel`, `openWith`, `extensions`, `scrollCapture`, `ocrCopy`, `clear`, `toggleCaptureScope`, `toggleToolbarLayout`).
- **`startup`** (alias: `startupTools`, `selection`): Keyboard shortcuts for selection-phase tools (`colorPicker`, `ruler`, `codeScanner`, `displayCapture`).

*Shortcut values use Qt key-sequence text (e.g. `Ctrl+C`, `Ctrl+Shift+Z`, or `Alt+R`). Shortcut keys can also be specified directly at the root of `shortcuts`.*

</details>

### Persistent Tool Defaults

Mark Shot remembers the most recently used annotation tool defaults and restores them on the next launch, so the toolbar reflects the saved styles from the very first paint.

State is written to a dedicated file at `~/.config/mark-shot/annotation-state.json` (Linux) or the Qt application config directory on other platforms. The file is independent from `config.json`: it only stores transient tool defaults and can be deleted at any time to reset the editor to its built-in defaults.

The persisted snapshot covers:

- Active drawing color and opacity, plus text background color.
- Per-tool widths: pen, shape, number, mosaic block size, laser.
- Rectangle fill, corner radius, and style (`Stroke` / `Highlight` / `Invert`).
- Magnifier scale and lens shape (`Circle` / `Rectangle`).
- Arrow style, highlighter style, and number badge style.
- Text font family.

Writes go through `QSaveFile` for atomic commits and are triggered immediately after every default-changing entry point (slider release, picker confirm, style toggle, font selection, color palette pick, etc.), so a crash never leaves the file half-written. Application-wide settings such as shortcuts, save path templates, OCR, translation, and upload remain in `config.json`.

### Image Upload Configuration

The `upload` section configures the sidebar upload action. When `upload.command` is empty, Mark Shot uses the bundled `mark-shot-upload` helper, which reads its behavior entirely from environment variables in `upload.env`. This keeps the config clean—no long shell commands needed.

<details>
<summary><b>Supported <code>upload.env</code> keys</b> (consumed by the bundled helper):</summary>

| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| `MARK_SHOT_UPLOAD_URL` | Yes | — | Upload endpoint URL. |
| `MARK_SHOT_UPLOAD_FIELD` | No | `image` | Multipart form field name for the file. |
| `MARK_SHOT_UPLOAD_API_KEY` | No | — | API key/token sent via the auth header. |
| `MARK_SHOT_UPLOAD_AUTH_HEADER` | No | `Authorization` | Header name for authentication. |
| `MARK_SHOT_UPLOAD_AUTH_SCHEME` | No | `Bearer` | Auth scheme prefix. Set to empty string to send the raw API key as the header value (sm.ms / ImgURL style). |
| `MARK_SHOT_UPLOAD_URL_PATH` | No | auto-detect | Dotted JSON path to the uploaded URL (e.g. `data.url`, `data.link`). |
| `MARK_SHOT_UPLOAD_DELETE_URL_PATH` | No | auto-detect | Dotted JSON path to the delete URL. |
| `MARK_SHOT_UPLOAD_HEADER_<Name>` | No | — | Extra HTTP request headers. Example: `MARK_SHOT_UPLOAD_HEADER_X-Custom: foo`. |
| `MARK_SHOT_UPLOAD_FIELD_<Name>` | No | — | Extra multipart form fields. Example: `MARK_SHOT_UPLOAD_FIELD_album: 123`. |

</details>

<details>
<summary>Example: ImgURL V3</summary>

```json
{
  "upload": {
    "timeoutMs": 60000,
    "env": {
      "MARK_SHOT_UPLOAD_URL": "https://www.imgurl.org/api/v3/upload",
      "MARK_SHOT_UPLOAD_FIELD": "file",
      "MARK_SHOT_UPLOAD_API_KEY": "sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
      "MARK_SHOT_UPLOAD_URL_PATH": "data.url"
    }
  }
}
```

ImgURL V3 uses `Authorization: Bearer <token>` authentication (`AUTH_SCHEME` defaults to `Bearer`, no need to change).

</details>

<details>
<summary>Example: sm.ms</summary>

```json
{
  "upload": {
    "env": {
      "MARK_SHOT_UPLOAD_URL": "https://sm.ms/api/v2/upload",
      "MARK_SHOT_UPLOAD_FIELD": "smfile",
      "MARK_SHOT_UPLOAD_API_KEY": "your-token",
      "MARK_SHOT_UPLOAD_AUTH_SCHEME": "",
      "MARK_SHOT_UPLOAD_URL_PATH": "data.url"
    }
  }
}
```

</details>

<details>
<summary>Example: imgbb</summary>

```json
{
  "upload": {
    "env": {
      "MARK_SHOT_UPLOAD_URL": "https://api.imgbb.com/1/upload?key=YOUR_API_KEY",
      "MARK_SHOT_UPLOAD_FIELD": "image",
      "MARK_SHOT_UPLOAD_URL_PATH": "data.url",
      "MARK_SHOT_UPLOAD_DELETE_URL_PATH": "data.delete_url"
    }
  }
}
```

</details>

<details>
<summary>Example: litterbox (temporary host, no API key)</summary>

```json
{
  "upload": {
    "command": "curl -sf --max-time 30 -A 'Mozilla/5.0' -F reqtype=fileupload -F time=72h -F fileToUpload=@{image} https://litterbox.catbox.moe/resources/internals/api.php",
    "timeoutMs": 35000
  }
}
```

litterbox returns a plain-text URL (not JSON); Mark Shot auto-detects any stdout line starting with `http://` or `https://` as the upload result.

</details>

**Custom command output format**: If you set `upload.command` to a custom script, it must print the URL to stdout. Both JSON (`{"url": "https://..."}`) and plain text (a line starting with `http://` or `https://`) are accepted.

### Pre-Capture Window Detection & Script Contribution Guide

To ensure precise window boundary detection across different Wayland compositors, Mark Shot uses a flexible external script invocation mechanism. Users can configure a detection script via `windowDetection.command`. The script is responsible for querying window geometries from the compositor and outputting the data in a unified format for Mark Shot to consume.

Mark Shot also auto-selects the matching detection script at runtime by probing the current desktop environment (`XDG_SESSION_TYPE`, `XDG_CURRENT_DESKTOP`, etc.). Supported environments are GNOME, KDE Plasma, Hyprland, and Niri; other Wayland sessions fall back to the niri script, and X11 sessions use the built-in native X11 detector (empty command). If the configured `windowDetection.command` does not match the current environment, Mark Shot corrects it in memory without modifying your `config.json`, so manual configuration is optional.

The project bundles default window detection scripts for the following window managers:
- **Niri**: `mark-shot-window-detection-niri`
- **Hyprland**: `mark-shot-window-detection-hyprland`
- **GNOME Wayland**: `mark-shot-window-detection-gnome` (requires the bundled GNOME Shell helper extension)
- **KDE Plasma / KWin**: `mark-shot-window-detection-kde`

<details>
<summary><b>Expand/Collapse Window Detection Script Configuration & Contribution Guide</b></summary>

#### How to Contribute Adapters
Currently, the repository ships adapter scripts for niri, Hyprland, GNOME Wayland, and KDE Plasma (KWin Wayland).

We highly welcome and encourage community members to contribute adapter scripts for various desktop environments and Wayland compositors to expand compatibility. If you run Mark Shot on Hyprland, Sway, KDE (KWin Wayland), or GNOME (Mutter Wayland) and have configured a working script, please submit a Pull Request to share it with the community. Here are implementation guidelines for different environments:
- **Hyprland**: Use `hyprctl clients -j` and parse the output JSON.
- **Sway**: Use `swaymsg -t get_tree` to fetch the layout tree.
- **KDE / KWin**: Implement a simple KWin Script, or query KWin's D-Bus interfaces.
- **GNOME**: Use the bundled `mark-shot-window-detection-gnome` script together with the `mark-shot-scroll-helper@snemc.org` GNOME Shell extension, which exports Mutter window geometry over D-Bus.

If the script fails to execute or times out (default: `1000ms`), Mark Shot will proceed with screenshot capture normally and fall back to its internal X11-based window detector where applicable.

#### How to Use & Configure:
1. Copy the script corresponding to your compositor from the `scripts/` directory in the repository to a folder in your system `$PATH` (e.g. `~/.local/bin/` or `/usr/local/bin/`).
2. Make the script executable:
   ```bash
   chmod +x ~/.local/bin/mark-shot-window-detection-niri
   # or
   chmod +x ~/.local/bin/mark-shot-window-detection-hyprland
   # or
   chmod +x ~/.local/bin/mark-shot-window-detection-gnome
   # or
   chmod +x ~/.local/bin/mark-shot-window-detection-kde
   ```
3. Update your `~/.config/mark-shot/config.json` configuration file, specifying the script name or its absolute path in the `windowDetection.command` field:
   ```json
   "windowDetection": {
     "command": "mark-shot-window-detection-gnome"
   }
   ```

#### 1. Input Provided to the Script (Environment Variables)

When invoked, Mark Shot passes the following environment variables to provide context about the current screen capture session:

| Variable Name | Type | Description |
| :--- | :--- | :--- |
| `MARK_SHOT_CONFIG` | String | Path to the config file (typically `~/.config/mark-shot/config.json`) |
| `MARK_SHOT_CAPTURE_OUTPUT` | String | Name of the output display to capture (e.g., `eDP-1`, `DP-2`) |
| `MARK_SHOT_CAPTURE_ALL_OUTPUTS` | Integer | `1` to capture all outputs; `0` to capture only the active screen |
| `MARK_SHOT_CAPTURE_X` | Integer | The logical start X coordinate of the capture area in the compositor |
| `MARK_SHOT_CAPTURE_Y` | Integer | The logical start Y coordinate of the capture area in the compositor |
| `MARK_SHOT_CAPTURE_WIDTH` | Integer | Logical width of the capture area |
| `MARK_SHOT_CAPTURE_HEIGHT` | Integer | Logical height of the capture area |

> [!NOTE]
> All window coordinates returned by the script must be in **global logical compositor coordinates**, matching the screen geometry queried by Qt. This prevents scaling or layout offset issues in multi-monitor setups.

#### 2. Agreed Output JSON Formats

The script must print detected window metadata as JSON to its standard output (`stdout`). The parser supports a wide range of compatible formats:

##### Root Node Schema
The root element can be an object containing a `windows` or `windowGeometries` array, or it can be a JSON array directly.
- **Option A (Wrapped Object)**: `{ "windows": [ ... ] }` or `{ "windowGeometries": [ ... ] }`
- **Option B (Direct Object representing one window)**: `{ "x": 100, "y": 100, "w": 400, "h": 300 }`
- **Option C (Direct Array)**: `[ ... ]`

##### Window Geometry Object Structures
Each element in the array (or the root object itself) can take one of the following four formats:

- **Key-Value Properties (Object)**:
  Directly defines integer fields for positions and sizes.
  - Horizontal coordinate keys: `x` or `left`
  - Vertical coordinate keys: `y` or `top`
  - Width keys: `width` or `w`
  - Height keys: `height` or `h`
  *Example*: `{ "x": 100, "y": 200, "w": 800, "h": 600 }`

- **Nested Coordinates/Sizes (Object)**:
  Uses the `at` field (as `[x, y]`) and the `size` field (as `[width, height]`).
  *Example*: `{ "at": [100, 200], "size": [800, 600] }`

- **Array / Inner Array Representation**:
  Directly lists 4 integers: `[x, y, width, height]`.
  Alternatively, specified under the `rect` property of an object.
  *Example*: `[100, 200, 800, 600]` or `{ "rect": [100, 200, 800, 600] }`

- **Geometry String Representation**:
  Uses a string matching the `x,y widthxheight` pattern (allows negative coordinates and spaces).
  Alternatively, specified under the `geometry` property of an object.
  *Example*: `"100,200 800x600"` or `{ "geometry": "100,200 800x600" }`

##### Complete JSON Output Example

```json
{
  "windows": [
    {
      "x": 100,
      "y": 150,
      "width": 800,
      "height": 600
    },
    {
      "left": 950,
      "top": 150,
      "w": 400,
      "h": 300
    },
    {
      "at": [100, 800],
      "size": [800, 450]
    },
    {
      "rect": [950, 800, 400, 300]
    },
    {
      "geometry": "100,1300 800x600"
    },
    [950, 1300, 400, 300]
  ]
}
```

</details>

When installing manually, install `mark-shot`, `mark-shot-ocr`, `mark-shot-code-scan`, `mark-shot-translate`, and `mark-shot-upload` together. Otherwise OCR, code scanning, translation, or image upload cannot call the backend helpers.

---
