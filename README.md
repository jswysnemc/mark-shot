<div align="center">
  <img src="data/icons/hicolor/scalable/apps/mark-shot.svg" alt="Mark Shot Logo" width="128" />
  <h1>Mark Shot</h1>
  <p>
    <a href="https://github.com/jswysnemc/mark-shot/releases">
      <img src="https://img.shields.io/github/v/release/jswysnemc/mark-shot?color=6da0f2&labelColor=4a5054&label=release&style=flat-square&logo=github" alt="Release" />
    </a>
    <a href="https://gitter.im/mark-shot/community">
      <img src="https://img.shields.io/badge/gitter-join%20chat-46bc99?labelColor=4a5054&style=flat-square&logo=gitter" alt="Gitter" />
    </a>
    <img src="https://img.shields.io/badge/language-C%2B%2B-dfb56c?labelColor=4a5054&style=flat-square&logo=c%2B%2B" alt="Language C++" />
    <img src="https://img.shields.io/badge/framework-Qt%206-92d076?labelColor=4a5054&style=flat-square&logo=qt" alt="Framework Qt 6" />
    <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows-28c0e7?labelColor=4a5054&style=flat-square" alt="Platform Linux | Windows" />
    <img src="https://img.shields.io/badge/display-Wayland%20%7C%20X11-9979d9?labelColor=4a5054&style=flat-square" alt="Display Wayland | X11" />
    <img src="https://img.shields.io/badge/features-Screenshot%20%7C%20OCR%20%7C%20Pin%20%7C%20Scroll-ff8f59?labelColor=4a5054&style=flat-square" alt="Features Screenshot | OCR | Pin | Scroll" />
  </p>
</div>

[中文说明](README.zh-CN.md)

**Tags**: `C++` / `Qt 6` / `Screenshot` / `Annotation` / `Pin Sticker` / `OCR` / `Scroll Screenshot` / `Wayland` / `Windows`


<details>
<summary>Video Demo</summary>
<p align="center">
  <video src="https://github.com/user-attachments/assets/4f86fcee-fef9-409e-98ba-1491ecee06c7" width="100%" controls></video>
</p>
</details>

`mark-shot` is a high-performance screenshot and annotation tool built with Qt 6. Originally designed for Wayland compositors such as `niri`, it now supports standard screenshot and annotation workflows on Linux (X11, GNOME, and wlroots/Wayland desktops) as well as Windows environments.

It captures screen frames instantly and opens an interactive fullscreen overlay, providing region cropping, rich annotation, clipboard copying, saving, and desktop pinning features.

---

## Features

### Advanced Annotation Toolset
- **Pen & Highlighter**: Smooth freehand drawing and semi-transparent overlay highlighting.
- **Geometric Shapes**: High-precision Line, Rectangle, and Ellipse paths. Rectangle additionally supports a style selector with three modes:
  - `Stroke`: outlined or filled rectangle with optional rounded corners.
  - `Highlight`: marker-pen overlay rendered with `CompositionMode_Multiply` and a semi-transparent fill.
  - `Invert`: inverts the RGB pixels covered by the rectangle while keeping the outline as a visual cue.
- **Refined Arrow**: Sharp 6-vertex acute arrow path rendering with anti-aliasing.
- **Dual-Gesture Text**:
  - Supports dynamic, ultra-large font sizing with fluid adjustment via scroll wheel or property sliders.
  - Implements a physical width buffer to prevent unexpected wrapping across extreme scales.
  - **Diagonal handles** scale font size and boundary box proportionally; **side borders** only adjust wrap width.
- **Laser Pointer**: Dedicated presentation tool with pen traces that dissolve smoothly over time.
- **Auto-Increment Marker**: Click to stamp sequential numbered markers.
- **Mosaic**: Applies high-fidelity acrylic frost blur to obscure sensitive information.
- **Magnifier with Independent Frames**: The magnifier loupe exposes resize handles on both the inner source viewfinder and the outer lens. Rectangle lenses get 8 corner/edge handles per frame, circular lenses get 4. Resizing either frame keeps the magnification ratio constant by scaling the other frame proportionally; translating one frame leaves the other untouched.
- **Startup Code Scan**: Press `Q` before selecting a region, drag around a QR code or barcode, and open the decoded result in a copyable window.
- **Quick Display Capture**: Press `D` before selecting a region to instantly capture all outputs, crop them by display, and hover a thumbnail to copy, edit, or save that display image.
- **Image Host Upload**: Press `Ctrl+U` or click the toolbar upload button after selecting a region to upload the screenshot to a custom image host (ImgURL, sm.ms, imgbb, litterbox, etc.). The returned URL is automatically copied to the clipboard. Configure the host via `upload.env`, or plug in any custom uploader via `upload.command`.

### Pinned Window Stickers
- Pins any cropped region or annotated screenshot as an independent, frameless, and top-level floating window.
- Supports direct selection of OCR-recognized text in the pinned window, with `Ctrl + C` and context-menu copying.
- Supports OpenAI-compatible LLM translation for OCR text, rendering translated text back onto the image at the original layout positions.
- **Interactive Gestures**:
  - Drag with left click to reposition.
  - Scroll mouse wheel to scale.
  - Double left-click or press `Esc` to close.
  - Right-click to open a context menu with options to rotate, copy image text, translate, save, copy, or close.

