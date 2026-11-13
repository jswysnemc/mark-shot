# Usage

**English** | [简体中文](zh-CN/usage.md)

## Taking a screenshot

Run `mark-shot`. The screen freezes and a selection overlay appears.

1. **Select.** Drag to draw a region. Hovering highlights the window or interface region under
   the pointer; click to accept it, or turn the scroll wheel to switch between the enclosing and
   the nested region. A loupe next to the pointer shows the pixel colour and coordinates.
2. **Adjust.** With the selection tool (`S`), drag the edges or the inside of the selection. The
   arrow keys move it by one pixel and `Alt` with an arrow key resizes it; add `Shift` for ten
   pixels.
3. **Annotate.** Pick a tool from the toolbar. See [Annotation tools](annotation-tools.md).
4. **Finish.** Use one of the actions below.

`Esc` or a right click during selection cancels the capture.

### Finishing

| Action | Key | Result |
| --- | --- | --- |
| Accept | `Enter` | Runs the default finish action |
| Copy | `Ctrl+C`, or double-click inside the selection | Copies the image to the clipboard |
| Save | `Ctrl+S` | Saves to the screenshots folder |
| Save as | `Ctrl+Alt+S` | Opens a file dialog |
| Pin | `Ctrl+P` | Pins the image above other windows |
| Cancel | `Esc` | Discards the capture |

The **default finish action** is the `to` setting in `[general]`, a combination of letters:

| Letter | Action |
| --- | --- |
| `c` | Copy to the clipboard |
| `s` | Save to the screenshots folder |
| `p` | Pin to the screen |
| `o` | Write the PNG to standard output |

The default is `cs`: copy and save. These actions also run in addition to the one you choose in
the editor, so with `to = "s"` a copy also saves the file. `-t LETTERS` overrides the setting for
one run.

Saved files go to `~/Pictures/screenshots/<year-month>/<date_time>.png`. Change the folder and the
name pattern in the settings window or in `[save]`; `--output DIRECTORY` overrides the folder for
one run.

### Capture modes

| Command | Behaviour |
| --- | --- |
| `mark-shot` | Select a region, then edit |
| `mark-shot -m` | Freeze only the monitor under the pointer |
| `mark-shot --all-outputs` | Freeze the complete desktop |
| `mark-shot --display NAME` | Use a specific output; repeat the option for several |
| `mark-shot --fullscreen` | Open the editor with the whole capture selected |
| `mark-shot -f` | Capture immediately without the editor |
| `mark-shot -r` | Drag a region and finish as soon as the button is released |
| `mark-shot -w` | Capture the active window |
| `mark-shot -d 3` | Wait three seconds first; 0 to 60, fractions allowed |
| `mark-shot picture.png` | Open an image file in the editor |

Without a scope option, all monitors are frozen. Set `freeze_scope = "cursor-screen"` in
`[capture]` to freeze only the monitor under the pointer by default.

Active window capture works on X11, KDE Wayland, niri and Windows. On niri the finish action must
include copying, for example `mark-shot -w -t c`. `mark-shot --list-displays` prints the output names.

On Linux, launching Mark Shot while a capture is already open hands the new request to the
running instance instead of opening a second overlay.

### Scrolling screenshots

Run `mark-shot --scroll`, or press `Ctrl+Shift+S` after selecting a region. Scroll the content
inside the selection at a steady pace; Mark Shot captures frames continuously and stitches them
into one long image. Fixed headers and footers are kept once. If consecutive frames do not
overlap enough, a hint asks you to scroll back slowly.

## Pinned images

A pinned image is a small window that stays above other windows.

| Start from | How |
| --- | --- |
| The editor | `Ctrl+P` or the pin button |
| An image file | `mark-shot --pin-image FILE` |
| The clipboard | `mark-shot --pin-clipboard` pins an image or plain text |

| Gesture | Result |
| --- | --- |
| Left drag | Move |
| Drag an edge or corner | Resize, keeping the aspect ratio |
| Scroll wheel | Zoom around the pointer |
| Double-click | Close |
| Right click | Menu: rotate, zoom, reset size, always on top, edit, copy, recognize text, translate, scan codes, save as, close |
| `Ctrl+V` | Pin the clipboard content as another window |

Copying and saving always use the original pixels, regardless of the displayed size. After text
recognition, dragging over recognized text selects it and `Ctrl+C` copies the selection. The
border and the always-on-top default are in `[pinned]`.

## Capture history

Completed captures are recorded automatically. Open the history window with `mark-shot --history`
or `Ctrl+H` in the editor. It shows thumbnails and a preview, and offers copy, pin, save as,
edit again, delete and clear.

