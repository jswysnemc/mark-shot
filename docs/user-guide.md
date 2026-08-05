# Mark Shot User Guide

This manual covers day-to-day operation of Mark Shot, with a focus on the
**window / component hover selection** feature (moving the mouse automatically
tracks and highlights the window under the cursor; a click selects it), the
annotation workflow, headless capture, and configuration.

> The docs in this repository are authored in the community fork and mirrored
> to the upstream and enterprise repositories. The enterprise edition adds an
> extra section for its local MCP server.

---

## 1. Quick Start

### 1.1 Launching

Start a region-capture session:

```bash
mark-shot
```

Press a desktop hotkey (see § 8) or run from a terminal. A frozen full-screen
overlay opens on the focused display. Move the mouse to draw a selection
rectangle, then release to enter the annotation editor.

### 1.2 Portable builds

If you use a portable bundle (`mark-shot-upstream`, `mark-shot-community`,
`mark-shot-enterprise`), launch it with the bundled launcher so that the
bundled Qt libraries, plugins, and helper scripts are found:

```bash
portable/mark-shot-community/bin/run-mark-shot.sh
```

The launcher prepends its `bin/` directory to `PATH`, which is required for the
window-detection helper scripts (`mark-shot-window-detection-*`) and OCR /
upload helpers.

---

## 2. Window / Component Hover Selection

Mark Shot can detect the windows of the current desktop before you pick a
region. While the selection overlay is open, **moving the mouse highlights the
window under the cursor** with a teal frame. **A plain left click (no drag)
selects that whole window** as the capture region; you can then annotate, copy,
pin, or save it directly.

The highlighted windows come from a per-compositor detection script that runs
before the overlay appears:

| Desktop | Detection source | Notes |
| :--- | :--- | :--- |
| GNOME Wayland | bundled `mark-shot-scroll-helper@snemc.org` Shell extension over D-Bus | requires the extension to be enabled (see § 2.1) |
| KDE Plasma Wayland | one-shot KWin scripting via `qdbus6` / `qdbus` + journalctl | requires a KWin session |
| Hyprland | `hyprctl -j clients` | |
| niri | `niri msg -j windows` + config parsing | |
| X11 | in-process XCB enumeration of `_NET_CLIENT_LIST_STACKING` | no script needed |
| Windows | in-process `EnumWindows` | no script needed |

Only **top-level windows** are tracked. Individual widgets inside a window
("components") are not exposed by Wayland compositors, so hover selection
targets whole windows on every platform.

### 2.1 GNOME Wayland: enable the helper extension

```bash
gnome-extensions enable mark-shot-scroll-helper@snemc.org
```

Verify the D-Bus helper answers:

```bash
gdbus call --session \
  --dest org.gnome.Shell \
  --object-path /org/gnome/Shell/Extensions/MarkShotScrollHelper \
  --method org.gnome.Shell.Extensions.MarkShotScrollHelper.Version
# -> ('5',)
```

If the call fails, log out and back in (or restart GNOME Shell on X11) and
retry. Without the extension the GNOME helper script exits with an error and
hover selection stays off (normal drag-selection still works).

### 2.2 How to use it

1. Trigger a capture (`mark-shot` or the desktop hotkey).
2. Without pressing any mouse button, move the cursor over a window. A teal
   frame outlines the window that would be selected.
3. **Click once** (press and release without moving more than a few pixels) to
   select that window. If windows overlap, the topmost window at the cursor
   wins (z-order aware).
4. Release enters the annotation editor with the window exactly framed.
5. To make a **manual** region instead, simply drag a rectangle as usual — the
   hover frame is ignored as soon as the drag exceeds the click threshold.

The hover highlight is disabled while the Color Picker (`C`) or Ruler (`R`)
startup tool is active, and remains available for Code Scanner (`Q`), display
capture (`D`), and GIF / Video recording startup modes.

### 2.3 Selecting windows on the correct monitor

Window detection runs per capture target. On a multi-monitor setup, each
frozen window receives only the windows intersecting its own geometry, so the
hover frame matches what you see on that display.

### 2.4 Enabling / disabling

The feature is enabled by default (`windowDetection.enabled = true`). Toggle it
in **Settings → Advanced → Window Detection Enabled**, or edit
`~/.config/mark-shot/config.json`:

```json
{
  "windowDetection": {
    "enabled": true,
    "command": "mark-shot-window-detection-gnome",
    "timeoutMs": 1000,
    "env": {}
  }
}
```