### Scrolling Screenshot Capture
- Captures a long scrolling region by combining PipeWire screencast frames with an interactive scrolling overlay and stitcher.
- Designed primarily for `niri` and similar Wayland environments where output geometry and capture timing can be controlled predictably.
- **Floating Drag Handle for Large Regions**: When the selected capture region is too large to fit the preview panel on the screen, the preview panel is hidden, and a **floating drag handle** (a small floating button with direction arrows) is shown near the selection edge instead.
  - **Drag to reposition selection**: Press and drag the floating handle to slide the capture region along the active scroll axis. This allows adjusting the target area and reaching off-screen content.
  - **Click to toggle axis**: Click the handle directly before capture starts to switch between vertical and horizontal scroll directions.
- **GNOME Wayland**: scrolling capture requires the bundled `mark-shot-scroll-helper@snemc.org` GNOME Shell extension. GNOME does not expose the capture and preview hooks Mark Shot needs to normal desktop applications, so the extension provides a private D-Bus helper for area screenshots and the scroll preview panel.
- **Compatibility notice**: scrolling capture on KDE, X11, and other non-`niri` environments is a test feature and is not complete yet. Portal backends, shell policies, window geometry behavior, frame timing, and scroll event handling differ substantially across these desktop stacks.
- If scrolling capture fails, use normal screenshots or configure an external long-screenshot command through Mark Shot extension commands.
- To report a scrolling capture issue, run `mark-shot --debug --debug-log /path/to/mark-shot.log`, reproduce the failure, then attach the log to a GitHub issue. The same logging can be enabled through `debug.enabled` and `debug.logPath` in `config.json`; `DEBUG=1` and `MARK_SHOT_DEBUG_LOG=/path/to/log` remain supported.

### Cross-Platform Display Server Support
- **Wayland**: Uses PipeWire portal screencast for experimental scrolling capture, `grim` for wlroots screenshot capture, `layer-shell-qt` for native overlay, and `wl-copy` for clipboard persistence.
- **GNOME Wayland**: Uses the Mark Shot Scroll Helper GNOME Shell extension for scrolling capture. Without the extension, Mark Shot disables the scrolling capture action on GNOME Wayland.
- **X11**: Uses `QScreen::grabWindow` for screen capture, fullscreen top-level window for overlay, and `xclip` for clipboard persistence.
- **Windows**: Uses Qt's native screen capture and clipboard APIs for the core screenshot, annotation, copy, save, and pin workflows. Linux-specific backends such as PipeWire, xdg-desktop-portal, `grim`, XCB window detection, LayerShellQt, and GNOME Shell helpers are disabled at build time.
- Linux display server backends are detected at runtime via `$XDG_SESSION_TYPE`; Windows uses Qt's native platform backend.

### Desktop Integration
- **Desktop Entries**:
  - `mark-shot.desktop`: Configures the utility system-wide, triggerable by custom shortcuts.
  - `mark-shot-edit.desktop`: Registers as an image editor, enabling users to right-click local files in file managers (Dolphin, Nautilus, etc.) and open them directly in annotation mode.
- Ships with scalable vector icons (`mark-shot.svg` and `mark-shot-edit.svg`).

### KDE KWin ScreenShot2 Authorization

On KDE Wayland, Mark Shot can use KWin's `org.kde.KWin.ScreenShot2` interface for exact area capture. KWin treats this as a restricted D-Bus interface, so the application must have a desktop entry that declares the permission.

<details>
<summary>KDE KWin ScreenShot2 Authorization Details (Click to expand)</summary>

Desktop entry permission:
```ini
X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2
```

Distribution packages and `cmake --install` install the required desktop entries automatically. If you run a locally built binary without installing the project, create or update `~/.local/share/applications/mark-shot.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Mark Shot
Comment=Wayland screenshot selection and annotation tool
Exec=/absolute/path/to/mark-shot
Icon=mark-shot
Terminal=false
Categories=Graphics;Utility;
X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2
```

If you bind Mark Shot through KDE's command shortcut service, also create `~/.local/share/applications/net.local.mark-shot.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Mark Shot Shortcut Service
Exec=/absolute/path/to/mark-shot
Icon=mark-shot
Terminal=false
NoDisplay=true
StartupNotify=false
Categories=Utility;
X-KDE-GlobalAccel-CommandShortcut=true
X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2
```

After changing desktop entries, refresh KDE's desktop file cache by logging out and back in. If the current KDE session still returns `NoAuthorized`, restart KWin or reboot once.
</details>

---

## Usage

### Command Line Interface (CLI)

```bash
# Capture screen with interactive region selection
mark-shot

# Capture all outputs on a multi-monitor setup
mark-shot --all-outputs

# Annotate full captured screen directly (skipping selection)
mark-shot --fullscreen

# Start with Move after region selection, Laser in fullscreen, and a red default color
mark-shot --default-tool move --fullscreen-default-tool laser --default-color '#FF4D4D'

# Open and annotate an existing local image file
mark-shot path/to/image.png

# Open an existing image directly as a pinned sticker window
mark-shot --pin-image path/to/image.png

