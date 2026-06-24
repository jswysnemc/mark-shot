# Changelog

## 0.1.31 - 2026-06-24

### Features & Enhancements

- **CLI Image Pinning**: Added a `--pin-image <path>` CLI option that opens an existing local image directly as a pinned sticker window, skipping the capture and selection flow entirely.
- **Color Picker History**: The startup Color Picker now remembers recently picked colors. History is persisted in `config.json` under `colorPicker.history` as `#RRGGBBAA` strings, capped at 7 entries, and rendered as swatches in the color panel.
- **Interface Language Setting**: Added a configurable interface language option (`ui.language`) with `system`, `english`, and `chinese` modes, selectable from the General settings page. Supersedes the legacy root-level `language` key.
- **Desktop-Aware Window Detection**: Mark Shot now detects the current desktop environment at runtime and auto-selects the matching window detection script (GNOME, KDE Plasma, Hyprland, Niri). Other Wayland sessions fall back to the niri script, and X11 sessions use native X11 detection. Mismatched configured commands are corrected in memory without modifying the config file.
- **GNOME Occluded Window Filtering**: The GNOME Shell scroll helper extension now filters fully occluded windows from detection results.
- **Prebuilt AUR Package**: Added a `mark-shot-bin` AUR package that installs prebuilt pacman packages downloaded from GitHub Releases, alongside the existing source-based `mark-shot` package.

### Bug Fixes

- **GNOME Adwaita Palette**: Overrode the application palette at the `qApp` level so the dark palette fully replaces the libqgtk3 base palette under GNOME Adwaita, fixing widget- and class-level `setPalette()` being merged away.
- **Native Window Detection Fallback**: Corrected the native window detection fallback path.
- **Color History Swatch Rendering**: Fixed rendering of color history swatches in the startup color dialog.
- **AUR Optional Dependencies**: Added `python-rapidocr`, `python-pillow`, and `python-zxing-cpp` as preferred OCR/code-scan optdepends, and removed `tesseract-data-chi_sim` so users can choose their own tesseract language data.

## 0.1.30 - 2026-06-20

### Features & Enhancements

- **Settings Configuration Dialog**: Introduced a dedicated settings dialog that consolidates every previously file-only option into a single window. Pages cover General, Capture, Annotation, Pinned, Scroll, Shortcuts, Storage, Integrations, and Advanced, each reading from and writing to the same `config.json`-backed store. The dialog ships with extracted design tokens and a custom navigation sidebar with hand-drawn vector icons, so the look stays consistent across pages and screen sizes.
- **Launch on Startup**: Added cross-platform autostart support behind a new `Launch on Startup` switch on the General settings page. On Linux it writes an XDG `autostart/mark-shot.desktop` entry that starts Mark Shot with `--tray`; on Windows it writes the current user's `Run` registry key. The switch is disabled automatically on platforms where autostart is unavailable, and applying settings syncs the system entry through the new `autostart` module (`src/autostart/`) atomically with `config.json` updates.
- **Portal Global Shortcut Support**: Added an `xdg-desktop-portal` based `GlobalShortcuts` backend (`src/global_shortcut_portal.cpp`) so global capture hotkeys work on Wayland compositors without X11. The tray controller now wires the portal backend alongside the existing register path, broadening hotkey compatibility across desktops.
- **Pinned Text Selection Toggle**: The pinned image window now exposes a configurable text-selection toggle. `pinned_window_config` was split out of `shot_window_config` into its own module, letting users control whether OCR text in pinned windows is selectable, with a matching unit test (`tests/pinned_window_config_test.cpp`).

### Bug Fixes

- **Settings Entry During Capture**: Opening settings from the annotation toolbar or the settings shortcut now closes the frozen capture session first and defers the dialog to the next event-loop tick via `openSettingsAfterClosingCapture`. This prevents the settings dialog from fighting the layer-shell capture window and avoids leaving frozen frames on screen.
- **Navigation Gear Icon**: Redrew the General navigation icon with an alternating-radius 8-tooth outline and a centered bore, replacing the radial-spoke look that read as a sun rather than a gear.
- **Pinned Window Placement on Wayland**: Extracted layer-shell geometry computation into `pinned_layer_shell_geometry` and taught the resize controller about Wayland-specific constraints, fixing off-screen and multi-monitor placement of pinned windows. Covered by `tests/pinned_layer_shell_geometry_test.cpp` and updated resize-controller tests.
- **Layer-Shell Window Animations**: Suppressed entry/exit animations on layer-shell windows for pinned, scroll, and capture surfaces so they appear and disappear instantly.
- **Persisted Annotation Color**: Ensured the persisted default annotation color is applied at startup before UI construction, preventing the color from briefly reverting to the built-in default on launch.

## 0.1.29 - 2026-06-18

### Features & Enhancements