By default the 50 most recent captures are kept, up to 256 MiB in total; the oldest are removed
first. Cancelled captures are not recorded. Settings are in `[history]`, see
[Configuration](configuration.md#capture-history).

## QR codes and barcodes

```sh
mark-shot --scan               # select a screen region
mark-shot --scan image.png     # scan an image file
mark-shot --scan-json image.png
```

Results open in a window where the text can be selected and copied. `--scan-json` prints a JSON
array of `{ "format", "text" }` objects instead and opens no window; it prints `[]` when nothing is
found. The scan button is also in the editor toolbar and in the pinned image menu. Decoding is
local and needs no model download.

## Upload and external programs

- **Upload** sends the annotated PNG to an image host and copies the returned URL. There is no
  default host; configure one first, see [Configuration](configuration.md#upload).
- **Open with** lists installed desktop applications and passes the image to the chosen one.

Both buttons are in the action toolbar.

## Scripted capture

`--capture-to` captures without any interface and prints a compact JSON report:

```sh
mark-shot --capture-to ~/shot.png
mark-shot --capture-to ~/shots --output-name build-42
mark-shot --capture-to ~/shot.png --region 100,100,800,600
mark-shot --capture-to ~/shot.png --display DP-1 --include-cursor
```

`--region` takes global logical coordinates as `X,Y,WIDTH,HEIGHT`.

## Command-line reference

```text
mark-shot [OPTIONS] [IMAGE]
```

**Capture and editing**

| Option | Meaning |
| --- | --- |
| `--all-outputs` | Freeze the complete desktop |
| `-m`, `--monitor` | Freeze only the monitor under the pointer |
| `--fullscreen` | Open the editor with the complete capture selected |
| `-f`, `--full` | Capture immediately without the editor |
| `-w`, `--window` | Capture the active window |
| `-r`, `--select-region` | Drag a region and finish immediately |
| `--ocr` | Select a region for text recognition |
| `-S`, `--scroll` | Select a region and stitch a scrolling screenshot |
| `-d`, `--delay SECONDS` | Delay the capture |
| `--display NAME` | Select an output; repeatable |
| `--include-cursor` | Include the pointer where the backend supports it |
| `--default-tool TOOL` | Initial tool after selection |
| `--fullscreen-default-tool TOOL` | Initial tool for `--fullscreen` |
| `--file-default-tool TOOL` | Initial tool for image files |
| `--default-color COLOR` | `#RRGGBB` or `#RRGGBBAA` |
| `--xdg-window` | Use a normal window instead of the overlay layer |
| `-t`, `--to LETTERS` | Finish actions: `c`, `s`, `p`, `o` |
| `--output DIRECTORY` | Save into this directory |

Tool names: `selection`, `pick`, `pen`, `highlighter`, `laser`, `marker`, `ruler`, `magnifier`,
`line`, `arrow`, `rectangle`, `ellipse`, `text`, `number`, `numerated-arrow`, `redact`, `ocr`,
`eyedropper`.

**Scripted capture**

| Option | Meaning |
| --- | --- |
| `--capture-to FILE_OR_DIR` | Write a PNG and print a JSON report |
| `--region X,Y,WIDTH,HEIGHT` | Global logical region; requires `--capture-to` |
| `--output-name NAME` | Base name when writing into a directory |
| `--list-displays` | Print output information as JSON |

**Other windows**

| Option | Meaning |
| --- | --- |
| `--pin-image FILE` | Pin an image file |
| `--pin-clipboard` | Pin a clipboard image or plain text |
| `--scan`, `--scan IMAGE`, `--scan-json IMAGE` | Scan QR codes and barcodes |
| `--history` | Open the capture history |
| `--settings` | Open preferences |
| `--record`, `--record-select` | Open the recording controller, or select a recording region |
| `--recording-status`, `--pause-recording`, `--resume-recording`, `--stop-recording` | Control an active recording |

**Configuration and diagnostics**

| Option | Meaning |
| --- | --- |
| `--background` | Run the tray and global shortcut service until quit |
| `--config FILE` | Use another configuration file |
| `--print-default-config` | Print the complete default configuration |
| `--debug`, `--no-debug` | Enable or disable diagnostic logging |
| `--debug-log FILE` | Write diagnostics to a chosen file |
| `-s`, `--speed` | Print startup timings to standard error |
| `-n`, `--nonotify` | Suppress desktop notifications |
| `-h`, `--help`, `-v`, `--version` | Show help or the version |
