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
- **GIF and Video Recording**: Press the configured startup recording shortcuts or use the tray menu to record a selected display or a custom region as GIF or MP4. Active recordings show tray and frozen-frame status, can be stopped with `S`, the overlay button, the tray menu, or `--stop-recording`, and send desktop notifications when recording starts or saves. On Wayland, recording prefers the PipeWire portal backend and can fall back to wlroots screencopy or polling capture when portal capture is unavailable.
- **Image Host Upload**: Press `Ctrl+U` or click the toolbar upload button after selecting a region to upload the screenshot to a custom image host (ImgURL, sm.ms, imgbb, litterbox, etc.). The returned URL is automatically copied to the clipboard. Configure the host via `upload.env`, or plug in any custom uploader via `upload.command`.
- **Mac-style Export Frame**: Adds transparent padding, rounded corners, and a soft shadow to saved, copied, uploaded, Open With, and extension-command images.

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
- **Wayland**: Uses PipeWire portal screencast for recording and experimental scrolling capture, including shared-memory and DMA-BUF frame paths, `grim` for wlroots screenshot capture, `layer-shell-qt` for native overlay, and `wl-copy` for clipboard persistence.
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
| `--recording-status` | Prints the current recording status as JSON through the running instance. |
| `--stop-recording` | Requests the running instance to stop the active recording. |
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

The tray menu also provides Capture, Fullscreen Capture, Start Recording, live recording status, Stop Recording, Settings, and Quit actions.

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

### Application Configuration

See [Configuration Reference](docs/configuration.md).

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

See [Release Notes](docs/releases.md).

## Feedback & Issues

If you encounter bugs or want to suggest new features, we recommend using GitHub CLI (`gh`) tool to submit an Issue. We provide templates and a script to automatically collect system information. For details, please refer to the [Issue Submission Guide](.doc/submit-issue-via-gh.md).

## License

This project is licensed under the **MIT License**. For details, please refer to the [LICENSE](LICENSE) file.

## Friendly Links
[linux.do](https://linux.do) Thanks to the promotion from the linux.do community

## Acknowledgements

Thanks to [serendipitywgy](https://github.com/serendipitywgy) for contributions from `serendipitywgy/mark-shot`, including cross-desktop compatibility improvements, the OCR copy toolbar action, and smart rectangle preselection.
