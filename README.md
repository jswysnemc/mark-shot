# Mark Shot

[中文说明](README.zh-CN.md)

<video src="https://github.com/user-attachments/assets/c2298867-06b4-404d-87bc-62ab8d81088b" width="100%" controls></video>

`mark-shot` is a high-performance screenshot and annotation tool built with Qt 6. It was originally designed for Wayland compositors such as `niri`, and it also supports standard screenshot and annotation workflows on X11/GNOME desktops.

It captures screen frames instantly and opens an interactive fullscreen overlay, providing region cropping, rich annotation, clipboard copying, saving, and desktop pinning features.

---

## Features

### Advanced Annotation Toolset
- **Pen & Highlighter**: Smooth freehand drawing and semi-transparent overlay highlighting.
- **Geometric Shapes**: High-precision Line, Rectangle, and Ellipse paths.
- **Refined Arrow**: Sharp 6-vertex acute arrow path rendering with anti-aliasing.
- **Dual-Gesture Text**:
  - Supports font sizes up to `1000px`, adjustable via scroll wheel or property sliders.
  - Implements a physical width buffer to prevent unexpected wrapping across extreme scales.
  - **Diagonal handles** scale font size and boundary box proportionally; **side borders** only adjust wrap width.
- **Laser Pointer**: Dedicated presentation tool with pen traces that dissolve smoothly over time.
- **Auto-Increment Marker**: Click to stamp sequential numbered markers.
- **Mosaic**: Applies high-fidelity acrylic frost blur to obscure sensitive information.

### Pinned Window Stickers
- Pins any cropped region or annotated screenshot as an independent, frameless, and top-level floating window.
- Supports direct selection of OCR-recognized text in the pinned window, with `Ctrl + C` and context-menu copying.
- Supports OpenAI-compatible LLM translation for OCR text, rendering translated text back onto the image at the original layout positions.
- **Interactive Gestures**:
  - Drag with left click to reposition.
  - Scroll mouse wheel to scale, or double-tap `Ctrl` to reset to original aspect ratio.
  - Double left-click or press `Esc` to close.
  - Right-click to open a context menu with options to rotate, copy image text, translate, save, copy, or adjust opacity (0.2 to 1.0).

### Scrolling Screenshot Capture
- Captures a long scrolling region by combining PipeWire screencast frames with an interactive scrolling overlay and stitcher.
- Designed primarily for `niri` and similar Wayland environments where output geometry and capture timing can be controlled predictably.
- **Compatibility notice**: scrolling capture on KDE, GNOME, X11, and other non-`niri` environments is a test feature and is not complete yet. Portal backends, shell policies, window geometry behavior, frame timing, and scroll event handling differ substantially across these desktop stacks.
- If scrolling capture fails, use normal screenshots or configure an external long-screenshot command through Mark Shot extension commands.
- To report a scrolling capture issue, run `DEBUG=1 mark-shot`, reproduce the failure, then attach `/tmp/mark-shot-scroll.log` to a GitHub issue. Set `MARK_SHOT_DEBUG_LOG=/path/to/log` if the log should be written elsewhere.

### Cross-Platform Display Server Support
- **Wayland**: Uses PipeWire portal screencast for experimental scrolling capture, `grim` for wlroots screenshot capture, `layer-shell-qt` for native overlay, and `wl-copy` for clipboard persistence.
- **X11**: Uses `QScreen::grabWindow` for screen capture, fullscreen top-level window for overlay, and `xclip` for clipboard persistence.
- Runtime auto-detection via `$XDG_SESSION_TYPE` — no configuration needed.

### Desktop Integration
- **Desktop Entries**:
  - `mark-shot.desktop`: Configures the utility system-wide, triggerable by custom shortcuts.
  - `mark-shot-edit.desktop`: Registers as an image editor, enabling users to right-click local files in file managers (Dolphin, Nautilus, etc.) and open them directly in annotation mode.
- Ships with scalable vector icons (`mark-shot.svg` and `mark-shot-edit.svg`).

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

# Start with Move after region selection, Laser in fullscreen, and a teal default color
mark-shot --default-tool move --fullscreen-default-tool laser --default-color '#2DD4BF'

# Open and annotate an existing local image file
mark-shot path/to/image.png

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
| `--default-tool <tool>` | Sets the annotation tool selected after region selection. Also seeds fullscreen mode unless `--fullscreen-default-tool` is set. |
| `--fullscreen-default-tool <tool>` | Sets the annotation tool selected in fullscreen annotation mode. |
| `--default-color <color>` | Sets the default annotation color. Supports `#RRGGBB` and `#RRGGBBAA`. |

### Compositor / Desktop Hotkey Integration

To bind `mark-shot` to a system screenshot shortcut, configure your compositor or desktop environment.

**niri** (`~/.config/niri/config.kdl`):
```kdl
binds {
    Mod+Shift+S { spawn "mark-shot"; }
}
```