# Force standard XDG window instead of Wayland layer-shell
mark-shot --xdg-window
```

### CLI Arguments

| Option | Description |
| :--- | :--- |
| `[file]` | **Positional**: Opens an existing local image in annotation mode instead of capturing the screen. |
| `-h`, `--help` | Displays help information and exits. |
| `-v`, `--version` | Displays version information and exits. |
| `--all-outputs` | Captures all screens on the virtual display environment instead of only the active one. |
| `--xdg-window` | Forces the use of a standard XDG fullscreen window (xdg-shell) instead of layer-shell. |
| `--fullscreen` | Skips region selection and opens annotation mode on the full screen frame directly. |
| `--tray` | Keeps Mark Shot running in the system tray and registers global capture hotkeys when supported. |
| `--capture` | Forces one-shot capture when tray autostart is enabled in the config. |
| `--pin-image <path>` | Opens an existing local image directly as a pinned sticker window, skipping capture and region selection. |
| `--default-tool <tool>` | Sets the annotation tool selected after region selection. Also seeds fullscreen mode unless `--fullscreen-default-tool` is set. |
| `--fullscreen-default-tool <tool>` | Sets the annotation tool selected in fullscreen annotation mode. |
| `--default-color <color>` | Sets the default annotation color. Supports `#RRGGBB` and `#RRGGBBAA`. |
| `--debug` | Enables debug logging for this run. |
| `--no-debug` | Disables debug logging for this run, overriding config and environment variables. |
| `--debug-log <path>` | Writes debug logs to the specified path and enables debug logging unless `--no-debug` is also set. |

### Compositor / Desktop Hotkey Integration

To bind `mark-shot` to a system screenshot shortcut, configure your compositor or desktop environment.

**Tray mode**:
```powershell
mark-shot --tray
```

Tray mode registers these global hotkeys by default:
- `Ctrl+Alt+S`: start region capture.

The tray menu also provides Capture, Fullscreen Capture, and Quit actions.

**niri** (`~/.config/niri/config.kdl`):
```kdl
binds {
    Mod+Shift+S { spawn "mark-shot"; }
}
```

**Hyprland** (`~/.config/hypr/hyprland.conf`):
```ini
# Bind Super+Shift+S to start mark-shot selection
bind = SUPER SHIFT, S, exec, mark-shot
# Bind Print key to start mark-shot selection
bind = , Print, exec, mark-shot
```

**Sway / i3** (`~/.config/sway/config` or `~/.config/i3/config`):
```ini
# Bind Super+Shift+S to start mark-shot selection
bindsym Mod4+Shift+S exec mark-shot
# Bind Print key to start mark-shot selection
bindsym Print exec mark-shot
```

**GNOME** (via custom keyboard shortcut in Settings → Keyboard → Keyboard Shortcuts → Custom Shortcuts).

### Extension Commands

The right-side action toolbar includes an **Extensions** button. It reads user-defined commands from `~/.config/mark-shot/extensions.json`. The file can be either a JSON array or an object with a `commands` array.

```json
{
  "commands": [
    {
      "name": "Long screenshot",
      "command": "./target/release/wayscrollshot {slurp}",
      "workingDirectory": "~/Desktop/projects/wayscrollshot",
      "closeOnStart": true
    },
    {
      "name": "OCR selection",
      "command": "ocr-tool {image}",
      "saveImage": true
    }
  ]
}
```

`command` is executed through `$SHELL -c` on Unix-like systems and `%COMSPEC% /C` on Windows, so shell features work. Use `{slurp}` to pass the current selection as `x,y widthxheight` geometry. Use `{image}` or `{imagePath}` to pass the current rendered selection as a temporary PNG path, or `{imageUrl}` for a `file://` URL. These placeholders are shell-quoted automatically. Set `saveImage` or `needsImage` to `true` to append the temporary PNG path when no image placeholder is present. `workingDirectory` and `cwd` are aliases. `closeOnStart` defaults to `true`, hiding and closing Mark Shot before the command starts.

### Application Config

Mark Shot reads application settings from `~/.config/mark-shot/config.json` on Linux and the Qt application config directory on other platforms. If the file does not exist, Mark Shot creates a default `config.json` on first startup. Pinned windows use the OCR and translation settings in the same file. The default OCR helper prefers `rapidocr` and can fall back to `tesseract`; the translation helper calls an OpenAI-compatible `/chat/completions` endpoint.

<details>
<summary>Application Config JSON Example & Options Details (Click to expand)</summary>

