# Changelog

## 0.1.20 - 2026-06-05

### Features & Enhancements

- **Scroll Capture Idle Pause**: Added an idle pause mechanism (1000ms delay) during scrolling capture when preview panel space is constrained. It automatically pauses capture, reveals the progress preview, and changes the action label to "Continue Capture" so the user can easily review the progress.
- **Scroll Capture Config Support**: Added full configuration integration for scrolling screenshots in the setup window, allowing users to configure scrolling parameters directly via the UI.

### Bug Fixes

- **Physical Pixel Preservation**: Fixed a regression in cropping arithmetic to ensure raw physical pixels are correctly preserved without scaling distortion.
- **Scroll Capture UI Polish**: Refactored the scrolling preview window and GNOME shell helper extension to remove the manual 'hide' action, simplify the extension D-Bus event handlers, standardise button layouts, and prevent background outline rendering artifacts when preview panels are hidden.

### Release Artifacts

- `mark-shot-v0.1.20-linux-x86_64.tar.gz`
- `mark-shot-v0.1.20-linux-arm64.tar.gz`
- `mark-shot_0.1.20_amd64.deb`
- `mark-shot_0.1.20_arm64.deb`
- `mark-shot_0.1.20_fedora_x86_64.rpm`
- `mark-shot_0.1.20_fedora_aarch64.rpm`
- `mark-shot-v0.1.20-linux-x86_64.AppImage`
- `mark-shot-v0.1.20-linux-x86_64.flatpak`

## 0.1.19 - 2026-06-05

### Features & Enhancements

- **GNOME Wayland Scrolling Capture**: Added the bundled `mark-shot-scroll-helper@snemc.org` GNOME Shell extension, enabling GNOME Wayland scrolling screenshots through a private D-Bus helper for area capture and native scroll preview controls.
- **On-Demand Pinned OCR**: Changed pinned windows to run OCR on demand by default, controlled through `pinnedWindow.autoOcr` and `MARK_SHOT_PINNED_AUTO_OCR`.
- **Pinned Translation Prefetch**: Added `translation.autoAfterOcr` and `MARK_SHOT_TRANSLATION_AUTO_AFTER_OCR` to optionally prefetch translations after pinned-window OCR completes.
- **Context Menu Text Copy**: Improved "Copy Image Text" so it can trigger OCR automatically when no recognized text is cached yet, then copy the result after OCR finishes.

### Packaging

- **Debian Compatibility Baseline**: Reworked `.deb` release packaging to build on Debian 12 without the optional LayerShellQt plugin, avoiding newer Ubuntu `t64` and GCC runtime dependencies for Debian-derived systems.

### Release Artifacts

- `mark-shot-v0.1.19-linux-x86_64.tar.gz`
- `mark-shot-v0.1.19-linux-arm64.tar.gz`
- `mark-shot_0.1.19_amd64.deb`
- `mark-shot_0.1.19_arm64.deb`
- `mark-shot_0.1.19_fedora_x86_64.rpm`
- `mark-shot_0.1.19_fedora_aarch64.rpm`
- `mark-shot-v0.1.19-linux-x86_64.AppImage`

## 0.1.18 - 2026-06-04

### Features & Enhancements

- **Configurable Shortcuts**: Added full support for customizing tool hotkeys, global action hotkeys, and startup tool hotkeys through `shortcuts` or `hotkeys` configurations, allowing extensive configuration aliases (e.g. `annotation.shortcuts`).
- **Pinned Window Border**: Added border settings (`borderEnabled`, `borderColor`, `borderWidth`) for pinned sticker windows, customizable via booleans, nested config objects, or direct properties.
- **Color Picker User Experience**: Tweaked the color picker copy behavior to add a short UI exit delay (180ms) for smoother clipboard transition.

### Bug Fixes

- **OCR Dependency Diagnostics**: Improved OCR error detection to recognize missing Python dependencies (e.g. `rapidocr` or `tesseract` import errors) in stdout/stderr and display friendly missing-backend notifications.

