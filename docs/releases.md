# Release Notes

### 0.1.47

- **Headless Capture CLI**: Screenshots can be taken without the interactive overlay, including capturing several displays in one run.
- **Text Size and Style Control**: The text tool exposes precise font size and style controls, and new annotations default to 20pt.
- **Hover Window Selection**: Hardened window detection when picking a window by hovering, with the Wayland session probe moved into its own tested module.
- **Capture Overlays and Settings**: Freezing covers all screens and keeps overlays out of the taskbar, and settings controls no longer change values from stray wheel scrolling.
- **AUR Publishing During Maintenance**: A release is no longer marked failed when `aur.archlinux.org` is down for maintenance. The AUR steps skip with a warning in that case and still fail on real errors.

### 0.1.46

- **Faster Scroll Capture Stitching**: The stitcher grows the long image in place instead of repainting it from scratch on every frame, cutting a 1200x900, 120-frame benchmark from 46.2 ms to 1.68 ms per frame. Fixed headers and footers are detected from consecutive frames and kept exactly once, and overlap verification now excludes bands that are about to be trimmed, fixing footers being stitched into the middle of the long image.
- **Blank Tray Icon**: Bitmap icons are now installed alongside the SVG and decode through the plugins bundled with Qt Base, so the tray no longer shows an empty slot on systems without the separate Qt SVG package. Every package now declares that package as a dependency.
- **Global Shortcuts on X11**: Cinnamon, Xfce, MATE, and other X11 desktops do not implement the Wayland-oriented xdg-desktop-portal `GlobalShortcuts` interface, so registration failed with "no such interface". X11 sessions now grab keys directly from the X server, including under active NumLock or CapsLock.
- **Arch Package FFmpeg Dependency**: Arch packages declared a bare `ffmpeg` dependency, so the FFmpeg 9 update left them passing the dependency check while failing to load `libavformat.so.62` at startup. Packages now declare versioned soname dependencies, so pacman refuses the install when the library generation does not match.

### 0.1.45

- **Native Debian 13 and Ubuntu 24.04 Packages**: Added AMD64 and ARM64 packages built and verified inside their target distributions, avoiding FFmpeg and Qt dependency mismatches. The Linux package guide lists the correct artifact for each supported distribution.
- **Compatible AppImage Baseline**: AppImage payloads are now built on Debian 12, reject symbols newer than `GLIBC_2.36`, and must start successfully on Ubuntu 24.04 before they are uploaded to a release.
- **KDE Startup Notification**: Disabled desktop-entry startup notification so the launch cursor and application icon no longer interfere with screenshots while Mark Shot starts.

### 0.1.44

- **Debian and Ubuntu Packages**: The `.deb` workflow failed on every matrix entry because the code scanner assumed the zxing-cpp 3.x interface, while Debian 12 ships 1.4 and Ubuntu 26.04 ships 2.3. The reader parameter class is now aliased by major version and `ZX_USE_UTF8` keeps `text()` returning `std::string` on every release, so both distributions build again with the built-in scanner enabled.

### 0.1.43

