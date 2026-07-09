# Release Notes

### 0.1.39

- **Wayland Multi-Monitor Capture**: Fixed mixed-scale multi-monitor screenshots by capturing Wayland outputs independently, preventing half-screen selection and incorrectly scaled overlays.
- **Niri DMS Window Geometry**: The niri window detector now reads DMS bar, dock, frame, and frame-exclusion settings so tiled-window selection aligns with the visible window bounds.
- **Pinned Windows Across Outputs**: Pinned layer-shell windows now rebind to the target output while being dragged, so images remain visible after moving between monitors.
- **Image Frame Default**: The optional macOS-style export frame is now disabled by default. Existing configurations can still enable it with `export.imageFrame.enabled`.
- **Capture Window Visibility Setting**: `capture.hideOwnWindows` is now read at capture time and applied consistently to single-screen and multi-screen paths, so settings changes take effect without restarting.
- **Number Badge Rotation**: Number annotations now rotate around the badge center, with matching hit testing and selection geometry.
- **Standalone Plugin Assets**: Release builds now publish provider plugins as separate checksummed assets, with a stabilized Rapid OCR plugin build.
- **Packaging and Documentation**: Arch packages now depend on FFmpeg for recording support, all package versions are synchronized, and installed packages include the linked configuration and release documentation.


### 0.1.38

- **Plugin Ecosystem Foundation**: Added provider plugin registration, user-level plugin directories, provider preference configuration, and a Plugins settings page for OCR, translation, and code scanning extensions.
- **GitHub Plugin Marketplace**: Added the C++/Qt plugin index parser, download, SHA-256 verification, and install flow. The marketplace can be hosted entirely on GitHub Releases without requiring Python.
- **C++ Rapid OCR Plugin Upgrade**: Rapid OCR now emits word-level tokens, splits Chinese text and punctuation into selectable characters, splits Latin text by whitespace, and reuses existing RapidOCR model directories.
- **Pinned Text Selection Fixes**: Fixed half-width highlight backgrounds for full-width Chinese characters and avoided unintended spaces when copying adjacent Chinese OCR tokens.

### 0.1.37

- **Windows Recording Audio**: Added native WASAPI loopback capture for Windows video recording, replacing the PulseAudio-only path on Windows.
- **Windows Release Packaging**: Enabled FFmpeg-backed Windows packages with runtime DLL deployment and Authenticode signing support for executables and DLLs.
- **Windows CI Build Fixes**: Fixed WASAPI GUID and recording test linkage so Windows builds, tests, signing, packaging, and artifact upload complete successfully.

<details>
<summary>Previous versions</summary>

### 0.1.36

- **Older PipeWire Build Compatibility**: Fixed Debian 12 / older PipeWire header builds by probing `spa_video_info_raw::flags` at configure time while keeping explicit DMA-BUF modifier detection on newer PipeWire versions.

### 0.1.35

- **Qt 6.4 DMA-BUF Build Compatibility**: Fixed Debian 12 / Qt 6.4 builds by guarding Qt Wayland native display access while preserving the Wayland EGL display path on Qt 6.5 and newer.

### 0.1.34

- **Theme Setting**: Added `ui.theme` with System, Dark, and Light options, including a General settings selector and immediate settings-dialog theme application.
- **PipeWire Recording Backend**: Improved Wayland recording capture with shared-memory and DMA-BUF PipeWire frame handling, plus wlroots screencopy and polling fallbacks when portal capture is unusable.
- **Recording Timeline Accuracy**: Aligned the recording status timer with saved video timestamps so portal authorization and capture startup delay are not counted in the displayed duration.
- **Settings Polish**: Localized the theme setting controls and normalized combobox and spinbox styling across widget styles.

### 0.1.33

- **GIF and Video Recording**: Added GIF and MP4 recording with stepped frame rates, display or region capture, optional video audio input, and configurable output directories.
- **Tray and CLI Recording Controls**: Added tray Start Recording and Stop Recording actions, live tray status, `--recording-status`, and `--stop-recording`.
- **Recording-Aware Capture Overlay**: Active recordings now show status in the frozen-frame overlay and can be stopped with `S` or the overlay button without blocking normal screenshots.
- **Save and Recording Notifications**: Added desktop notifications for recording start/save/failure and screenshot save completion.
- **Recording Dialog Updates**: The recording dialog now switches between GIF and video modes, defaults to the current display, and updates frame rate, audio, and output path controls as the mode changes.
- **Wayland and Text Selection Fixes**: Improved mixed-DPI Wayland capture placement and fixed right-click context menus so editable text selections are preserved.

### 0.1.32

- **Startup Shortcut Hint Panel**: Replaced the centered startup hint pill with a PixPin-style vertical shortcut panel that defaults to the left-bottom corner and moves to the left-top corner when the pointer approaches it.
- **Input Device Hints**: Added keyboard, mouse, and mouse-wheel glyphs to the startup shortcut panel so shortcut rows communicate the expected input method more clearly.
- **Window Z-Order Selection**: Improved window ordering across GNOME, KDE Plasma, Hyprland, X11, and Windows so region selection prefers the visually topmost matching window.
- **Wayland Fcitx5 Candidate Support**: Adjusted layer-shell cursor-rectangle handling so fcitx5 candidate windows appear correctly under Wayland capture overlays.
- **Settings Gear Icons**: Redrew the settings toolbar and General settings navigation icons as clearer gear glyphs instead of sun-like radial icons.
- **Tray Mode Compatibility**: Fixed startup behavior when Mark Shot is launched directly into tray mode on environments without an immediately available system tray.
- **Wayland Text Editor Width**: Prevented the annotation text editor from shrinking unexpectedly on fractional-scale Wayland displays.