- `command`: the detection script. On GNOME / KDE / Hyprland / niri Wayland the
  bundled `mark-shot-window-detection-*` script matching your session is chosen
  automatically; on X11 and Windows the platform is enumerated in-process and
  `command` may be left empty. **A user-supplied custom command (for example an
  absolute path) is always respected.**
- `timeoutMs`: maximum wait for the script (100–30000 ms, default 1000).
- `env`: extra environment variables passed to the script. Per-compositor
  tweaks (offsets) are documented in the script headers.

### 2.5 Troubleshooting

| Symptom | Check |
| :--- | :--- |
| No teal frame on GNOME Wayland | extension enabled? `gdbus` call above must return a version |
| No teal frame on X11 / Windows | none — platform enumeration is built in; make sure the capture session is not using a startup pointer tool |
| Hover frame picks the wrong (underneath) window | z-order data missing from a custom detection script; windows without `zOrder` are ranked as the bottom layer |
| Capture starts slowly | the detection script runs before the overlay; raise `timeoutMs` only if the desktop is slow, or set `enabled:false` to skip it |
| See diagnostics | run `mark-shot --debug --debug-log /tmp/mark-shot.log`; look for `window-detection` lines |

---

## 3. Region Selection & Startup Tools

Before the region is committed you can use the startup overlay tools:

| Hotkey | Tool | Behaviour |
| :---: | :--- | :--- |
| `C` | Color Picker | Sample a pixel; wheel resizes the loupe; left click opens a color panel (HEX / RGB / HSL / HSV / Qt formats); right click or `Esc` exits |
| `R` | Ruler | Hover reads pixel coordinates; left-drag measures a rectangle with width, height, diagonal and area; right click or `Esc` exits |
| `Q` | Code Scanner | Drag a region around a QR / barcode; the decoded result opens in a copyable window |
| `D` | Display Capture | Captures all outputs, crops per display, shows hoverable thumbnails (copy / edit / save) |
| `S` | Stop active GIF / video recording | stops the recording shown in the overlay |

`Esc` cancels the session; right click (no startup tool) also cancels.

---

## 4. Annotation Tools

After a region is selected (or a local image is opened) the editor opens with
the annotation toolbar. Tools are switched with the number keys or the toolbar:

| Hotkey | Tool | Description |
| :---: | :--- | :--- |
| `V` | Move / Pan | move the whole selection, pan a local image canvas |
| `S` | Select | select, move, scale, rotate, delete existing annotations |
| `P` | Pen | smooth freehand strokes |
| `L` | Line | straight lines |
| `H` | Highlighter | semi-transparent marker; freehand or straight line style |
| `R` | Rectangle | box with `Stroke` / `Highlight` / `Invert` styles, rounded corners |
| `E` | Ellipse | ellipse / circle |
| `A` | Arrow | classic arrows (fletched, KDE, bidirectional) |
| `T` | Text | rich text; wheel or sliders resize; diagonal handles scale both, side handles adjust wrap; exact pt size, font family, bold / italic in the font panel |
| `N` | Number | sequential numbered markers (Arabic, alpha, roman, Chinese, …) |
| `M` | Mosaic | acrylic frost blur to hide sensitive content |
| `G` | Laser | temporary strokes that dissolve automatically |

Drawing tips:

- Hold `Ctrl` while drawing a rectangle / ellipse to constrain to a square /
  circle.
- Scroll the wheel while a tool is active to adjust stroke width, text size,
  number scale, or mosaic block size (live preview).
- Under `Select`, scroll to zoom the canvas and hold the middle button to pan;
  double-tap `Ctrl` to reset.

### 4.1 Editing an existing annotation

Switch to **Select** (`S`). Click an annotation to show its handles:

- drag inside to move;
- drag corner / edge handles to resize;
- drag the round handle above the top edge to rotate;
- press `Delete` / `Backspace` to remove;
- double-click text to edit it in place.

The property panel (right side) edits the selected annotation: color, width,
style, text font family / size / bold / italic. Multiple annotations can be
selected by dragging a selection box under the `Select` tool; the group can
then be moved, resized, rotated and deleted together.

### 4.2 Actions

| Shortcut | Action |
| :--- | :--- |
| `Ctrl+C` | copy to clipboard |
| `Ctrl+S` / `Enter` | save (path template from settings) |
| `Ctrl+P` | pin as a floating sticker window |
| `Ctrl+U` | upload to the configured image host; URL is copied |
| `Ctrl+Z` / `Ctrl+Y` | undo / redo |
| `F` | toggle the capture scope (selection ↔ full screen) |

### 4.3 Export frame

Enable **Settings → Export → Mac-style frame** to add transparent padding,
rounded corners, and a soft shadow to saved / copied / uploaded images.

---

## 5. Pinned Window Stickers