- **Pause and Resume Recording**: Recordings can be paused and resumed from the floating control bar, the tray menu, a global shortcut, or `--pause-recording`. Capture, encoding, and audio stop together, and the paused span is subtracted from the output timeline so audio stays in sync.
- **Recording Control Bar and Region Frame**: Region recordings show a red frame around the captured area plus a compact floating bar with elapsed time, pause, and stop. Both stay outside the recorded area and let clicks pass through elsewhere. Full-screen recordings skip the overlay because it would be captured, leaving the tray and shortcuts in charge.
- **Hardware Encoders on More GPUs**: Video encoding adds VAAPI and Quick Sync on Linux plus AMF and Quick Sync on Windows instead of only NVENC. Candidates are filtered by the codecs FFmpeg actually ships and the device nodes present, and multi-GPU systems try each render node so cards without encode support are skipped.
- **Parallel Pixel Conversion**: BGRA to YUV conversion runs across row slices in a persistent thread pool, removing the conversion bottleneck from the writer thread.
- **GIF Per-Frame Palettes**: GIF recording quantizes through libavfilter `palettegen`/`paletteuse` with a per-frame palette instead of the fixed 3-3-2 palette, cutting average channel error on gradients from over 20 to roughly 3.
- **MKV Container and Quality Presets**: Video recordings can be written as MKV, which stays playable if a recording is interrupted, and a quality preset drives the constant-quality value, encoder preset, and bitrate.
- **Recording Countdown**: An optional 3 or 5 second countdown runs before capture starts.
- **Open Folder from Save Notification**: The recording-saved notification can reveal the file in the file manager.
- **KDE Recording on NVIDIA**: KWin cannot export usable DMA-BUF buffers on the NVIDIA proprietary driver, so recording failed immediately there. Single-GPU KDE sessions with that driver now negotiate shared memory automatically, hybrid-GPU machines keep DMA-BUF, `MARK_SHOT_FORCE_DMABUF` overrides the avoidance, and import failures explain the workaround.
- **Static Screen Duration**: Event-driven capture backends emit no frames while the screen is unchanged, and the catch-up limit compressed those spans. A heartbeat now writes repeat frames during idle periods so the duration matches real time.
- **GIF Capture Backend**: GIF recording was excluded from wlroots screencopy and fell back to full-path polling captures on niri and sway. It now shares the video capture path, with the writer dropping frames above the target rate.
- **Odd Frame Sizes**: Odd capture widths and heights are cropped instead of scaled, removing slight distortion in the encoded video.
- **Recording Queue Depth**: The pending-frame queue is sized from frame memory and frame rate instead of a fixed one or two frames, so encoding jitter no longer drops frames unnecessarily.

### 0.1.42

- **Shape Marker Tool**: Added a toolbar shape-marker group with triangle, star, check, cross, card suits, plus, ban, and more. Re-clicking the tool opens a shape palette with a solid panel background.
- **Curve Marker Glyphs**: Spade, club, and ban markers use cubic curves and rounded strokes for clearer silhouettes.
- **Dual Debian Packages**: Keep separate Debian 12 and Ubuntu 26.04 `.deb` packages for different FFmpeg/Qt shared-library generations.
- **Plugin Dependency Packaging**: Deb packaging scans the main binary and installed plugin modules when generating shared-library `Depends`.
- **Wayland Overlay Exclusive Zone**: Layer-shell overlays cover exclusive zones again so frozen captures no longer leave a live waybar gap.
- **Invert Rectangle Border**: Invert rectangles no longer draw an outer red frame.
- **FFmpeg Arm64 Compatibility**: Added channel-layout API fallbacks for FFmpeg 4.4 builders such as Ubuntu 22.04 arm64.
- **Settings About Page**: Moved About into its own settings navigation page and tightened sidebar spacing.

### 0.1.41

- **Windows Build Compatibility**: PipeWire SPA buffer helpers and their dedicated tests now compile only on Linux, restoring Windows builds and release packages while preserving Linux PipeWire capture behavior.

### 0.1.40

- **Runtime Capture Settings**: Cursor inclusion, freeze scope, and default annotation tools are now read for every capture, so settings changes apply without restarting Mark Shot.
- **Default Move Tool**: New installations now start annotation editing with the Move tool instead of Pen unless an explicit default is configured.
- **Draggable Toolbars**: The annotation toolbar and right-side action toolbar now include drag grips and retain their user-selected positions during editing.
- **Cursor Feedback**: The Move tool uses an arrow outside the selection, while toolbars, property panels, color controls, font lists, extension panels, and combo box popups no longer inherit drawing crosshairs.
- **Text Annotation Layout**: Text backgrounds include additional width padding to prevent premature wrapping in the editor.
- **KDE and PipeWire Capture**: Improved KWin own-window handling, KDE Wayland capture compatibility, and PipeWire buffer data-type processing with expanded tests.

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