```json
{
  "env": {
    "QT_FONT_DPI": 96
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
| `capture.freezeScope` | String | `"all-screens"` | Scope of displays frozen during region screenshot session. Only effective in multi-monitor setups when not capturing all outputs explicitly. Supported values: `all-screens` (freeze all monitors) and `cursor-screen` (freeze only the monitor containing the mouse cursor). Aliases: `freezeScope`, `freezeDisplayScope`, etc. |
| `capture.wayland.kde.kwinScreenshot.enabled` | Boolean | `true` | Whether to enable KWin `org.kde.KWin.ScreenShot2` restricted D-Bus interface screenshot capture on KDE Wayland. If disabled, fallback to standard Portal capture. |
| `debug.enabled` | Boolean | `false` | Enables debug logging on Linux and Windows. CLI `--debug` / `--no-debug` override this value; `DEBUG=1` still enables logging unless `--no-debug` is set. |
| `debug.logPath` | String | system temp `mark-shot-scroll.log` | Debug log destination. CLI `--debug-log` overrides this value; `MARK_SHOT_DEBUG_LOG` remains supported when no config or CLI path is set. |
| `annotation.defaultTool` | String | `"move"` | The default annotation tool active after selecting a region. Supported values: `move`, `select`, `pen`, `line`, `highlighter`, `rectangle`, `ellipse`, `arrow`, `text`, `number`, `mosaic`, `magnifier`, `laser`. Overridden by CLI `--default-tool`. |
| `annotation.fullscreenDefaultTool` | String | `"laser"` | The default tool active in fullscreen annotation mode. Overridden by CLI `--fullscreen-default-tool`. If configured as `move` in fullscreen, the program defaults to `select`. |
| `annotation.defaultColor` | String | `"#FF4D4D"` | Initial annotation color. Supports `#RRGGBB` (opaque) or `#RRGGBBAA` (with alpha). Overridden by CLI `--default-color`. |
| `save.pathTemplate` | String | `"{pictures}/mark-shot/mark-shot-{datetime}.png"` | Default PNG path used by Save and as the initial Save As filename. Parent directories are created before saving. Aliases include `save.path`, `save.location`, root `savePathTemplate`, and directory-only `save.directory`. |
| `save.directoryTemplate` | String | `""` | Directory-only save template. If set, filename automatically uses `mark-shot-{datetime}.png`. Aliases: `save.directory`, `save.dir`, `save.folder`. |
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
| `windowDetection.env` | Object | `{}` | Environment variables passed to the window boundary detection script. Alias: `environment`. <br>• **Niri Script**: Supports `MARK_SHOT_NIRI_PANEL_EDGE` (`top`/`bottom`/`left`/`right`/`none`) and pixel offsets `MARK_SHOT_NIRI_OFFSET_X/Y/WIDTH/HEIGHT`.<br>• **Hyprland Script**: Supports `MARK_SHOT_HYPRLAND_INCLUDE_INACTIVE` (`1`/`0`) and pixel offsets `MARK_SHOT_HYPRLAND_OFFSET_X/Y/WIDTH/HEIGHT`.<br>• **GNOME Script**: Supports pixel offsets `MARK_SHOT_GNOME_OFFSET_X/Y/WIDTH/HEIGHT`. |

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

## Compilation & Installation

### Installation Guide

##### Arch Linux (AUR)
Arch Linux users can install directly from the AUR using helpers like `paru` or `yay`:
```bash
# Build from source
paru -S mark-shot
# or
yay -S mark-shot

# Install the prebuilt binary package instead
paru -S mark-shot-bin
# or
yay -S mark-shot-bin
```

`mark-shot` compiles from source; `mark-shot-bin` downloads the prebuilt pacman package from GitHub Releases.

##### NixOS
NixOS users can install mark-shot by adding it as a Flake input:
```nix
# flake.nix
mark-shot = {
  url = "github:jswysnemc/mark-shot";
  inputs.nixpkgs.follows = "nixpkgs";
};

# home-manager
home.packages = with pkgs; [
  # other user packages
  inputs.mark-shot.packages.${pkgs.stdenv.hostPlatform.system}.default
]
```

##### Other Distributions (Pre-built Packages)
For other distributions (such as Debian, Ubuntu, or Fedora), download the compiled package from the Releases page and install it via:
- **Debian / Ubuntu**:
  ```bash
  sudo apt install ./mark-shot_<version>_amd64.deb
  ```
- **Fedora**:
  ```bash
  sudo dnf install ./mark-shot-<version>-1.x86_64.rpm
  ```

The official `.deb` package is built on a Debian 12 compatibility baseline. It intentionally avoids linking the optional LayerShellQt plugin so that Deepin and other Debian-derived systems with Qt 6.8-era packages can install it without Ubuntu `t64` or newer GCC runtime dependencies.

### Dependencies

#### Wayland (Arch Linux)

```bash
sudo pacman -S --needed base-devel cmake ninja pkgconf qt6-base qt6-wayland layer-shell-qt pipewire grim wl-clipboard
```

#### X11/GNOME (Ubuntu/Debian)

```bash
# Build essentials
sudo apt install build-essential cmake ninja-build pkg-config libpipewire-0.3-dev

# Portal and clipboard tools
sudo apt install xdg-desktop-portal pipewire xclip

# Qt 6 (if not available in system repos, install via aqtinstall)
pip install aqtinstall
aqt install-qt linux desktop 6.7.3 gcc_64 --outputdir ~/Qt
```

> **Note**: On Ubuntu 22.04 where the system ships Qt 5, installing Qt 6 to `~/Qt` keeps the system untouched. Pass `-DCMAKE_PREFIX_PATH=$HOME/Qt/6.7.3/gcc_64` when configuring.

#### fcitx5 Chinese Input Method (Qt 6 on X11)

Qt 6 does not ship a fcitx5 input context plugin. To enable Chinese input, build the plugin from source:

