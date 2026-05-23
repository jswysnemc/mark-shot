# Mark Shot

[中文说明](README.zh-CN.md)

<video src="https://github.com/user-attachments/assets/c2298867-06b4-404d-87bc-62ab8d81088b" width="100%" controls></video>

`mark-shot` is a high-performance Wayland screenshot and annotation tool built with Qt 6, deeply optimized for modern Wayland compositors such as `niri`.

It captures lossless screen frames instantly using `grim` and opens an interactive fullscreen overlay, providing region cropping, rich annotation, clipboard copying, saving, and desktop pinning features.

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
- Pins any cropped region or annotated screenshot as an independent, frameless, and top-level floating window (`PinnedImageWindow`).
- **Interactive Gestures**:
  - Drag with left click to reposition.
  - Scroll mouse wheel to scale, or double-tap `Ctrl` to reset to original aspect ratio.
  - Double left-click or press `Esc` to close.
  - Right-click to open a context menu with options to rotate, save, copy, or adjust opacity (0.2 to 1.0).
- **Accidental Click Filter**: Integrates a `LeftClickMenuFilter` that blocks non-left-click triggers within the popup menu area to avoid accidental triggers in Qt 6 environments.

### Wayland & Desktop Integration
- **Wayland Native**: Utilizes `layer-shell-qt` for native overlay layouts; falls back to standard XDG fullscreen windows via `--xdg-window`.
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

### Compositor Hotkey Integration

To bind `mark-shot` to a system screenshot shortcut, configure your compositor config. 

For example, in `niri` (`~/.config/niri/config.kdl`):
```kdl
binds {
    Mod+Shift+S { spawn "mark-shot"; }
}
```

### Extension Commands

The right-side action toolbar includes an **Extensions** button. It reads user-defined commands from `~/.config/mark-shot/extensions.json`. The file can be either a JSON array or an object with a `commands` array.

```json
{
  "commands": [
    {
      "name": "Long screenshot",
      "command": "./target/release/wayscrollshot {slurp}",
      "workingDirectory": "~/Desktop/projecies/wayscrollshot",
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

`command` is executed through `$SHELL -c`, so shell features work. Use `{slurp}` to pass the current selection as `x,y widthxheight` geometry. Use `{image}` or `{imagePath}` to pass the current rendered selection as a temporary PNG path, or `{imageUrl}` for a `file://` URL. These placeholders are shell-quoted automatically, so write them without extra quotes. Set `saveImage` or `needsImage` to `true` to append the temporary PNG path when no image placeholder is present. `workingDirectory` and `cwd` are aliases. `closeOnStart` defaults to `true`, hiding and closing Mark Shot before the command starts.

---

## Compilation & Installation

### Dependencies (Arch Linux)

Install the necessary dependencies before building:

```bash
sudo pacman -S --needed base-devel cmake ninja qt6-base qt6-wayland layer-shell-qt grim wl-clipboard
```

### Build Steps

Build the binary using CMake and Ninja:

```bash
# Configure the project
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# Build the executable
cmake --build build
```

The compiled binary will be located at `./build/mark-shot`.

### Installation

Install the application and register desktop launchers with system mime associations:

```bash
cmake --install build --prefix "$HOME/.local"
```

---

## Shortcuts & Interactive Gestures

### Tool Switching

| Hotkey | Tool | Description |
| :---: | :--- | :--- |
| **V** | Move / Pan | Moves and pans the image canvas (in local file mode). |
| **S** | Select | Selects, moves, scales, or deletes existing vector annotations. Right-click selected area to pin. |
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

### Global Actions

| Hotkey | Action |
| :---: | :--- |
| **Esc** | Closes the screenshot/annotation window. |
| **Ctrl + C** | Confirms pending text edits and copies selection to Wayland system clipboard. |
| **Ctrl + S** or **Enter** | Confirms pending text edits and saves selection to a file. |
| **Ctrl + Z** | Undoes the last annotation. |
| **Ctrl + Y** or **Ctrl + Shift + Z** | Redoes the last undone annotation. |
| **Backspace** or **Delete** | Deletes the selected annotation object (under Select tool). |
| **F** | Toggles the active capture scope between selection and full screen. |

### Advanced Interaction Tips

- **Constrain Drawing**: Hold `Ctrl` while drawing Rectangles or Ellipses to constrain them to perfect squares or circles.
- **Quick Select Tool**: Right-click once on the canvas to switch to the **Select** tool instantly.
- **Radial Color Palette**: Double right-click on the canvas to open the radial color picker overlay.
- **Scroll Wheel Regulation**: While a drawing tool is active, scroll the mouse wheel to dynamically adjust stroke width, text size, auto-increment number scale, or mosaic block size.
- **Canvas Zoom & Pan**: Under **Select** tool (or in local image mode), scroll the mouse wheel to zoom the canvas, and hold the middle mouse button to pan. Double-tap `Ctrl` to reset zoom and pan.

### Pinned Window Actions

| Gesture / Shortcut | Description |
| :--- | :--- |
| **Hold Left Click & Drag** | Repositions the floating window on your desktop. |
| **Scroll Wheel Up / Down** | Scales the floating window size proportionally. |
| **Double Left Click** | Closes the pinned window immediately. |
| **Double Tap Ctrl Key** | Resets the window size to original aspect ratio. |
| **Right Click** | Opens the context menu (Rotate, Opacity, Save, Copy, Close). |
| **Esc Key** | Closes the currently active pinned window. |

---

## License

This project is licensed under the **MIT License**. For details, please refer to the [LICENSE](LICENSE) file.
