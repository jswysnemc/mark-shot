# Mark Shot

[中文说明](README.zh-CN.md)

`mark-shot` is a Qt 6 screenshot tool for Wayland compositors such as niri. It captures a frozen screen frame with `grim`, opens a fullscreen annotation surface, and lets you select, mark, save, copy, or open the edited image with another desktop application.

## Features

- Captures the current output by default, with `--all-outputs` for a full compositor capture.
- Opens an existing image file for annotation with `mark-shot path/to/image.png`.
- Supports region selection, fullscreen annotation, and selection resize or move.
- Provides pen, line, highlighter, rectangle, ellipse, arrow, text, number, and mosaic tools.
- Provides a laser pointer tool for temporary teaching marks that fade automatically.
- Supports undo, redo, save, copy to Wayland clipboard, and open-with actions.
- Uses a layer-shell overlay by default and can fall back to a regular fullscreen xdg window with `--xdg-window`.
- Designed for compositors that support `wlr-screencopy`, especially niri.

## Build

Arch Linux dependencies:

```bash
sudo pacman -S --needed base-devel cmake ninja qt6-base qt6-wayland layer-shell-qt grim wl-clipboard
```

Build and run:

```bash
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
./build/mark-shot
```

Install locally:

```bash
cmake --install build --prefix "$HOME/.local"
```

## Usage

Capture the current output:

```bash
mark-shot
```

Capture every output:

```bash
mark-shot --all-outputs
```

Annotate the full captured frame without selecting a region first:

```bash
mark-shot --fullscreen
```

Open an existing image file and annotate it directly:

```bash
mark-shot path/to/image.png
```

Use a regular fullscreen xdg window instead of layer-shell:

```bash
mark-shot --xdg-window
```

Example niri binding:

```kdl
Mod+Shift+S { spawn "mark-shot"; }
```

## Controls

- Drag to create a selection.
- `V`, `S`, `P`, `L`, `H`, `R`, `E`, `A`, `T`, `N`, `M`, and `G` switch to move, select, pen, line, highlighter, rectangle, ellipse, arrow, text, number, mosaic, and laser.
- `F` toggles the annotation scope between the current selection and the full captured frame.
- In full-frame annotation mode, the layout button toggles the toolbar between horizontal and vertical layouts.
- The move tool drags the selection or resizes it from edges and corners.
- The select tool picks existing annotations; drag the object or its handles to move or resize it.
- Drawing tools and selected annotations show a toolbar-attached property panel with width and opacity sliders, an alpha-capable Qt color picker, text background color, shape fill options, rectangle corner radius, and text font editing where applicable.
- Click the red `x` on a selected annotation to delete only that annotation.
- Hold `Ctrl` while drawing rectangles or ellipses to constrain them to squares or circles.
- Right click switches to the select tool, and double right click opens the radial color palette.
- Mouse wheel changes stroke width, number size, text size, or mosaic block size for the active tool or selected annotation.
- In image-file mode, mouse wheel zooms around the cursor, middle-button drag pans the image, and double-tapping `Ctrl` resets the view.
- `Ctrl+Z` undoes, and `Ctrl+Shift+Z` or `Ctrl+Y` redoes.
- `Ctrl+C` copies the edited selection.
- `Ctrl+S` or `Enter` saves the edited selection.
- `Esc` exits.

## License

MIT License.