- **Independent Magnifier Frame Resize**: The magnifier annotation now exposes resize handles on both the inner source viewfinder and the outer lens. Rectangle lenses get 8 corner/edge handles per frame; circular lenses get 4. Resizing either frame keeps `magnifierScale` constant by scaling the other frame proportionally, so the loupe ratio is preserved no matter which side the user grabs. The drag/translate logic moves into a dedicated `shot_window_magnifier_drag.cpp` to keep the annotation editing pass small and the resize flow self-contained.
- **Rectangle Highlight & Invert Styles**: The rectangle tool gains a style selector with three modes:
  - `Stroke`: existing outlined or filled rectangle with optional rounded corners.
  - `Highlight`: marker-pen overlay using `CompositionMode_Multiply` with a semi-transparent fill, mirroring the highlighter look but bound to a rectangle.
  - `Invert`: inverts the RGB pixels covered by the rectangle, with the outline stroke kept as a visual cue.
  The fill toggle and corner radius slider are hidden when Highlight or Invert is active, since those styles do not consume `filled` or `cornerRadius`.
- **Persisted Annotation Tool Defaults**: Tool defaults now survive across sessions through a dedicated state file at `~/.config/mark-shot/annotation-state.json`. The persisted snapshot covers active color and opacity, text background color, per-tool widths (pen / shape / number / mosaic block / laser), rectangle fill / corner radius / style, magnifier scale and lens shape, arrow / highlighter / number badge styles, and text font family. Writes go through `QSaveFile` for atomic commits, triggered immediately after every default-changing entry point so a crash never leaves the file half-written. The state is loaded before UI construction so the toolbar reflects the saved defaults from the very first paint.

### Bug Fixes

- **Annotation Width State Consistency**: Unified the standard stroke width used by Pen, Line, Arrow, Laser, Rectangle, and Ellipse tools while keeping Highlighter width, Number size, Text size, and Mosaic granularity independent. Mouse wheel and slider adjustments now flow through the same state update path, selected-object width changes persist correctly, and high-frequency width edits are debounced before writing history or disk state.
- **Translation Target Language Controls**: Added a target language selector to the OCR result floating panel and the pinned image context menu. The selector now uses a stable custom dropdown affordance, saves `translation.targetLanguage`, and clears stale pinned-window translation overlays when the target language changes.

## 0.1.28 - 2026-06-16

### Features & Enhancements

- **Configurable Clipboard Image Policy**: Added `clipboard.image.mode` with `image/png`, `url`, and `threshold` modes, plus `thresholdM` size control in megabytes. The default now keeps direct `image/png` clipboard data for better paste compatibility.
- **Shift-Constrained Line Drawing**: Holding `Shift` while drawing Line, Arrow, or straight Highlighter annotations now snaps strokes to horizontal, vertical, or 45-degree directions.
- **Updated Demo Video**: Replaced the README demo video with the latest GitHub user-attachments asset.

### Bug Fixes

- **Default Config Creation**: Ensured startup creates a default `config.json` when missing and includes clipboard defaults.

## 0.1.27 - 2026-06-11

### Features & Enhancements

- **Multi-point Line/Arrow Skeleton Editing**: Introduced support for adding, dragging, and deleting multiple skeleton (control) points on line and arrow annotations. Paths are smoothed using continuous quadratic Bezier curves, ensuring endpoints precisely target endpoints.
- **KDE Window Detection Script**: Added bundled `mark-shot-window-detection-kde` helper for KDE Plasma (KWin Wayland), expanding window boundary detection support.
- **Toolbar Appearance Configuration**: Added `toolbarAppearance` config options to customize the annotation toolbar layout and icon styles.
- **Shortcut & Interaction Improvements**: Enhanced keyboard and scroll interactions (e.g. using Backspace/Delete to remove selected skeleton points), and refactored input shortcut processing logic.

### Bug Fixes

- **Release Packaging**: Repaired AppImage and RPM release packaging, fixed tag-based AUR package publishing, and included the KDE helper in RPM packages.

## 0.1.26 - 2026-06-10

### Features & Enhancements

- **Custom Save Path & Placeholders**: Introduced flexible screenshot save templates (`save.pathTemplate` and `save.directoryTemplate`), supporting 30+ dynamic placeholders like `{pictures}`, `{datetime}`, and custom formatting like `{datetime:yyyy-MM-dd}` for versatile directory structures and naming schemes.
- **KDE KWin Screenshot Control Switch**: Added the `capture.wayland.kde.kwinScreenshot.enabled` option to enable or disable using KWin's restricted `org.kde.KWin.ScreenShot2` D-Bus interface, facilitating fallback debug routines.
- **Document Layout Optimization**: Refactored the user guide to collapse long KDE DBus setup details and application configuration parameters, improving overall readability.

## 0.1.25 - 2026-06-09

