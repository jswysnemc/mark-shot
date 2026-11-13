# Shortcuts

**English** | [简体中文](zh-CN/shortcuts.md)

These are the default bindings inside the editor. The global shortcut that starts a capture is
separate; see [Desktop integration](desktop-integration.md).

## Finishing and general actions

| Action | Default | Configuration key |
| --- | --- | --- |
| Accept with the default finish action | `Return` | `accept` |
| Copy | `Ctrl+C` | `copy` |
| Save | `Ctrl+S` | `save` |
| Save as | `Ctrl+Alt+S` | `save_as` |
| Pin | `Ctrl+P` | `pin` |
| Cancel | `Escape` | `cancel` |
| Undo | `Ctrl+Z` | `undo` |
| Redo | `Ctrl+Shift+Z`, `Ctrl+Y` | `redo` |
| Select all annotations | `Ctrl+A` | `select_all` |
| Delete | `Delete`, `Backspace` | `delete` |
| Scrolling screenshot | `Ctrl+Shift+S` | `scroll_capture` |
| Capture history | `Ctrl+H` | `history` |
| Settings | `Ctrl+,` | `settings` |
| Switch light and dark theme | `Ctrl+Shift+L` | `toggle_theme` |
| Hide or show the toolbars | `Space` | `toggle_ui` |
| Toggle the pointer loupe | `Shift+M` | `toggle_magnifier` |
| Increase size | `]` | `size_up` |
| Decrease size | `[` | `size_down` |

## Recording

| Action | Default | Configuration key |
| --- | --- | --- |
| Record video | `Ctrl+Shift+R` | `record_video` |
| Record GIF | `Ctrl+Shift+G` | `record_gif` |
| Pause or resume | `Ctrl+Shift+P` | `record_pause` |
| Stop | `Ctrl+Shift+E` | `record_stop` |

## Tools

| Tool | Default | Configuration key |
| --- | --- | --- |
| Selection | `S` | `tool_selection` |
| Pick | `V` | `tool_pick` |
| Pen | `P` | `tool_pen` |
| Highlighter | `H` | `tool_highlighter` |
| Line | `D` | `tool_line` |
| Arrow | `A` | `tool_arrow` |
| Rectangle | `R` | `tool_rectangle` |
| Ellipse | `C` | `tool_circle` |
| Marker | `K` | `tool_marker` |
| Text | `T` | `tool_text` |
| Number | `N` | `tool_number` |
| Numbered arrow | `Shift+N` | `tool_numerated_arrow` |
| Redact | `M`, `X` | `tool_redact` |
| Magnifier | `Z` | `tool_magnifier` |
| Ruler | `U` | `tool_ruler` |
| Laser | `G` | `tool_laser` |
| Eyedropper | `I` | `tool_eyedropper` |
| OCR | `O` | `tool_ocr` |

## Moving and resizing the selection

| Action | Default | Configuration key |
| --- | --- | --- |
| Move by 1 pixel | Arrow keys | `move_left`, `move_right`, `move_up`, `move_down` |
| Move by 10 pixels | `Shift` + arrow keys | `move_*_fast` |
| Resize by 1 pixel | `Alt` + arrow keys | `resize_left`, `resize_right`, `resize_up`, `resize_down` |
| Resize by 10 pixels | `Alt+Shift` + arrow keys | `resize_*_fast` |

The step sizes are `nudge` and `nudge_fast` in `[selection]`.

## Actions without a default key

These actions are available from the toolbar and can be given a key:
`open_with`, `extensions`, `upload`, `translate`, `scan_code`, `clear_annotations`,
`toggle_scope`, `toggle_toolbar_layout`.

## Modifiers while drawing

| Modifier | Effect |
| --- | --- |
| `Shift` | Square or circle for rectangle and ellipse; 45 degree steps for lines, arrows and rotation |
| `Ctrl` | Keep the aspect ratio while resizing an annotation |
| `Ctrl` pressed twice | Reset canvas zoom and position |

## Changing bindings

Open **Settings → Shortcuts**. Every action can be recorded, extended with an additional key,
cleared or restored, and conflicts are reported.

Bindings can also be edited in the `[keys]` table of the configuration file. A value is one key
or a list of keys; an empty list removes the binding:

```toml
[keys]
copy = "Ctrl+C"
redo = ["Ctrl+Shift+Z", "Ctrl+Y"]
upload = "F8"
toggle_ui = []
```

Removing a tool from the toolbar does not disable its key.