### 0.1.31

- **CLI Image Pinning**: Added `--pin-image <path>` to open an existing local image directly as a pinned sticker window, skipping capture and selection.
- **Color Picker History**: The startup Color Picker now remembers recently picked colors, persisted in `config.json` under `colorPicker.history` (capped at 7 `#RRGGBBAA` entries) and shown as swatches in the color panel.
- **Interface Language Setting**: Added a configurable `ui.language` option (`system` / `english` / `chinese`) selectable from the General settings page; supersedes the legacy root-level `language` key.
- **Desktop-Aware Window Detection**: Mark Shot now auto-selects the matching window detection script at runtime (GNOME, KDE Plasma, Hyprland, Niri), falling back to the niri script on other Wayland sessions and native X11 detection on X11. Mismatched configured commands are corrected in memory without touching `config.json`.
- **GNOME Occluded Window Filtering**: The GNOME Shell scroll helper extension now filters fully occluded windows from detection results.
- **Prebuilt AUR Package**: Added a `mark-shot-bin` AUR package installing prebuilt pacman packages from GitHub Releases, alongside the source-based `mark-shot` package.
- **GNOME Adwaita Palette Fix**: Overrode the application palette at the `qApp` level so the dark palette fully replaces the libqtk3 base palette under GNOME Adwaita.
- **AUR Optional Dependencies**: Added `python-rapidocr`, `python-pillow`, and `python-zxing-cpp` as preferred OCR/code-scan optdepends.

### 0.1.30

- **Settings Configuration Dialog**: Added a dedicated settings window with pages for General, Capture, Annotation, Pinned, Scroll, Shortcuts, Storage, Integrations, and Advanced. Every previously file-only option is now editable in one place, backed by the same `config.json` store, with shared design tokens and a custom navigation sidebar.
- **Launch on Startup**: Added a `Launch on Startup` switch on the General page. Linux writes an XDG `autostart` desktop entry; Windows writes the current user's `Run` registry key. The switch disables itself on unsupported platforms.
- **Portal Global Shortcut Support**: Added an `xdg-desktop-portal` `GlobalShortcuts` backend so global capture hotkeys work on Wayland without X11.
- **Pinned Text Selection Toggle**: The pinned image window now exposes a configurable text-selection toggle, with `pinned_window_config` split into its own module.
- **Settings Entry During Capture**: Opening settings from the toolbar or shortcut now closes the frozen capture session first and defers the dialog to the next event-loop tick, avoiding conflicts with the layer-shell capture window.
- **Pinned Window Placement on Wayland**: Extracted layer-shell geometry computation and improved the resize controller, fixing off-screen and multi-monitor placement of pinned windows.

### 0.1.29

- **Independent Magnifier Frame Resize**: The magnifier annotation now exposes resize handles on both the inner source viewfinder and the outer lens. Rectangle lenses get 8 corner/edge handles per frame, circular lenses get 4. Resizing either frame keeps `magnifierScale` constant by scaling the other frame proportionally, so the loupe ratio stays consistent regardless of which side the user grabs.
- **Rectangle Highlight & Invert Styles**: The rectangle tool gains a style selector with three modes—`Stroke` (existing outlined / filled rectangle with optional rounded corners), `Highlight` (marker-pen overlay using `CompositionMode_Multiply` with semi-transparent fill), and `Invert` (inverts the RGB pixels covered by the rectangle while keeping the outline as a visual cue). Fill toggle and corner radius are hidden for `Highlight` and `Invert`.
- **Persistent Tool Defaults**: Annotation tool defaults (color, opacity, per-tool widths, rectangle fill / corner radius / style, magnifier scale and shape, arrow / highlighter / number style, text font, text background color) now survive across sessions through a dedicated `annotation-state.json` file. Writes are atomic via `QSaveFile` and triggered immediately after every default-changing entry point.

### 0.1.28

- **Configurable Clipboard Image Policy**: Added `clipboard.image.mode` with `image/png`, `url`, and `threshold` modes. The default now keeps direct `image/png` clipboard data for better compatibility with office suites and browser input fields, while `thresholdM` can still switch large images to file URL mode.
- **Default Runtime Config Creation**: Ensured runtime startup creates a default `config.json` when the file is missing, including the new clipboard defaults.
- **Shift-Constrained Line Drawing**: Holding `Shift` while drawing Line, Arrow, or straight Highlighter annotations now snaps the stroke to horizontal, vertical, or 45-degree directions.

### 0.1.27

- **Multi-point Line/Arrow Skeleton Editing**: Introduced support for adding, dragging, and deleting multiple skeleton (control) points on line and arrow annotations. Paths are smoothed using continuous quadratic Bezier curves, ensuring endpoints precisely target endpoints.
- **Shortcut & Interaction Improvements**: Enhanced keyboard and scroll interactions (e.g. using Backspace/Delete to remove selected skeleton points), and refactored input shortcut processing logic.

### 0.1.26

- **Custom Save Path & Placeholders**: Introduced flexible screenshot save templates (`save.pathTemplate` and `save.directoryTemplate`), supporting 30+ dynamic placeholders like `{pictures}`, `{datetime}`, and custom formatting like `{datetime:yyyy-MM-dd}` for versatile directory structures and naming schemes.
- **KDE KWin Screenshot Control Switch**: Added the `capture.wayland.kde.kwinScreenshot.enabled` option to enable or disable using KWin's restricted `org.kde.KWin.ScreenShot2` D-Bus interface, facilitating fallback debug routines.
- **Document Layout Optimization & Details Collapsing**: Refactored the user guide to collapse long KDE DBus setup details and application configuration parameters, improving overall readability.

</details>

---