### Features & Enhancements

- **Configure Screen Freeze Scope**: 
  - Introduced screen freeze scope config (`capture.freezeScope` / `captureFreezeScope`) to control which screens are frozen during region selection screenshots in multi-monitor environments.
  - Supports `all-screens` (default) to capture and freeze all connected outputs or `cursor-screen` to freeze only the display containing the cursor.
- **Pinned Image Window Architecture Refactor**: 
  - Extracted the massive inline sticker/pinned window logic from `shot_window_pinned_window.cpp` into a modular directory structure under `src/pinned_window/` (`pinned_image_window.cpp/h`, OCR, translation, selection, geometry, and resize controller modules) for better code readability and maintenance.
- **Modularized Startup and Capture Initialization**:
  - Refactored various initialization routines (environment variable override, configuration parsing, default tools, and Qt portal service disabling) out of `main.cpp` into a dedicated `startup_config` module.
  - Relocated session launch logic to a standalone `capture_session_launcher` module, separating viewport calculation, window instantiation, and screen freezing strategies from the main executable entry.

### Tests

- **Added Unit Tests**:
  - Added unit tests for capture freeze scope (`tests/capture_freeze_scope_test.cpp`) and pinned resize controller (`tests/pinned_resize_controller_test.cpp`).

## 0.1.24 - 2026-06-09

### Features & Enhancements

- **Wayland Pinned Window Topmost Support (Always on Top)**: 
  - Implemented custom cross-platform topmost management (`pinned_window_top.cpp`) for pinned image windows.
  - Added native support for Wayland LayerShell Top protocol role, with dynamic role switching and fallback strategies.
  - Introduced configurable `"alwaysOnTop"` preference for pinned windows (default `true`) with toggle option in the context menu.
  - Added delayed scheduling (`schedulePinnedWindowRaise`) to reliably assert topmost status across different Wayland compositors during window mapping.
- **Windows Graphics Capture (WGC) Backend**:
  - Added high-performance native Windows Graphics Capture backend (`screen_capture_windows_wgc.cpp`) for smooth, hardware-accelerated screen capturing on Windows 10/11.
  - Enabled borderless window capture mode to strip shadow margins and window borders for clean screenshot outputs.
  - Resolved MinGW compatibility and runtime dependency issues for MSYS2/UCRT64 toolchains.
- **GNOME Shell Extension & Window Detection Refactor**:
  - Heavy refactoring of the GNOME Shell scroll helper extension (`extension.js`) to improve GNOME Wayland compatibility.
  - Introduced `mark-shot-window-detection-gnome` helper script for reliable window geometry and boundary detection under GNOME.
- **Unified Configuration Storage**:
  - Implemented standard configuration store (`app_config_store.cpp`) for atomic preference updates, improving robustness when reading and writing configuration settings.
- **Improved Annotation Workflows**:
  - Added support for multiple number stamp sequences and numbering styles (Arabic, Alphabetic, Roman, Chinese, and Heavenly Stems) with sequence reset button.
  - Smart automatic repositioning of the text annotation editor based on remaining boundary space to prevent input panels from clipping.
  - Enabled native input method (IME) support for the text editor and ensured text cursor visibility during long multi-line inputs.
- **Configurable Debug Logging**:
  - Added `--debug`, `--no-debug`, and `--debug-log <path>` CLI options for configurable troubleshooting.
  - Added `debug.enabled` and `debug.logPath` to `config.json` while maintaining backward compatibility with `DEBUG` env vars.

### Bug Fixes

- **Windows Scroll & Capture Artifacts**:
  - Fixed scroll preview positioning and visibility issues in multi-monitor setups.
  - Added exclusionary logic in Windows hook routines to filter out scroll preview overlays from capture frames.
- **Windows Thread Affinity**:
  - Corrected Windows affinity configuration logic during system API integration.
- **Wayland Overlay Handling**:
  - Improved screen overlay placement and coordinate translations under Wayland LayerShell environments.

### CI & Build

- **Windows Build Pipeline**:
  - Configured CI workflows to provision required C++ WinRT headers, allowing successful automated builds on Windows runner environments.

### Release Artifacts

- `mark-shot-v0.1.24-linux-x86_64.tar.gz`
- `mark-shot-v0.1.24-linux-arm64.tar.gz`
- `mark-shot_0.1.24_amd64.deb`
- `mark-shot_0.1.24_arm64.deb`
- `mark-shot_0.1.24_fedora_x86_64.rpm`
- `mark-shot_0.1.24_fedora_aarch64.rpm`
- `mark-shot-v0.1.24-linux-x86_64.AppImage`
- `mark-shot-v0.1.24-linux-x86_64.flatpak`

## 0.1.23 - 2026-06-08

### Features & Enhancements