```bash
sudo apt install libfcitx5utils-dev libfcitx5config-dev libfcitx5core-dev libfcitx5-qt-dev extra-cmake-modules

git clone --depth 1 --branch 5.0.10 https://github.com/fcitx/fcitx5-qt.git /tmp/fcitx5-qt
cmake -B /tmp/fcitx5-qt/build -S /tmp/fcitx5-qt \
  -DCMAKE_PREFIX_PATH=$HOME/Qt/6.7.3/gcc_64 \
  -DENABLE_QT4=OFF -DENABLE_QT5=OFF -DENABLE_QT6=ON
cmake --build /tmp/fcitx5-qt/build

cp /tmp/fcitx5-qt/build/qt6/platforminputcontext/libfcitx5platforminputcontextplugin.so \
   ~/Qt/6.7.3/gcc_64/plugins/platforminputcontexts/
cp /tmp/fcitx5-qt/build/qt6/dbusaddons/libFcitx5Qt6DBusAddons.so* \
   ~/Qt/6.7.3/gcc_64/lib/
```

#### OCR Backend (Optional)

Mark Shot delegates text recognition to the bundled `mark-shot-ocr` Python script. It supports **RapidOCR** (primary, based on PaddleOCR PP-OCR models) and **Tesseract** (fallback). On Linux the script is installed automatically; on Windows it must be configured manually.

<details>
<summary><b>Linux</b></summary>

```bash
python3 -m venv ~/.local/share/mark-shot/ocr-venv
~/.local/share/mark-shot/ocr-venv/bin/pip install -U pip rapidocr onnxruntime
```

The installed `mark-shot-ocr` helper is discovered automatically—no config changes needed.

**Environment variables** (optional):

| Variable | Description | Default |
|----------|-------------|---------|
| `MARK_SHOT_OCR_VERSION` | PaddleOCR version (`PP-OCRv5`, `PP-OCRv4`, …) | `PP-OCRv5` |
| `MARK_SHOT_OCR_MODEL_TYPE` | Model size: `mobile` or `server` | `mobile` |
| `MARK_SHOT_OCR_MODEL_DIR` | Custom model storage directory | `~/.local/share/mark-shot/models` |
| `MARK_SHOT_OCR_NO_VENV` | Set to `1` to disable automatic venv re-exec | — |
| `MARK_SHOT_OCR_PYTHON` | Override the Python interpreter used for re-exec | `~/.local/share/mark-shot/ocr-venv/bin/python` |

</details>

<details>
<summary><b>Windows</b></summary>

The bundled helper scripts are not installed on Windows. Complete the following steps to enable OCR:

**1. Install Python 3**