**GNOME** (via custom keyboard shortcut in Settings → Keyboard → Custom Shortcuts).

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

`command` is executed through `$SHELL -c`, so shell features work. Use `{slurp}` to pass the current selection as `x,y widthxheight` geometry. Use `{image}` or `{imagePath}` to pass the current rendered selection as a temporary PNG path, or `{imageUrl}` for a `file://` URL. These placeholders are shell-quoted automatically. Set `saveImage` or `needsImage` to `true` to append the temporary PNG path when no image placeholder is present. `workingDirectory` and `cwd` are aliases. `closeOnStart` defaults to `true`, hiding and closing Mark Shot before the command starts.

### Application Config

Mark Shot reads application settings from `~/.config/mark-shot/config.json`. Pinned windows use the OCR and translation settings in the same file. The default OCR helper prefers `rapidocr` and can fall back to `tesseract`; the translation helper calls an OpenAI-compatible `/chat/completions` endpoint.

```json
{
  "annotation": {
    "defaultTool": "move",
    "fullscreenDefaultTool": "laser",
    "defaultColor": "#2DD4BF"
  },
  "windowDetection": {
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

`annotation.defaultTool` sets the tool selected after region selection. `annotation.fullscreenDefaultTool` sets the tool selected in fullscreen annotation mode, including `--fullscreen` and image-file annotation. Supported values are `move`, `select`, `pen`, `line`, `highlighter`, `rectangle`, `ellipse`, `arrow`, `text`, `number`, `mosaic`, and `laser`. `--default-tool <tool>` overrides the normal default and, for compatibility, also seeds fullscreen mode unless `--fullscreen-default-tool <tool>` is set.

Fullscreen annotation has no separate capture selection to move. If its default tool is configured as `move`, Mark Shot starts fullscreen mode with `select` instead.

`annotation.defaultColor` sets the initial annotation color. Use `#RRGGBB` for opaque colors or `#RRGGBBAA` to include alpha. The runtime option `--default-color <color>` overrides this config value.

`windowDetection.env` (alias: `environment`) is passed to the detection script as environment variables.
- **Niri Script** (`mark-shot-window-detection-niri`): Supports `MARK_SHOT_NIRI_PANEL_EDGE` (`top`, `bottom`, `left`, `right`, or `none`) and pixel adjustments through `MARK_SHOT_NIRI_OFFSET_X/Y/WIDTH/HEIGHT`.
- **Hyprland Script** (`mark-shot-window-detection-hyprland`): Supports `MARK_SHOT_HYPRLAND_INCLUDE_INACTIVE` (`1` to detect windows on inactive workspaces, defaults to `0` for active only) and pixel offsets via `MARK_SHOT_HYPRLAND_OFFSET_X/Y/WIDTH/HEIGHT` to calibrate window borders.

### Pre-Capture Window Detection & Script Contribution Guide

To ensure precise window boundary detection across different Wayland compositors, Mark Shot uses a flexible external script invocation mechanism. Users can configure a detection script via `windowDetection.command`. The script is responsible for querying window geometries from the compositor and outputting the data in a unified format for Mark Shot to consume.

The project bundles default window detection scripts for the following window managers:
- **Niri**: `mark-shot-window-detection-niri`
- **Hyprland**: `mark-shot-window-detection-hyprland`

We highly welcome and encourage community members to contribute adapter scripts for various desktop environments and Wayland compositors to expand compatibility.

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

#### 3. How to Contribute Adapters

Currently, the repository only ships an adapter script for the niri window manager: `mark-shot-window-detection-niri`.

If you run Mark Shot on Hyprland, Sway, KDE (KWin Wayland), or GNOME (Mutter Wayland) and have configured a working script, please submit a Pull Request to share it with the community. Here are implementation guidelines for different environments:
- **Hyprland**: Use `hyprctl clients -j` and parse the output JSON.
- **Sway**: Use `swaymsg -t get_tree` to fetch the layout tree.
- **KDE / KWin**: Implement a simple KWin Script, or query KWin's D-Bus interfaces.
- **GNOME**: Since GNOME Wayland lacks an official client-side shell command to query window geometry, you typically need to write a GNOME Shell extension that exports Mutter's window geometry over D-Bus, then query that D-Bus interface in your script.

If the script fails to execute or times out (default: `1000ms`), Mark Shot will proceed with screenshot capture normally and fall back to its internal X11-based window detector where applicable.

When installing manually, install `mark-shot`, `mark-shot-ocr`, and `mark-shot-translate` together. Otherwise the pinned window opens, but image-text copying and translation cannot call the backend helpers.

---

## Compilation & Installation

### Official Release Artifacts

Each release publishes Linux binary archives and Debian packages:

- `linux-x86_64.tar.gz` and `linux-arm64.tar.gz`
- `amd64.deb` and `arm64.deb`