| Gesture / Shortcut | Behaviour |
| :--- | :--- |
| left-drag | reposition the sticker |
| wheel | scale proportionally |
| double left click / `Esc` | close |
| right click | context menu (rotate, zoom, always-on-top, copy text, translate, save, copy, close) |

OCR text inside a pinned window is selectable and copyable (`Ctrl+C` /
context menu). Translation (OpenAI-compatible endpoint) renders the translated
text back onto the image at the original layout positions.

---

## 6. Scrolling Screenshot

1. Select a region (or use the floating drag handle for very large regions).
2. The overlay scrolls the target window; captured frames are stitched into a
   long image.
3. GNOME Wayland requires the Mark Shot Scroll Helper extension (§ 2.1).

Scrolling capture is production-ready on niri and similar wlroots/Wayland
compositors; on KDE, X11 and other stacks it is a test feature. If it fails,
use normal screenshots or a custom extension command.

---

## 7. Headless Capture (CLI)

Non-interactive capture writes a PNG and prints JSON:

```bash
# primary screen
mark-shot --capture-to /tmp/shot.png

# directory (timestamped file name)
mark-shot --capture-to /tmp/shots/

# region
mark-shot --capture-to /tmp/r.png --region 0,0,1280,720

# a specific display, with cursor
mark-shot --capture-to /tmp/w.png --display DP-1 --include-cursor

# several displays at once (one PNG each)
mark-shot --capture-to /tmp/shots/ --display DP-1 --display DP-2

# list outputs
mark-shot --list-displays
```

All headless options are mutually exclusive with a positional image file.
See the README for the full argument table.

---

## 8. Desktop Hotkeys & Tray

Tray mode (`mark-shot --tray`) registers `Ctrl+Alt+S` for region capture and
provides capture / recording / settings / quit menu entries. Desktop hotkeys:

- **GNOME**: Settings → Keyboard → Shortcuts → Custom Shortcuts → bind to `mark-shot`.
- **KDE**: custom shortcut bound to `mark-shot` (plus the KWin ScreenShot2
  permission for exact KDE capture, see README).
- **Hyprland**: `bind = SUPER SHIFT, S, exec, mark-shot` and `bind = , Print, exec, mark-shot`.
- **niri**: `binds { Mod+Shift+S { spawn "mark-shot"; } }`.
- **Sway / i3**: `bindsym Mod4+Shift+S exec mark-shot`.

---

## 9. Configuration & Backends

- Config file: `~/.config/mark-shot/config.json` (Linux), created on first run.
- Full reference: [Configuration](configuration.md).
- Backends: Wayland (PipeWire portal / grim / wlroots screencopy), X11
  (`QScreen::grabWindow`), Windows (native WGC). Recording prefers the
  PipeWire portal and falls back automatically.

Optional helpers:

```bash
# OCR (RapidOCR / Tesseract)
python3 -m venv ~/.local/share/mark-shot/ocr-venv
~/.local/share/mark-shot/ocr-venv/bin/pip install -U pip rapidocr onnxruntime

# Code scan (zxing-cpp)
python3 -m venv ~/.local/share/mark-shot/code-scan-venv
~/.local/share/mark-shot/code-scan-venv/bin/pip install -U pip zxing-cpp pillow
```

---

## 10. Feature Testing Checklist

Use this to verify a build end-to-end:

1. **Launch** — `run-mark-shot.sh` opens the frozen overlay.
2. **Window hover** — move the mouse over a window: teal frame follows; single
   click selects the window; overlapping windows pick the topmost one.
3. **Manual region** — drag a rectangle; release; editor opens.
4. **Annotate** — draw with each tool (Pen, Line, Rectangle, Ellipse, Arrow,
   Highlighter, Text, Number, Mosaic, Magnifier, Laser); undo/redo; Select to
   move/resize/rotate/delete; double-click a text to edit.
5. **Copy / Save / Pin / Upload** — `Ctrl+C`, `Ctrl+S`, `Ctrl+P`, `Ctrl+U`.
6. **Startup tools** — `C` color picker, `R` ruler, `Q` code scan, `D` display
   capture.
7. **Headless** — `--capture-to`, `--region`, `--display`, `--list-displays`.
8. **Tray + hotkey** — `mark-shot --tray`, press `Ctrl+Alt+S`.
9. **Portable specifics** — bundle finds its own Qt libs/plugins/scripts.

---

## 11. Feedback

Report issues with `gh issue create` using the bundled
[issue submission guide](../../.doc/submit-issue-via-gh.md). Attach a debug log
captured with `mark-shot --debug --debug-log /tmp/mark-shot.log`.
