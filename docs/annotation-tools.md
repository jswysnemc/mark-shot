# Annotation tools

**English** | [简体中文](zh-CN/annotation-tools.md)

After a region is selected, the main toolbar holds the drawing tools and the action toolbar holds
finish actions. A property bar next to the toolbar shows the settings of the current tool. Both
toolbars can be rearranged in **Settings → Toolbar**.

The keys below are the defaults; all of them can be changed, see [Shortcuts](shortcuts.md).

## Tools

| Tool | Key | Purpose |
| --- | --- | --- |
| Selection | `S` | Move and resize the captured region |
| Pick | `V` | Select, move, resize, rotate and delete annotations |
| Pen | `P` | Freehand stroke |
| Highlighter | `H` | Translucent freehand stroke |
| Line | `D` | Straight or curved line; solid, dashed or dotted |
| Arrow | `A` | Arrow with four head styles and three shaft styles |
| Rectangle | `R` | Outline or filled rectangle with optional rounded corners |
| Ellipse | `C` | Outline or filled ellipse |
| Marker | `K` | One of 24 preset shapes |
| Text | `T` | Text with font, size, bold, italic and background |
| Number | `N` | Auto-incrementing numbered badge |
| Numbered arrow | `Shift+N` | Arrow with a number, counted separately |
| Redact | `M`, `X` | Pixelate, blur, grayscale, invert or brighten a region |
| Magnifier | `Z` | Persistent loupe that enlarges part of the image |
| Ruler | `U` | Measure length or angle |
| Laser | `G` | Temporary stroke for presentations |
| Eyedropper | `I` | Pick a colour from the image |
| OCR | `O` | Recognize text in the selection |

## Tool details

**Line and arrow.** Hold `Shift` to snap the direction to 45 degree steps. With the pick tool,
double-click the path to insert a control point and drag it to bend the line; `Delete` removes the
selected control point. Arrow heads are filled or open, single or double ended.

**Rectangle and ellipse.** Hold `Shift` while dragging for a square or a circle. The `1:1` switch
in the property bar makes that permanent, and `R` sets the corner radius of rectangles in image
pixels.

**Text.** Click to place the caret and type. `Enter` starts a new line; `Esc`, `Ctrl+Enter` or a
click outside commits the text. The font panel offers search, a point size from 8 to 300, bold and
italic. Input methods are supported on Wayland and X11. To change an existing text, select it with
the pick tool and use the edit button in the property bar.

**Number.** Click for a round badge, or drag to add a pointer tail. Seven styles are available:
digits, upper and lower case letters, upper and lower case Roman numerals, Heavenly Stems and
Chinese numerals. Deleting a badge does not renumber the others. The reset button restarts the
counter.

**Redact.** The effect is computed from the original capture, never from other annotations. The
strength ranges from 4 to 48 and sets the block size or the blur radius in image pixels.

**Magnifier.** Drag to create a source frame and a lens. Both frames can be moved separately;
resizing one resizes the other so the zoom factor stays fixed. The lens is round or rectangular
and the zoom ranges from 1.25 to 6.

**Ruler.** Shows width, height, diagonal and area, or an angle, in image pixels. Press `U` again
to switch between length and angle for the next measurement.

**Laser.** Strokes fade out about two seconds after the button is released. They are never part
of the saved image and are not recorded in the undo history.

## Editing annotations

Press `V` and click an annotation to select it. A dashed frame with handles appears.

| Action | How |
| --- | --- |
| Move | Drag the annotation |
| Resize | Drag a handle; hold `Ctrl` to keep the aspect ratio |
| Rotate | Drag the rotation handle; hold `Shift` to snap to 45 degree steps |
| Delete | Click the red cross on the frame, or press `Delete` |
| Select several | Drag a box around them, or press `Ctrl+A` for all |
| Change properties | Use the property bar; the change applies to the selection |
| Undo, redo | `Ctrl+Z`, `Ctrl+Shift+Z` or `Ctrl+Y` |

A right click cancels the annotation being drawn and switches to the pick tool. A double right
click opens the colour panel.

## Size, colour and view

- `[` and `]` decrease and increase the stroke width or font size. With a drawing tool the scroll
  wheel does the same and shows the value next to the pointer.
- The colour panel has a palette, recent colours, a hex field and an opacity slider.
- With the pick tool, or when editing an image file, the scroll wheel zooms the canvas around the
  pointer. Drag with the middle button to pan, and press `Ctrl` twice quickly to reset the view.
- `Space` hides and shows the toolbars.

The last used colour, width and style of every tool are remembered between sessions.

## Export frame

The export frame adds transparent padding, rounded corners and a soft shadow around the image when
it is saved or copied from the editor. It is off by default. Enable it in
**Settings → Pins & frames**, where a live preview shows the result, or in
`[export.image_frame]`:

```toml
[export.image_frame]
enabled = true
padding = 112
corner_radius = 18.0
shadow_radius = 72
shadow_offset_y = 28
shadow_opacity = 0.32
```

Pinned images and the on-screen preview keep the original image without the frame.