Download and install Python 3.10 or later from [python.org](https://www.python.org/downloads/). Make sure to check **Add python.exe to PATH** during installation.

**2. Copy the OCR helper script**

Copy `scripts/mark-shot-ocr` from the [Mark Shot repository](https://github.com/jswysnemc/mark-shot) to a local directory, for example `%LOCALAPPDATA%\mark-shot\mark-shot-ocr.py`.

```powershell
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\mark-shot"
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/jswysnemc/mark-shot/main/scripts/mark-shot-ocr" `
  -OutFile "$env:LOCALAPPDATA\mark-shot\mark-shot-ocr.py"
```

**3. Create a virtual environment and install dependencies**

```powershell
python -m venv "$env:LOCALAPPDATA\mark-shot\ocr-venv"
& "$env:LOCALAPPDATA\mark-shot\ocr-venv\Scripts\pip.exe" install -U pip rapidocr onnxruntime
```

> `onnxruntime` provides CPU-based inference. If you have a compatible GPU, you can install `onnxruntime-directml` or `onnxruntime-gpu` instead for faster recognition.

**4. Configure `ocr.command` in `config.json`**

Open `%LOCALAPPDATA%\mark-shot\config.json` (create it if it does not exist) and set `ocr.command`:

```json
{
  "ocr": {
    "enabled": true,
    "backend": "rapidocr",
    "command": "\"%LOCALAPPDATA%\\mark-shot\\ocr-venv\\Scripts\\python.exe\" \"%LOCALAPPDATA%\\mark-shot\\mark-shot-ocr.py\" --format json --backend rapidocr {image}",
    "timeoutMs": 30000
  }
}
```

Replace `%LOCALAPPDATA%` with the actual expanded path (e.g. `C:\Users\YourName\AppData\Local`). The `{image}` placeholder is replaced with the temporary screenshot path at runtime; if omitted, Mark Shot appends it automatically.

> **Tip**: Set the environment variable `MARK_SHOT_OCR_NO_VENV=1` to skip the script's built-in venv auto-detection, since the venv Python is already invoked directly.

</details>

#### Code Scan Backend (Optional)

```bash
python3 -m venv ~/.local/share/mark-shot/code-scan-venv
~/.local/share/mark-shot/code-scan-venv/bin/pip install -U pip zxing-cpp pillow
```

The code scanner helper prefers `zxing-cpp` for QR Code, Data Matrix, Aztec, PDF417, EAN, UPC, Code 39, Code 93, Code 128, and other common formats. It can also fall back to `pyzbar` or OpenCV QR detection when those packages are available.

#### Image Upload Backend (optional)

The image upload feature uses the bundled `mark-shot-upload` Python script by default. It has no third-party dependencies (Python 3 standard library only) and is configured entirely through environment variables in `upload.env`. See the [Image Upload Configuration](#image-upload-configuration) section above for supported keys and provider examples.

For providers that return a plain-text URL instead of JSON (e.g. litterbox), set `upload.command` to a custom `curl` invocation—Mark Shot auto-detects any stdout line starting with `http://` or `https://` as the upload result.

#### Windows

Install Qt 6 for your compiler toolchain, CMake, Ninja, and a C++17 compiler such as MSVC or MinGW. The Windows build does not require Qt DBus, PipeWire, X11/XCB, LayerShellQt, `grim`, `wl-copy`, or `xclip`.

```powershell
cmake -S . -B build-windows -G Ninja -DCMAKE_BUILD_TYPE=Release `
  -DCMAKE_PREFIX_PATH=C:\Qt\6.7.3\msvc2019_64
cmake --build build-windows
```

Windows support currently targets normal screenshots and image annotation. Scrolling capture, compositor-specific window detection, and Linux desktop entries are not available on Windows. The bundled Python helper scripts (`mark-shot-ocr`, `mark-shot-code-scan`, `mark-shot-translate`) are not installed automatically—see the [OCR Backend](#ocr-backend-optional), [Code Scan Backend](#code-scan-backend-optional), and translation sections above for manual Windows setup instructions.

### Build Steps

```bash
# With system Qt 6
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# If Qt 6 is installed under the user directory, add CMAKE_PREFIX_PATH
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_PREFIX_PATH=$HOME/Qt/6.7.3/gcc_64

# Build
cmake --build build
```

Or build with Nix:

```bash
nix build
```

LayerShellQt is detected automatically. When found, full Wayland layer-shell support is enabled. When absent, the build succeeds and falls back to standard fullscreen windows at runtime.

### Installation

```bash
cmake --install build --prefix "$HOME/.local"
```

This installs the binary, helper scripts (`mark-shot-ocr`, `mark-shot-code-scan`, `mark-shot-translate`, `mark-shot-upload`), desktop entries, and icons.

### GNOME Wayland Scrolling Capture Extension

GNOME Wayland scrolling capture requires the **Mark Shot Scroll Helper** extension. Without it, Mark Shot cannot perform silent repeated area screenshots or display the GNOME-native scroll preview panel, causing the scrolling capture action to be disabled on GNOME Wayland.

The extension files are bundled in the project repository at `packaging/gnome-extension/mark-shot-scroll-helper@snemc.org`.

<details>
<summary><b>Expand/Collapse GNOME Wayland Scrolling Capture Extension Installation & Enable Guide</b></summary>

##### Method A: Installed from Distribution Package
If Mark Shot was installed via a distribution package, the extension is already installed system-wide. Enable it with:
```bash
gnome-extensions enable mark-shot-scroll-helper@snemc.org
```
*If not found, log out and log back in, then retry.*

##### Method B: Installed from Repository Source
To manually install and enable the extension directly from the repository source folder:
```bash
# Define the extension UUID
UUID=mark-shot-scroll-helper@snemc.org

# Create the user GNOME extensions directory
mkdir -p "$HOME/.local/share/gnome-shell/extensions"

# Copy the extension files from the repository
cp -r "packaging/gnome-extension/$UUID" "$HOME/.local/share/gnome-shell/extensions/"

# Enable the extension (you may need to restart GNOME Shell or log out and back in)
gnome-extensions enable "$UUID"
```

Verify that the helper D-Bus interface is available:

```bash
gdbus call --session \
  --dest org.gnome.Shell \
  --object-path /org/gnome/Shell/Extensions/MarkShotScrollHelper \
  --method org.gnome.Shell.Extensions.MarkShotScrollHelper.Version
```

The expected result is `('4.2',)`. On GNOME Wayland, restart `mark-shot` after enabling the extension.

</details>

---

## Shortcuts & Interactive Gestures

### Tool Switching

| Hotkey | Tool | Description |
| :---: | :--- | :--- |
| **V** | Move / Pan | Moves and pans the image canvas (in local file mode). |
| **S** | Select | Selects, moves, scales, or deletes existing vector annotations. |
| **P** | Pen | Draws smooth freehand curves. |
| **L** | Line | Draws straight lines. |
| **H** | Highlighter | Semi-transparent highlight strokes. |
| **R** | Rectangle | Draws rectangular bounding boxes. |
| **E** | Ellipse | Draws elliptical bounding boxes. |
| **A** | Arrow | Draws classic pointy-tailed arrows. |
| **T** | Text | Types rich text (supports 1000px size and dual-gesture scale). |
| **N** | Number | Stamps sequential auto-incrementing numbered markers. |
| **M** | Mosaic | Covers sensitive data with acrylic frost blur. |
| **G** | Laser | Places temporary laser markings that dissolve automatically over time. |

### Startup Overlay Tools

| Hotkey | Tool | Description |
| :---: | :--- | :--- |
| **C** | Color Picker | Samples a screenshot pixel before selecting a region. Use the mouse wheel to resize the loupe, left click to open a color panel with copyable HEX, RGB, HSL, HSV, and Qt formats. Right click or Esc returns to normal selection. |
| **R** | Ruler | Measures coordinates before selecting a region. Hover reads the current pixel, and left-drag draws a measured rectangle with pixel ticks, width, height, diagonal, and area. Right click or Esc returns to normal selection. |
| **Q** | Code Scanner | Enters QR code and barcode scan mode. Select a region to decode codes inside it; the result opens in a copyable window. Right click or Esc returns to normal selection. |
| **D** | Display Capture | Instantly captures all outputs, crops the snapshot by display, and shows thumbnails with hover actions for copy, edit, and save. |

### Global Actions

| Hotkey | Action |
| :---: | :--- |
| **Esc** | Closes the screenshot/annotation window. |
| **Ctrl + C** | Confirms pending text edits and copies selection to system clipboard. |
| **Ctrl + S** or **Enter** | Confirms pending text edits and saves selection to a file. |
| **Ctrl + P** | Pins the current selection as a floating sticker window. |
| **Ctrl + U** | Uploads the current screenshot to the configured image host; the returned URL is copied to the clipboard. |
| **Ctrl + Z** | Undoes the last annotation. |
| **Ctrl + Y** or **Ctrl + Shift + Z** | Redoes the last undone annotation. |
| **Backspace** or **Delete** | Deletes the selected annotation object (under Select tool). |
| **F** | Toggles the active capture scope between selection and full screen. |

### Advanced Interaction Tips

- **Constrain Drawing**: Hold `Ctrl` while drawing Rectangles or Ellipses to constrain them to perfect squares or circles.
- **Quick Select Tool**: Right-click once on the canvas to switch to the **Select** tool instantly.
- **Quick Color Switch**: Double right-click on the canvas to open the radial color palette and quickly switch the active annotation color.
- **Scroll Wheel Regulation**: While a drawing tool is active, scroll the mouse wheel to dynamically adjust stroke width, text size, auto-increment number scale, or mosaic block size.
- **Canvas Zoom & Pan**: Under **Select** tool (or in local image mode), scroll the mouse wheel to zoom the canvas, and hold the middle mouse button to pan. Double-tap `Ctrl` to reset zoom and pan.

### Pinned Window Actions

| Gesture / Shortcut | Description |
| :--- | :--- |
| **Hold Left Click & Drag** | Repositions the floating window on your desktop. |
| **Scroll Wheel Up / Down** | Scales the floating window size proportionally. |
| **Double Left Click** | Closes the pinned window immediately. |
| **Right Click** | Opens the context menu (Rotate, Zoom, Always on Top, Copy Image Text, Translate, Save, Copy, Close). |
| **Esc Key** | Closes the currently active pinned window. |

---

## Release Notes

### 0.1.32

- **Startup Shortcut Hint Panel**: Replaced the centered startup hint pill with a PixPin-style vertical shortcut panel that defaults to the left-bottom corner and moves to the left-top corner when the pointer approaches it.
- **Input Device Hints**: Added keyboard, mouse, and mouse-wheel glyphs to the startup shortcut panel so shortcut rows communicate the expected input method more clearly.
- **Window Z-Order Selection**: Improved window ordering across GNOME, KDE Plasma, Hyprland, X11, and Windows so region selection prefers the visually topmost matching window.
- **Wayland Fcitx5 Candidate Support**: Adjusted layer-shell cursor-rectangle handling so fcitx5 candidate windows appear correctly under Wayland capture overlays.
- **Settings Gear Icons**: Redrew the settings toolbar and General settings navigation icons as clearer gear glyphs instead of sun-like radial icons.
- **Tray Mode Compatibility**: Fixed startup behavior when Mark Shot is launched directly into tray mode on environments without an immediately available system tray.
- **Wayland Text Editor Width**: Prevented the annotation text editor from shrinking unexpectedly on fractional-scale Wayland displays.

<details>
<summary>Previous versions</summary>

### 0.1.31

- **CLI Image Pinning**: Added `--pin-image <path>` to open an existing local image directly as a pinned sticker window, skipping capture and selection.
- **Color Picker History**: The startup Color Picker now remembers recently picked colors, persisted in `config.json` under `colorPicker.history` (capped at 7 `#RRGGBBAA` entries) and shown as swatches in the color panel.
- **Interface Language Setting**: Added a configurable `ui.language` option (`system` / `english` / `chinese`) selectable from the General settings page; supersedes the legacy root-level `language` key.
- **Desktop-Aware Window Detection**: Mark Shot now auto-selects the matching window detection script at runtime (GNOME, KDE Plasma, Hyprland, Niri), falling back to the niri script on other Wayland sessions and native X11 detection on X11. Mismatched configured commands are corrected in memory without touching `config.json`.
- **GNOME Occluded Window Filtering**: The GNOME Shell scroll helper extension now filters fully occluded windows from detection results.
- **Prebuilt AUR Package**: Added a `mark-shot-bin` AUR package installing prebuilt pacman packages from GitHub Releases, alongside the source-based `mark-shot` package.
- **GNOME Adwaita Palette Fix**: Overrode the application palette at the `qApp` level so the dark palette fully replaces the libqtk3 base palette under GNOME Adwaita.
- **AUR Optional Dependencies**: Added `python-rapidocr`, `python-pillow`, and `python-zxing-cpp` as preferred OCR/code-scan optdepends.

### 0.1.30

- **Settings Configuration Dialog**: Added a dedicated settings window with pages for General, Capture, Annotation, Pinned, Scroll, Shortcuts, Storage, Integrations, and Advanced. Every previously file-only option is now editable in one place, backed by the same `config.json` store, with shared design tokens and a custom navigation sidebar.
- **Launch on Startup**: Added a `Launch on Startup` switch on the General page. Linux writes an XDG `autostart` desktop entry; Windows writes the current user's `Run` registry key. The switch disables itself on unsupported platforms.
- **Portal Global Shortcut Support**: Added an `xdg-desktop-portal` `GlobalShortcuts` backend so global capture hotkeys work on Wayland without X11.
- **Pinned Text Selection Toggle**: The pinned image window now exposes a configurable text-selection toggle, with `pinned_window_config` split into its own module.
- **Settings Entry During Capture**: Opening settings from the toolbar or shortcut now closes the frozen capture session first and defers the dialog to the next event-loop tick, avoiding conflicts with the layer-shell capture window.
- **Pinned Window Placement on Wayland**: Extracted layer-shell geometry computation and improved the resize controller, fixing off-screen and multi-monitor placement of pinned windows.

### 0.1.29

- **Independent Magnifier Frame Resize**: The magnifier annotation now exposes resize handles on both the inner source viewfinder and the outer lens. Rectangle lenses get 8 corner/edge handles per frame, circular lenses get 4. Resizing either frame keeps `magnifierScale` constant by scaling the other frame proportionally, so the loupe ratio stays consistent regardless of which side the user grabs.
- **Rectangle Highlight & Invert Styles**: The rectangle tool gains a style selector with three modes—`Stroke` (existing outlined / filled rectangle with optional rounded corners), `Highlight` (marker-pen overlay using `CompositionMode_Multiply` with semi-transparent fill), and `Invert` (inverts the RGB pixels covered by the rectangle while keeping the outline as a visual cue). Fill toggle and corner radius are hidden for `Highlight` and `Invert`.
- **Persistent Tool Defaults**: Annotation tool defaults (color, opacity, per-tool widths, rectangle fill / corner radius / style, magnifier scale and shape, arrow / highlighter / number style, text font, text background color) now survive across sessions through a dedicated `annotation-state.json` file. Writes are atomic via `QSaveFile` and triggered immediately after every default-changing entry point.

### 0.1.28

- **Configurable Clipboard Image Policy**: Added `clipboard.image.mode` with `image/png`, `url`, and `threshold` modes. The default now keeps direct `image/png` clipboard data for better compatibility with office suites and browser input fields, while `thresholdM` can still switch large images to file URL mode.
- **Default Runtime Config Creation**: Ensured runtime startup creates a default `config.json` when the file is missing, including the new clipboard defaults.
- **Shift-Constrained Line Drawing**: Holding `Shift` while drawing Line, Arrow, or straight Highlighter annotations now snaps the stroke to horizontal, vertical, or 45-degree directions.

### 0.1.27

- **Multi-point Line/Arrow Skeleton Editing**: Introduced support for adding, dragging, and deleting multiple skeleton (control) points on line and arrow annotations. Paths are smoothed using continuous quadratic Bezier curves, ensuring endpoints precisely target endpoints.
- **Shortcut & Interaction Improvements**: Enhanced keyboard and scroll interactions (e.g. using Backspace/Delete to remove selected skeleton points), and refactored input shortcut processing logic.

### 0.1.26

- **Custom Save Path & Placeholders**: Introduced flexible screenshot save templates (`save.pathTemplate` and `save.directoryTemplate`), supporting 30+ dynamic placeholders like `{pictures}`, `{datetime}`, and custom formatting like `{datetime:yyyy-MM-dd}` for versatile directory structures and naming schemes.
- **KDE KWin Screenshot Control Switch**: Added the `capture.wayland.kde.kwinScreenshot.enabled` option to enable or disable using KWin's restricted `org.kde.KWin.ScreenShot2` D-Bus interface, facilitating fallback debug routines.
- **Document Layout Optimization & Details Collapsing**: Refactored the user guide to collapse long KDE DBus setup details and application configuration parameters, improving overall readability.

</details>

---

## Feedback & Issues

If you encounter bugs or want to suggest new features, we recommend using GitHub CLI (`gh`) tool to submit an Issue. We provide templates and a script to automatically collect system information. For details, please refer to the [Issue Submission Guide](.doc/submit-issue-via-gh.md).

## License

This project is licensed under the **MIT License**. For details, please refer to the [LICENSE](LICENSE) file.

## Friendly Links
[linux.do](https://linux.do) Thanks to the promotion from the linux.do community

## Acknowledgements

Thanks to [serendipitywgy](https://github.com/serendipitywgy) for contributions from `serendipitywgy/mark-shot`, including cross-desktop compatibility improvements, the OCR copy toolbar action, and smart rectangle preselection.
