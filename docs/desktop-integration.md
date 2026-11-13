# Desktop integration

**English** | [简体中文](zh-CN/desktop-integration.md)

There are two ways to start a capture with a key: bind the `mark-shot` command in your compositor,
or let Mark Shot register a global shortcut itself. The first is the simplest on tiling
compositors; the second works without editing compositor configuration.

## Binding a key in the compositor

**niri** (`~/.config/niri/config.kdl`)

```kdl
binds {
    Print { spawn "mark-shot"; }
    Shift+Print { spawn "mark-shot" "--scroll"; }
}
```

**Sway** (`~/.config/sway/config`)

```text
bindsym Print exec mark-shot
bindsym Shift+Print exec mark-shot --scroll
```

**Hyprland** (`~/.config/hypr/hyprland.conf`)

```text
bind = , Print, exec, mark-shot
bind = SHIFT, Print, exec, mark-shot --scroll
```

On GNOME and KDE, add a custom shortcut in the system keyboard settings and enter `mark-shot` as
the command.

## Built-in global shortcut and tray icon

Both are off by default and are switched on in the settings window:

- **Settings → Shortcuts** enables the global capture shortcut. Click the key field and press the
  combination. The default is `PrintScreen`.
- **Settings → Appearance** enables the tray icon. Its menu offers capture, pin clipboard, screen
  recording, settings and quit.

```toml
[desktop]
tray_enabled = true
shortcuts_enabled = true
screenshot_shortcut = "Ctrl+Shift+S"
```

| Platform | Global shortcut | Tray |
| --- | --- | --- |
| Linux X11 | Registered directly; a conflict is reported as a failure | StatusNotifierItem |
| Linux Wayland | XDG GlobalShortcuts portal; the desktop may ask for confirmation and decides the final combination | StatusNotifierItem |
| Windows | Registered directly; a conflict is reported as a failure | Notification area |

The settings window shows whether registration succeeded. Notes:

- On Wayland, the shortcut requires a portal that implements GlobalShortcuts. Without one,
  registration fails and a compositor binding is the alternative.
- GNOME has no tray area by default. Install an extension that supports StatusNotifierItem, such
  as AppIndicator support.
- If the desktop already uses `PrintScreen`, pick a free combination.

## Background service

When the tray or the global shortcut is enabled, a small background service holds the
registration. It starts automatically with the settings window or the editor, and keeps running
after they close. Choosing **Quit** in the tray menu stops it; turning both switches off stops it
as well.

Mark Shot does not add itself to your login items. To have the tray and the shortcut available
right after login, start the service from your session:

```sh
mark-shot --background
```

For example, in niri:

```kdl
spawn-at-startup "mark-shot" "--background"
```

The command exits immediately when both features are disabled.

## Editor window type

On Wayland the editor can be a layer-shell overlay or a normal fullscreen window:

```toml
[general]
overlay = "auto"    # auto | window | layer
```

| Value | Behaviour |
| --- | --- |
| `window` | A fullscreen window that can be moved to another workspace or minimized |
| `layer` | Drawn above everything until the capture ends |
| `auto` | `layer` on niri and COSMIC, `window` elsewhere |

`mark-shot --xdg-window` forces a normal window for one run.

## Opening images

The desktop entry registers Mark Shot for PNG, JPEG and WebP files, so "Open with Mark Shot" in a
file manager opens the image in the editor. The same works from a terminal:

```sh
mark-shot picture.png
```

## Notifications

Mark Shot sends a desktop notification when a capture is saved or fails. Disable them in the
settings window, in `[notifications]`, or for one run with `--nonotify`.