The Debian package installs `mark-shot`, helper scripts, desktop entries, icons, and runtime metadata together.

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

```bash
python3 -m venv ~/.local/share/mark-shot/ocr-venv
~/.local/share/mark-shot/ocr-venv/bin/pip install -U pip rapidocr onnxruntime
```

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

LayerShellQt is detected automatically. When found, full Wayland layer-shell support is enabled. When absent, the build succeeds and falls back to standard fullscreen windows at runtime.

### Installation

```bash
cmake --install build --prefix "$HOME/.local"
```

This installs the binary, helper scripts (`mark-shot-ocr`, `mark-shot-translate`), desktop entries, and icons.

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

### Global Actions

| Hotkey | Action |
| :---: | :--- |
| **Esc** | Closes the screenshot/annotation window. |
| **Ctrl + C** | Confirms pending text edits and copies selection to system clipboard. |
| **Ctrl + S** or **Enter** | Confirms pending text edits and saves selection to a file. |
| **Ctrl + P** | Pins the current selection as a floating sticker window. |
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
| **Double Tap Ctrl Key** | Resets the window size to original aspect ratio. |
| **Right Click** | Opens the context menu (Rotate, Opacity, Copy Image Text, Translate, Save, Copy, Close). |
| **Esc Key** | Closes the currently active pinned window. |

---

## Release Notes

### 0.1.16

- **Startup Overlay Tools**: Added a Color Picker (hotkey `C`, supports loupe resizing via scroll wheel and copying HEX/RGB/HSL/HSV/Qt formats) and a Ruler (hotkey `R`, measures coordinates, area, diagonal, and size) available before selecting a capture region.
- **Multi-Screen Capture Sessions**: Reconfigured capture logic to fully support simultaneous screen capture and multi-window linkage across multiple displays.
- **Configurable Default Tools & Color**: Supported defining initial annotation tools (`defaultTool`, `fullscreenDefaultTool`) and `defaultColor` in the application configuration, overridable via CLI flags.
- **Enhanced Niri Window Detection**: Allowed configuring `env` (or `environment`) blocks in the window detection config to pass variables like offsets (`MARK_SHOT_NIRI_OFFSET_*`) and panel edges to the detection script, resolving alignment bounds and filtering tiny windows.

### 0.1.15

- **Flexible Scrolling Area Adjustment**: Supported dragging edges from the direction controls to dynamically resize the scrolling capture region on the fly.
- **Interactive Overview Navigation**: Replaced the bottom scrollbar in the preview panel with direct viewport dragging on the mini-map for cleaner navigation.
- **Seamless Live-Follow Scrolling**: Added mouse-wheel navigation within the preview panel, which automatically snaps back to tracking live capture updates once you scroll back to the active edge.
- **Configurable Window Borders Detection**: Supported detecting window boundaries on Wayland using external scripts, including a default script for `niri`.
- **Dual-Mode Desktop Builds**: Added compilation options and released dual-variant AppImage packages (native Layer Shell and regular XDG window variants) to ensure cross-desktop compatibility.
- **Persistent Clipboard Storage**: Resolved clipboard data loss issues after application exit, keeping images reliably in the system clipboard.

### 0.1.14

- **Optimized Scrolling Capture Stability**: Refined portal negotiation and logical coordinate handling to improve stitching accuracy.
- **Updated Scroll Compatibility Guide**: Clarified experimental support status and logging tools for KDE, GNOME, and X11 environments.
- **Accurate Release Versioning**: Linked CLI metadata to CMake compilation configurations to ensure `mark-shot --version` reports correctly.

### 0.1.13

- Fixed scrolling screenshot selection geometry on scaled `niri` outputs.

### 0.1.12

- Added experimental native scrolling screenshot capture for Wayland.
- Reworked the annotation property panel into a compact icon-based layout.
- Added alternate arrow styles, including a KDE/Spectacle-like arrow.
- Improved GNOME portal behavior by avoiding duplicate host portal app registration.
- Added Linux ARM64 tarball and Ubuntu/Debian ARM64 `.deb` release artifacts.
- Improved PipeWire SPA header compatibility for older distributions.

Scrolling screenshot capture is not guaranteed on GNOME or KDE. The feature depends on portal capture behavior, compositor timing, window geometry, and scroll event handling, so robust GNOME/KDE support has a high adaptation cost.

---

## License

This project is licensed under the **MIT License**. For details, please refer to the [LICENSE](LICENSE) file.
## Friendly Links
[linux.do](https://linux.do) Thanks to the promotion from the linux.do community

## Acknowledgements

Thanks to [serendipitywgy](https://github.com/serendipitywgy) for contributions from `serendipitywgy/mark-shot`, including cross-desktop compatibility improvements, the OCR copy toolbar action, and smart rectangle preselection.