### Release Artifacts

- `mark-shot-v0.1.18-linux-x86_64.tar.gz`
- `mark-shot-v0.1.18-linux-arm64.tar.gz`
- `mark-shot_0.1.18_amd64.deb`
- `mark-shot_0.1.18_arm64.deb`
- `mark-shot_0.1.18_fedora_x86_64.rpm`
- `mark-shot_0.1.18_fedora_aarch64.rpm`
- `mark-shot-v0.1.18-linux-x86_64-layershell.AppImage`
- `mark-shot-v0.1.18-linux-x86_64-nolayershell.AppImage`

## 0.1.17 - 2026-06-04

### Features & Enhancements

- **Quick Save Shortcut**: Mapped `Ctrl+S` to perform a direct quick save to the default directory (usually `~/Pictures`) without opening the file dialog, sending a desktop notification via D-Bus on completion. Changed the toolbar save button to "Save As".
- **Application Environment Configuration**: Supported configuring a top-level `env` (or `environment`) block in `config.json` to load variables like `QT_FONT_DPI` prior to `QApplication` creation, preventing environment settings from breaking screenshot geometry.
- **Improved Ruler Layout and Interaction**: Standardized layout parameters in the measurement ruler and improved its overlap detection algorithm to ensure the metadata box does not obscure the ruler metrics or cursor indicators.
- **Scroll Live Follow**: Automatically re-enable live-follow mode when new content is appended during scrolling capture, even if the user has manually panned the viewport.

### Bug Fixes

- **Linux Capture Geometry Stabilization**: Relocated and consolidated screenshot crop arithmetic to standalone module `capture_geometry` with dedicated unit tests. Resolved scaling and multi-monitor layout coordinate rounding bugs on Wayland, mapping image-space selections back to logical geometry accurately.
- **NixOS OCR Directory Access**: Redirected RapidOCR model caching to a writable directory under XDG data home (defaulting to `~/.local/share/mark-shot/models`) with optional override via `MARK_SHOT_OCR_MODEL_DIR`, avoiding crashes on read-only environments like NixOS.
- **Missing Header Include**: Fixed a compilation error by explicitly including the `<QClipboard>` header.

### Release Artifacts

- `mark-shot-v0.1.17-linux-x86_64.tar.gz`
- `mark-shot-v0.1.17-linux-arm64.tar.gz`
- `mark-shot_0.1.17_amd64.deb`
- `mark-shot_0.1.17_arm64.deb`
- `mark-shot_0.1.17_fedora_x86_64.rpm`
- `mark-shot_0.1.17_fedora_aarch64.rpm`
- `mark-shot-v0.1.17-linux-x86_64-layershell.AppImage`
- `mark-shot-v0.1.17-linux-x86_64-nolayershell.AppImage`

## 0.1.16 - 2026-06-04

### Features & Enhancements

- **Startup Overlay Tools**: Added a Color Picker (hotkey `C`, supports loupe resizing via scroll wheel and copying HEX/RGB/HSL/HSV/Qt formats) and a Ruler (hotkey `R`, measures coordinates, area, diagonal, and size) available before selecting a capture region.
- **Multi-Screen Capture Sessions**: Reconfigured capture logic to fully support simultaneous screen capture and multi-window linkage across multiple displays.
- **Configurable Default Tools & Color**: Supported defining initial annotation tools (`defaultTool`, `fullscreenDefaultTool`) and `defaultColor` in the application configuration, overridable via CLI flags.
- **Enhanced Niri Window Detection**: Allowed configuring `env` (or `environment`) blocks in the window detection config to pass variables like offsets (`MARK_SHOT_NIRI_OFFSET_*`) and panel edges to the detection script, resolving alignment bounds and filtering tiny windows.

### Release Artifacts