- **Windows Build Support & Tray Integration**: Added support for compiling on Windows (MSYS2/UCRT64) with Qt 6. Implemented system tray icon support, global hotkey registration, native system fonts, and virtual screen geometry calculations for multi-monitor Windows setups.
- **App Icon & Tray Default Configuration**: Added application icon `mark-shot.ico` and updated default tray settings.
- **Supporting Documentation & Code Comments**: Added detailed in-code documentation and comments for core modules to make the system architecture clearer.

### Bug Fixes

- **Windows Screen Capture Configuration**: Expose virtual screen geometries on Windows builds and ensure default config creation at startup to avoid runtime launch issues.

### Refactoring

- **Source Code Restructuring**: Split oversized source files (such as `shot_window.cpp` and `screen_capture.cpp`) into cohesive submodules to improve maintainability and testability.

### Release Artifacts

- `mark-shot-v0.1.23-linux-x86_64.tar.gz`
- `mark-shot-v0.1.23-linux-arm64.tar.gz`
- `mark-shot_0.1.23_amd64.deb`
- `mark-shot_0.1.23_arm64.deb`
- `mark-shot_0.1.23_fedora_x86_64.rpm`
- `mark-shot_0.1.23_fedora_aarch64.rpm`
- `mark-shot-v0.1.23-linux-x86_64.AppImage`
- `mark-shot-v0.1.23-linux-x86_64.flatpak`

## 0.1.22 - 2026-06-07

### Features & Enhancements

- **Annotation Rotation & Curved Arrow**: Added rotation handle to annotation items (rectangles, ellipses, text, etc.) allowing arbitrary angle adjustments. Upgraded arrow annotations to support curvature adjustment via Bezier curve control points.
- **Highlighter Style & Magnifier Scale**: Added freehand and straight-line drawing modes for highlighters with a selector in the property bar. Added magnifier scale customisation slider (default 2.75) to precisely tweak magnification strength.
- **Scroll Capture Frame Polish**: Re-engineered X11 window input masks to correctly overlay capture border regions without sacrificing click-through capabilities for nested scrolling.
- **Scroll Capture Edge Artifact Scrubbing**: Implemented an automated scan-and-repair algorithm (`scrubCaptureFrameArtifacts`) to scrub stray outline pixels and border remnants from final scrolling capture composites.
- **Scroll Capture Hide Preview option**: Introduced `scrollCapture.hidePreviewDuringCapture` (`hidePreviewWhileCapturing`) to collapse preview panel structures while capturing.
- **Rebuilt X11 Window Boundary Detection**: Switched X11 window lookup queries to leverage root stacking trackers (`_NET_CLIENT_LIST_STACKING` / `_NET_CLIENT_LIST`), parse window extents (`_NET_FRAME_EXTENTS`), skip obscured frames (`_NET_WM_STATE_HIDDEN` or iconic state), and bypass override-redirect surfaces. Added `windowDetection.enabled` global flag.

### Release Artifacts

- `mark-shot-v0.1.22-linux-x86_64.tar.gz`
- `mark-shot-v0.1.22-linux-arm64.tar.gz`
- `mark-shot_0.1.22_amd64.deb`
- `mark-shot_0.1.22_arm64.deb`
- `mark-shot_0.1.22_fedora_x86_64.rpm`
- `mark-shot_0.1.22_fedora_aarch64.rpm`
- `mark-shot-v0.1.22-linux-x86_64.AppImage`
- `mark-shot-v0.1.22-linux-x86_64.flatpak`

## 0.1.21 - 2026-06-06

### Features & Enhancements

- **Magnifier Annotation Tool**: Added a magnifier annotation tool supporting independent positioning of the magnification source and the lens. Once placed, users can drag the source circle and lens circle separately to precisely adjust magnification parameters.
- **Editable OCR Result Window**: Enabled an editable floating result panel by default for main selection OCR. Users can edit, copy, or translate the recognized text within the panel, or drag the panel to move it.
- **OCR Result Config & Environment Variables**: Integrated config option `ocr.resultPanel` (boolean or object) and environment variables `MARK_SHOT_OCR_RESULT_PANEL` / `MARK_SHOT_OCR_RESULT_WINDOW` to toggle the result panel. Users can disable this option to restore direct clipboard copy behavior.

### Release Artifacts

- `mark-shot-v0.1.21-linux-x86_64.tar.gz`
- `mark-shot-v0.1.21-linux-arm64.tar.gz`
- `mark-shot_0.1.21_amd64.deb`
- `mark-shot_0.1.21_arm64.deb`
- `mark-shot_0.1.21_fedora_x86_64.rpm`
- `mark-shot_0.1.21_fedora_aarch64.rpm`
- `mark-shot-v0.1.21-linux-x86_64.AppImage`
- `mark-shot-v0.1.21-linux-x86_64.flatpak`

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
