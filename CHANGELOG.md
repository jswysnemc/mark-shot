# Changelog

## 0.1.12 - 2026-06-01

### Highlights

- Added native scrolling screenshot capture for Wayland sessions using PipeWire screencast, a guided scrolling overlay, and image stitching.
- Reworked the annotation property panel with compact icon controls and clearer editing actions.
- Added selectable arrow styles, including the classic fletched style and a KDE/Spectacle-like open arrow style.
- Improved screenshot capture behavior on GNOME and other portal-based desktops by avoiding duplicate host portal app registration.
- Added Linux `arm64` release tarballs and Ubuntu/Debian `arm64` `.deb` packages alongside existing `x86_64` and `amd64` artifacts.
- Improved compatibility with older PipeWire SPA headers used by some distributions.

### Scrolling Screenshot Compatibility

Scrolling screenshot support is experimental. The implementation relies on PipeWire screencast frames, desktop portal behavior, compositor timing, and window geometry heuristics. It is currently tuned for `niri` and similar Wayland setups.

GNOME and KDE may fail to provide the required capture behavior or may return frames that cannot be stitched reliably. Fully adapting this feature to GNOME Shell, KWin, and different portal backends is difficult because each stack exposes different capture permissions, frame timing, window positioning, and scrolling behavior.

If scrolling capture does not work on GNOME or KDE, use the normal screenshot flow or an external long-screenshot tool through Mark Shot extension commands.

### Release Artifacts

- `mark-shot-v0.1.12-linux-x86_64.tar.gz`
- `mark-shot-v0.1.12-linux-arm64.tar.gz`
- `mark-shot_0.1.12_amd64.deb`
- `mark-shot_0.1.12_arm64.deb`