- `mark-shot-v0.1.16-linux-x86_64.tar.gz`
- `mark-shot-v0.1.16-linux-arm64.tar.gz`
- `mark-shot_0.1.16_amd64.deb`
- `mark-shot_0.1.16_arm64.deb`
- `mark-shot_0.1.16_fedora_x86_64.rpm`
- `mark-shot_0.1.16_fedora_aarch64.rpm`
- `mark-shot-v0.1.16-linux-x86_64-layershell.AppImage`
- `mark-shot-v0.1.16-linux-x86_64-nolayershell.AppImage`

## 0.1.15 - 2026-06-03

### Features & Enhancements

- **Flexible Scrolling Area Adjustment**: Supported dragging edges from the direction controls to dynamically resize the scrolling capture region on the fly.
- **Interactive Overview Navigation**: Replaced the bottom scrollbar in the preview panel with direct viewport dragging on the mini-map, offering a cleaner and more intuitive navigation experience.
- **Seamless Live-Follow Scrolling**: Added mouse-wheel navigation within the preview panel, which automatically snaps back to tracking live capture updates once you scroll back to the active edge.
- **Configurable Window Borders Detection**: Introduced an external script execution mechanism to auto-detect window boundaries on Wayland. Included a default helper for `niri` window manager and supported custom detection scripts for other compositors (e.g., Hyprland, Sway).
- **Dual-Mode Desktop Builds**: Added dedicated compilation flags and released dual-variant binaries (supporting native Wayland Layer Shell layer and standard XDG window shells separately) to ensure compatibility across diverse desktop environments.
- **AppImage Formats Support**: Provided portable x86_64 AppImage packages for both `layershell` and `nolayershell` variants, simplifying deployment on modern Linux distributions.

### Bug Fixes

- **Persistent Clipboard Storage**: Resolved clipboard data loss issues after application exit. Images are now kept reliably in the system clipboard via background integration with Wayland (`wl-copy`) and X11 (`xclip`).

### Release Artifacts

- `mark-shot-v0.1.15-linux-x86_64.tar.gz`
- `mark-shot-v0.1.15-linux-arm64.tar.gz`
- `mark-shot_0.1.15_amd64.deb`
- `mark-shot_0.1.15_arm64.deb`
- `mark-shot-v0.1.15-linux-x86_64-layershell.AppImage`
- `mark-shot-v0.1.15-linux-x86_64-nolayershell.AppImage`

## 0.1.14 - 2026-06-02

### Highlights

- Improved portal screencast negotiation with optional libportal support before falling back to direct D-Bus portal calls.
- Normalized portal screenshot and PipeWire screencast crops so scrolling capture uses compositor logical geometry consistently.
- Added first-frame settle timing and no-layer-shell panel spacing to reduce accidental overlay capture in desktop environments that use regular XDG windows.
- Synced the application version with the CMake project version so `mark-shot --version` tracks release metadata.

### Scrolling Screenshot Compatibility

Scrolling screenshot support outside `niri` remains a test feature. KDE, GNOME, X11, and other non-`niri` environments are not complete targets yet because portal behavior, compositor timing, window geometry feedback, and scroll event handling vary across desktop stacks.

If scrolling capture fails, run `DEBUG=1 mark-shot`, reproduce the issue, and attach `/tmp/mark-shot-scroll.log` to a GitHub issue. Set `MARK_SHOT_DEBUG_LOG=/path/to/log` to write the debug log elsewhere.

### Release Artifacts

- `mark-shot-v0.1.14-linux-x86_64.tar.gz`
- `mark-shot-v0.1.14-linux-arm64.tar.gz`
- `mark-shot_0.1.14_amd64.deb`
- `mark-shot_0.1.14_arm64.deb`

## 0.1.13 - 2026-06-01

### Fixes

- Fixed scrolling screenshot selection geometry on scaled `niri` outputs by mapping image pixel coordinates back to compositor logical coordinates before starting the scroll capture session.

### Release Artifacts

- `mark-shot-v0.1.13-linux-x86_64.tar.gz`
- `mark-shot-v0.1.13-linux-arm64.tar.gz`
- `mark-shot_0.1.13_amd64.deb`
- `mark-shot_0.1.13_arm64.deb`

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
