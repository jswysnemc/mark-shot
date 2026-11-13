# Platform support

**English** | [简体中文](zh-CN/platform-support.md)

Mark Shot chooses its backends from the protocols the running desktop actually offers, not from
the name of the distribution or desktop. Capturing pixels, listing windows and keeping a window on
top are separate capabilities, so one can work where another does not.

## Overview

| Environment | Screenshot | Editor window | Clipboard |
| --- | --- | --- | --- |
| niri, Sway and other wlroots compositors | ext-image-copy-capture, wlr-screencopy, portal as a fallback | layer-shell or xdg-shell | ext or wlr data-control |
| KDE Plasma (Wayland) | KWin ScreenShot2, then the protocols above | layer-shell or xdg-shell | data-control, Xwayland as a fallback |
| GNOME (Wayland) | ScreenCast portal with PipeWire | xdg-shell | Xwayland selection bridge |
| X11 | XGetImage, RandR, XFixes for the pointer | Native X11 window | CLIPBOARD and PRIMARY |
| Windows | GDI | Native window | System clipboard: PNG, DIBV5, Unicode text |

On Linux the session type decides the protocol. A Wayland session uses Wayland even when
`DISPLAY` is set, because Xwayland cannot see the Wayland desktop. `MARK_SHOT_BACKEND` and
`MARK_SHOT_CAPTURE` override the choice, see
[Configuration](configuration.md#environment-variables).

## Feature matrix

| Feature | wlroots, niri | KDE Wayland | GNOME Wayland | X11 | Windows |
| --- | --- | --- | --- | --- | --- |
| Region and full-screen capture | Yes | Yes | Yes, through the portal | Yes | Yes |
| Multi-monitor capture | Yes | Yes | Yes | Yes | Yes |
| Active window capture (`-w`) | niri only, requires copying | Yes | No | Yes | Yes |
| Window snapping while selecting | niri and Sway | No | No | Yes | Yes |
| Interface region detection | Yes | Yes | Yes | Yes | Yes |
| Pinned images always on top | With layer-shell | With layer-shell | No | Yes | Yes |
| Global shortcut | Portal | Portal | Portal | Yes | Yes |
| Tray icon | Yes | Yes | Needs an extension | Yes | Yes |
| Screen recording | Yes | Yes | Through the portal | Yes | Yes |

Text input methods are supported through text-input-v3 on Wayland and XIM on X11.

"Interface region detection" finds rectangles in the frozen image. It works everywhere, but it
does not know what a control is. "Window snapping" uses real window geometry from the desktop and
needs a desktop-specific source.

## Notes per environment

### niri, Sway and wlroots compositors

- Window geometry comes from the niri or Sway IPC. Other compositors have no window source yet and
  rely on image-based detection and manual selection.
- On niri the active window capture goes through the compositor's own screenshot action, which
  writes to the clipboard. The finish action must therefore include copy: `mark-shot -w -t c`.
- The editor uses a layer-shell overlay on niri by default, see
  [Desktop integration](desktop-integration.md#editor-window-type).

### KDE Plasma

- Capture uses KWin's ScreenShot2 interface. Set `kde_kwin_screenshot = false` in `[capture]` to
  use the desktop portal instead.
- A complete interactive regression run has not been done on KDE Wayland yet.

### GNOME

- The first capture opens the portal dialog to choose a monitor. The permission is remembered
  after a successful capture.
- GNOME does not offer layer-shell or data-control. Mark Shot uses a normal window and copies
  through Xwayland, so Xwayland must be available for clipboard support.
- There is no public interface for the active window or for window geometry, so `-w` and window
  snapping report that they are unsupported.
- The tray needs an extension that provides StatusNotifierItem.

### X11

- Captures use physical pixels and are not resampled per monitor DPI.
- The active window capture contains the visible client area of the foreground window.
- Always-on-top pins use `_NET_WM_STATE_ABOVE`; the window manager decides whether to honour it.
- Monitor hotplug during a capture and mixed-DPI hardware have not been verified. KDE on X11 and
  Xfce have not had desktop-specific regression runs.

### Windows

- Capture, editing, the clipboard, saving, pinning, OCR and recording are implemented with native
  Windows interfaces.
- Window snapping uses the window list, DWM frame bounds and UI Automation.
- Verification so far relies on automated runs under Wine and on cross-compilation checks.
  Behaviour on real Windows hardware, mixed-DPI monitor setups and system input methods has not
  been fully verified. Treat the Windows build as experimental.

## Reporting a platform problem

Include the following in a bug report:

```sh
mark-shot --version
echo "$XDG_SESSION_TYPE $XDG_CURRENT_DESKTOP"
mark-shot --list-displays
mark-shot --debug-log /tmp/mark-shot.log     # reproduce the problem, then attach the file
```
