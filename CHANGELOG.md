# Changelog

## 0.1.56 - 2026-10-02

### Features & Enhancements

- **Remembered Screen-Sharing Authorization**: Request persistent ScreenCast permission and rotate single-use restore tokens across captures and application restarts. Tokens are stored in a user-private file and scoped to the desktop, display, cursor policy, and capture mode. Sharing still ends when the capture closes; revoked permissions or unsupported portals may require confirmation again.

### Bug Fixes

- **KDE Scroll Capture Fallback**: Try KWin ScreenShot2 after a preferred screencast fails, while respecting the user's KWin setting and own-window policy. Fixes #125.
- **Secondary-Display Recording Prompts**: Polling recordings use native fallbacks before portal screenshots, disable per-frame interactive requests, and stop retrying failed screencasts for the current recording. Fixes #126.
- **Multi-User Single Instance**: Isolate Unix sockets through the user's runtime directory or UID to prevent cross-user permission collisions. Merges #124.
- **Portal Cancellation and Recording Fallback**: Do not submit a second authorization request through D-Bus after libportal has submitted one. Preserve raw recording callbacks when libportal initialization falls back before submitting a request.

## 0.1.55 - 2026-09-30

### Features & Enhancements

- **Delayed Capture**: Added the `--delay <seconds>` CLI option (0–60s, decimals allowed) to wait before taking screenshots or headless captures, with automatic forwarding to running instances. The system tray menu now provides 3s, 5s, and 10s presets with cancel support.
- **Selection Aspect Ratio Lock & Exact Sizing**: Supported Shift-drag square constraint during region selection. Pressing `Ctrl+R` opens the exact size panel with width/height inputs and presets for 1:1, 4:3, 3:2, 16:9, 21:9, 3:4, and 9:16 aspect ratio locks.
- **Annotation Filter Styles & Spotlight**: Rectangle annotation now supports `Spotlight` mode which dims unselected areas. Mosaic tool now features selectable styles: `Pixelate`, `Blur`, `Grayscale`, `Invert`, and `Brighten`, with adjustable intensity via mouse wheel.
- **OCR Table View & Structured Export**: When recognized OCR text aligns in rows and columns, a dedicated **Table** tab appears in the OCR result window. Cells can be edited inline and copied as Spreadsheet (TSV), Markdown, CSV, or HTML.

### Bug Fixes

- **Wayland ScreenCast SelectSources Prompts**: Resolved repeated user authorization prompts by properly handling pending state before nested event loops.
- **Polling Capture Stream Life Cycle**: Prevented potential use-after-free conditions in the polling capture stream when capture sessions terminate during nested event processing.

## 0.1.54 - 2026-09-25

### Features & Enhancements

- **Google Gemini and Anthropic Claude Translation**: OCR translation adds two LLM provider plugins — `gemini` (Google Gemini `generateContent`) and `anthropic` (Anthropic Messages API). Credentials live in `translation.gemini` and `translation.anthropic`, or in environment variables, and can be entered on the Integrations settings page, which now offers provider selection and tabbed LLM credentials. `auto` resolves in the fixed order `openai-compatible` → `gemini` → `anthropic` → `tencent-tmt` → `baidu-fanyi` → `youdao-nmt`. Each plugin reads only its own config object, so the shared OpenAI-compatible `apiKey`, `model`, and `systemPrompt` are not inherited. `temperature` and Gemini `thinkingLevel` are sent only when set explicitly. Defaults are `gemini-3.5-flash-lite` and `claude-haiku-4-5`. See [docs/translation-providers.md](docs/translation-providers.md).
- **Configurable Translation System Prompt**: LLM providers accept a per-provider `systemPrompt`. Leaving it empty keeps the built-in translator prompt.

### Bug Fixes

- **HiDPI Crosshair Hotspot**: The capture crosshair hotspot is specified in logical coordinates. On HiDPI Wayland outputs the selection and annotations stay under the pointer instead of jumping away from the visual center. Fixes #116.
- **Portal Global Shortcuts App ID**: Shortcut registration uses a dedicated D-Bus connection so xdg-desktop-portal can bind the application id. `GlobalShortcuts.CreateSession` no longer fails with "An app id is required" when another portal call already used the shared session bus. Fixes #115.
- **Recording Dialog Focus on Multiple Screens**: Opening the recording dialog hides capture overlays on every screen. Leftover layer-shell overlays no longer keep exclusive keyboard focus, so the dialog can be focused and clicked. Fixes #114.
- **Debian Source Tag Fallback**: The debian-source workflow checks out `HEAD` when the upstream release tag is not present yet.

## 0.1.53 - 2026-09-21

### Features & Enhancements

- **Staged Windows Plugin Updates**: Plugin updates on Windows stage under `.pending-updates` so loaded DLLs remain locked without triggering file-overwrite errors, and verified updates apply before plugin discovery on restart. User plugins take priority over application-bundled plugins. See [docs/plugin-distribution.md](docs/plugin-distribution.md).
- **On-Demand Pinned Text Selection**: When automatic OCR on pinned stickers (`pinnedWindow.autoOcr`) is disabled, the first text-selection gesture triggers OCR on demand while preserving the gesture rectangle until recognition completes, and preserves blank-area window dragging. See [docs/configuration.md](docs/configuration.md).
- **Screenshot History Window Theme**: The screenshot history window aligns with settings design tokens and theme, adds an explicit close button, and preserves action button readability in compact window sizes.
- **HiDPI Capture Crosshair Scaling**: The capture crosshair cursor scales by integer factors based on device pixel ratio, keeping rendering crisp on HiDPI displays while retaining 256-byte row stride hardware buffer alignment.
- **Dynamic Toolbar Cursor Refresh**: Dynamic toolbar interactions asynchronously refresh the cursor to match underlying child widgets immediately after layout updates.

### Bug Fixes

- **OCR Model Path Alignment & Recovery**: RapidOCR model lookup rules now align with the model downloader across Windows (`%LOCALAPPDATA%/mark-shot/models`) and Linux (`~/.local/share/mark-shot/models` or `XDG_DATA_HOME`), refresh plugin availability automatically after downloads complete, and allow subsequent recognition retries after missing-model errors.
- **Marketplace Plugin Runtime Dependencies**: Removed unused direct Protobuf and Abseil dependencies from the RapidOCR plugin to prevent dynamic linking failures after system library updates, accompanied by explicit ELF dependency checks.
- **Windows Shell Command Quoting**: Preserved native argument quoting when launching external commands via cmd.exe, preventing paths with spaces or special characters from being incorrectly escaped under `CommandLineToArgvW` rules.
- **Windows Test Environment Compatibility**: Linked matching ONNX Runtime libraries and aligned test subsystem entry points with Qt `qmain` on Windows, and dynamically calculated settings form layout test viewports based on font metrics for runners lacking desktop fonts.

## 0.1.52 - 2026-09-13

### Features & Enhancements

- **OCR Result Window Redesign**: The OCR result window now separates the editable recognized text and the translation into dedicated panes with per-pane copy and undo actions plus copy feedback on the buttons. **Translate** in the top bar reveals the translation pane and its language controls, and the **Text / Source image** tabs share one content area that restores edits, undo history, scroll positions, and pane sizes. Wide windows show both panes side by side, narrow windows stack them, and compact windows keep their content. See [docs/user-guide.md](docs/user-guide.md).
- **Streamlined Recording Dialog**: The recording dialog keeps Video/GIF mode selection, display or region capture, and audio input in a compact layout. **Save to** and **Recording options** expand on demand, switching between Video and GIF retains each mode's frame rate and the selected audio input, and the window grows to fit expanded content while staying scrollable with visible bottom actions on small screens.
- **Settings Form Layout**: Settings labels and fields align across groups, numeric and shortcut fields use compact widths, and switch captions sit next to their checkboxes. Narrow windows replace the sidebar with a category selector and place labels above fields, with long labels wrapping to fit. **Save** applies without closing, **Undo changes** restores the most recently saved values, and pressing `Enter` in a single-line field saves while keeping the window open.
- **Interaction Cursor States**: Annotation editing now covers the complete cursor state machine. Hovering, moving, and resizing annotations show the matching cursor, and adjusting annotation width shows a live size preview beside the width cursor. See [docs/interaction-cursors.md](docs/interaction-cursors.md).

### Bug Fixes

- **KDE Pinned Windows**: The always-on-top KWin script now scopes to Mark Shot's own windows so pinned state no longer leaks between stickers, and resizing is handed to the window manager through native resize requests that preserve the image aspect ratio and resize anchors. See [docs/kde-pinned-windows.md](docs/kde-pinned-windows.md).
- **Hyprland Pinned Image Dragging**: Dragging a pinned layer-shell image on Hyprland now follows the pointer with a live drag preview and rebinds to the target output, so images no longer jump or disappear while being moved. See [docs/hyprland-pinned-windows.md](docs/hyprland-pinned-windows.md).
- **Compact OCR Windows**: Resizing an OCR result window to compact dimensions no longer drops its text, translation, or scroll state.

## 0.1.51 - 2026-09-06

### Features & Enhancements

- **Translation Request Body Extensions**: `translation.extraBody` merges extra JSON fields into the top level of OpenAI-compatible request bodies, for both the built-in translation task and the `translate-openai` plugin. The default is `{}`; omitting it or using an empty object preserves existing requests. `model`, `temperature`, and `messages` are always owned by the application, and a non-object value is reported as a configuration error before a request is sent. See [docs/configuration.md](docs/configuration.md).
- **Capture Hot-Path Caching**: Scroll capture ticks no longer repeat environment probes. Wayland session and desktop detection, KWin ScreenShot2 availability, the GNOME helper version, and the KWin screenshot setting are cached (mtime-checked for the config file, short TTLs for D-Bus probes), removing per-frame file reads, JSON parsing, and D-Bus round trips.
- **Faster First Screencast Frame**: The fixed 1500 ms settle sleep before the first screencast frame is replaced by a condition-variable wait with a single 2500 ms deadline, so capture returns as soon as the compositor delivers a frame instead of always paying the full delay.
- **Negotiated Stream Geometry Cache**: PipeWire stream geometry is computed once at format negotiation and reused by every frame callback, instead of being re-parsed from stream properties on each frame.
- **Scroll Capture Frame-Rate Cap**: Scroll capture requests a target FPS matching the session interval (about 22 fps at 45 ms), so dense frames are dropped inside PipeWire instead of triggering pointless full-frame readbacks.

### Bug Fixes

- **KDE Window Hover Selection**: Fully occluded windows are filtered out of KDE window detection, so hovering no longer selects a hidden window stacked underneath another one. Window info is also collected after the frame capture starts, giving fresher geometry for the overlay.
- **DMA-BUF Import Fallback**: A failed EGL DMA-BUF import now sets a process-level broken marker, and the next stream negotiation falls back to shared memory instead of repeating the doomed import per frame.

## 0.1.50 - 2026-08-30

### Features & Enhancements

- **Tencent, Baidu, and Youdao Translation**: OCR translation is no longer limited to OpenAI-compatible endpoints. Three provider plugins ship alongside it — `tencent-tmt` (Tencent Cloud API 3.0, TC3-HMAC-SHA256), `baidu-fanyi` (Baidu general text translation, MD5 signature), and `youdao-nmt` (Youdao text translation, v3 signature). Credentials are entered on the Integrations settings page or read from environment variables, and the provider is selected through `translation.provider` or the Plugins settings page. See [docs/translation-providers.md](docs/translation-providers.md).
- **Shared Translation Plugin Infrastructure**: Config lookup, synchronous HTTP, language-name normalization, and character-budget batching moved into a shared layer that all four translation plugins use, replacing what would otherwise be four copies of the config path search.
- **Deterministic Provider Auto-selection**: With four translation plugins installed, the `auto` chain now sorts candidates by a fixed order (`openai-compatible`, `tencent-tmt`, `baidu-fanyi`, `youdao-nmt`) instead of taking whichever plugin the registry happened to load first, so existing configurations keep resolving to the same provider.
- **Debian and Ubuntu Source Packaging**: Conventional Debian source packaging under `debian/`, an Ubuntu 26.04 Resolute source and binary validation workflow, and an optional Launchpad PPA publication workflow. See [docs/ubuntu-packaging.md](docs/ubuntu-packaging.md).

### Bug Fixes

- **Batch Translation Alignment**: Translation responses whose segment count does not match the request are rejected outright. A vendor-side mismatch surfaces as an error instead of silently shifting translated text between segments.
- **Qt 6.2 Compatibility**: The Tencent signer used `QTimeZone::UTC`, which requires Qt 6.5 and broke builds against the Qt 6.2 baseline.

## 0.1.49 - 2026-08-23

### Features & Enhancements

- **Selection Loupe**: Region selection can show a magnifier next to the cursor and nudge the pointer with the arrow keys (Shift+arrow moves 10 pixels). The feature is off by default and is enabled from Settings -> Capture -> Selection Loupe or `capture.selectionLoupe.enabled`. On Wayland, if the compositor rejects cursor warping, the system pointer is hidden and a software crosshair is drawn at the logical point; clicks and drags use that point.

### Bug Fixes

- **GNOME Custom Shortcuts**: `gsettings` values are written as quoted GVariant string literals, so commands such as `"/usr/bin/mark-shot" --capture` register instead of failing at the space after the quoted path.
- **High-refresh Wayland Selection**: Initial region dragging coalesces full-frame repaints and skips loading LayerShellQt on GNOME Wayland, which does not support layer-shell.
- **GNOME Window Helper Backoff**: A missing or failing bundled GNOME window-detection helper is not probed again for 30 seconds in the same process.
- **Kvantum Tooltips**: The process-wide `QToolTip` palette and stylesheet follow the application theme so hover labels stay readable on dark Kvantum frames.
- **Wayland Upload Clipboard**: An uploaded image URL is published through a persistent `wl-copy` owner when available, so the clipboard still holds the URL after Mark Shot exits. A failed copy now reports “Copy failed” instead of a false success toast.
- **KDE Pinned Always-on-Top**: Pinned sticker windows stay above other windows on Plasma Wayland by loading a session KWin script that sets `keepAbove`. The window remains a normal xdg-toplevel, so dragging and resizing are unchanged.

## 0.1.48 - 2026-08-16

### Features & Enhancements

- **Double Click Action**: Double clicking an empty area inside the selection finishes a capture in one gesture. The action is configurable through `capture.doubleClickAction` and the Capture settings page — copy and close (default), save to the default folder, save as, pin to screen, cancel, or do nothing. Double clicking a text annotation still opens the inline editor.
- **Selection History**: Every confirmed capture selection is persisted, and comma / period step back and forth through the last ten selections while picking a region. Entries are stored in global logical coordinates and mapped back into the current frame, so they survive multi-monitor layouts and both portal- and grim-backed captures.
- **Recording UX**: The recording dialog gained audio device selection and a cleaner layout, and grabber teardown is deferred so stopping a recording no longer lags.
- **Plugin Marketplace**: The in-app marketplace and its GitHub-hosted plugin index shipped together, listing the OCR, translation, and code-scan provider plugins with per-platform SHA-256 checksums.
- **Marker Shapes**: The marker tool covers more shapes, with pinned-window OCR highlight fixes alongside.
- **GNOME Hotkey Fallback**: `xdg-desktop-portal-gnome` does not implement the `GlobalShortcuts` interface and X11 key grabbing is unavailable inside a Wayland session, so tray hotkeys could not be registered on GNOME Wayland at all. A last-resort backend now writes them into GNOME's media-keys custom keybindings through gsettings, cleaning up stale entries from crashed sessions and removing only Mark Shot's own bindings when unregistering.

### Bug Fixes

- **KDE Wayland Scroll Capture**: Scroll capture failed immediately on KDE Wayland because every route was exhausted — KWin routing is skipped for screencast requests, the silent screencast start needs portal authorization, the portal screenshot fallback is disabled during live scrolling, and grim is unsupported by KWin. One interactive portal prompt is now allowed as the last resort of the first captured frame to establish a reusable PipeWire session; later ticks reuse it silently and wlroots setups still succeed through grim before any prompt appears.
- **Auto Translation Overlay**: Auto Translate After OCR finished in the background without ever activating the overlay, so results stayed hidden until a manual Translate click. The overlay also drew theme-palette text on its fixed light background, leaving translations unreadable under the dark theme. The overlay now activates when background translation finishes and paints its text with an explicit dark foreground.
- **Bare Print Key on X11**: Mint and Cinnamon users commonly bind Print alone, but the X11 backend rejected every no-modifier sequence before `XGrabKey` ran and reported a misleading "already in use" error. Bare Print now registers, and Sys_Req plus all keysym levels are probed when resolving Print keycodes.
- **Blank Tray Slot**: StatusNotifierItem hosts run out of process and resolve the advertised icon name in their own standard paths only, so the tray slot stayed empty whenever the icon was reachable only through process-private lookup paths such as Nix wrapper `XDG_DATA_DIRS` or a development checkout. The name is now advertised only when the icon file exists in a shared location, and hosts otherwise receive pixmap data.
- **Marker Tool Name**: Selecting the marker tool highlighted the Ellipse toolbar button because `currentToolName()` reported markers as ellipses. Thanks to @webfrogs for the fix.

## 0.1.47 - 2026-08-10

### Features & Enhancements

- **Headless Capture CLI**: Screenshots can be taken without the interactive overlay, including capturing several displays in one run.
- **Text Size and Style Control**: The text tool exposes precise font size and style controls, and new annotations default to 20pt.

### Bug Fixes

- **Hover Window Selection**: Hardened window detection when picking a window by hovering, and the Wayland session probe moved into its own module with dedicated tests.
- **Capture Overlays**: Freezing now covers all screens and keeps overlays out of the taskbar.
- **Settings Wheel Guard**: Settings controls no longer change values from stray wheel scrolling over the window.
- **PipeWire Build Guard**: `pipewire_buffer_data_types` compiles without PipeWire headers, with the DMA-BUF avoidance policy kept inside the guards.
- **AUR Publishing During Maintenance**: Both AUR jobs exited non-zero when `aur.archlinux.org` refused connections during maintenance, marking a release failed even though every package had already been built and uploaded. Those steps now skip with a warning when git reports the maintenance notice, and still fail on real errors such as authentication problems.

## 0.1.46 - 2026-08-10

### Features & Enhancements

- **Scroll Capture Stitching Rework**: The stitcher was split into focused modules — match search, row signatures, fixed-region detection, and an incremental long-image buffer — and the long image is now grown in place instead of being repainted from scratch on every frame. On a 1200x900, 120-frame benchmark, stitching dropped from 46.2 ms to 1.68 ms per frame. Fixed headers and footers are detected from consecutive frames and trimmed so each is kept exactly once, and overlap verification now excludes the fixed bands that are about to be trimmed, which is what previously caused footers to be stitched into the middle of the long image and content to be lost.
- **X11 Global Shortcuts**: Global shortcuts now have a native X11 backend that grabs keys directly from the X server. The backend is selected by session type, with automatic fallback between the native and portal paths.

### Bug Fixes

- **Blank Tray Icon**: The application shipped only SVG icons, and the Qt SVG image plugin lives in a separate package (`qt6-svg` / `libqt6svg6`) that was never declared as a dependency. Without it, icon-theme lookup returned an unrenderable icon and the tray showed an empty slot. Bitmap icons are now installed in eight sizes and decode through the plugins bundled with Qt Base, the tray prefers a named theme icon so StatusNotifierItem hosts can render it themselves, and every package now declares the Qt SVG runtime.
- **Global Shortcuts on X11 Desktops**: Registration failed on Cinnamon, Xfce, MATE, and other X11 desktops with "no such interface" because the only backend was the xdg-desktop-portal `GlobalShortcuts` interface, which is a Wayland-oriented API those desktops do not implement. X11 sessions now grab keys natively, including under active NumLock or CapsLock.
- **Arch Package FFmpeg Dependency**: Arch packages declared a bare `ffmpeg` dependency, so the FFmpeg 9 update — which bumps every soname by one — left the package satisfying its dependency check while failing to load `libavformat.so.62` at startup. Packages now declare versioned soname dependencies, generated from the linked binaries for the prebuilt package, so pacman refuses the install outright when the library generation does not match.

## 0.1.45 - 2026-08-06

### Features & Enhancements

- **Linux Package Guide**: Added dedicated English and Chinese package guides that map Debian, Ubuntu, Fedora, AppImage, and Arch users to the correct artifacts and document the AppImage compatibility baseline.

### Bug Fixes

- **Debian 13 and Ubuntu 24.04 Packages**: Added native AMD64 and ARM64 builds for Debian 13 and Ubuntu 24.04 so each package links against the FFmpeg and Qt shared-library generations available on its target distribution. Every DEB is installed, dependency-checked, and started inside its target container before release upload.
- **AppImage glibc Compatibility**: Moved AppImage builds from Arch Linux to Debian 12, enforced a maximum `GLIBC_2.36` symbol requirement, and added an Ubuntu 24.04 startup test before release upload.
- **KDE Startup Notification**: Disabled desktop-entry startup notification so KDE no longer shows the Mark Shot launch cursor and icon while a screenshot is starting.

## 0.1.44 - 2026-07-26

### Bug Fixes

- **Debian and Ubuntu Packages**: The `.deb` workflow failed on every matrix entry because the code scanner assumed the zxing-cpp 3.x interface, while Debian 12 ships 1.4 and Ubuntu 26.04 ships 2.3. The reader parameter class is now aliased by major version and `ZX_USE_UTF8` keeps `text()` returning `std::string` on every release, so both distributions build again with the built-in scanner enabled. The plugin write-barcode test stays disabled below 3.0, which is the only version with a `CreateBarcode` equivalent.

## 0.1.43 - 2026-07-26

### Features & Enhancements

- **Pause and Resume Recording**: Recordings can be paused and resumed from the floating control bar, the tray menu, a global shortcut, or `--pause-recording`. Capture, encoding, and audio all stop together, and the paused span is subtracted from the output timeline so audio stays in sync.
- **Recording Control Bar and Region Frame**: Region recordings now show a red frame around the captured area plus a compact floating bar with elapsed time, pause, and stop. Both stay outside the recorded area and let clicks pass through everywhere else. Full-screen recordings skip the overlay because it would be captured, leaving the tray and shortcuts in charge.
- **Hardware Encoders on More GPUs**: Video encoding now considers VAAPI and Quick Sync on Linux and adds AMF and Quick Sync on Windows, instead of only NVENC. Candidates are probed against the codecs FFmpeg actually ships and the device nodes present on the machine, and multi-GPU systems try each render node so cards without encode support are skipped.
- **Parallel Pixel Conversion**: BGRA to YUV conversion runs across row slices in a persistent thread pool, removing the single-threaded conversion bottleneck from the writer thread.
- **GIF Per-Frame Palettes**: GIF recording quantizes through libavfilter `palettegen`/`paletteuse` with a per-frame palette instead of the fixed 3-3-2 palette, cutting average channel error on gradients from over 20 to roughly 3.
- **MKV Container and Quality Presets**: Video recordings can be written as MKV, which stays playable if a recording is interrupted, and a quality preset (balanced, higher quality, smaller file) drives the constant-quality value, encoder preset, and bitrate.
- **Recording Countdown**: An optional 3 or 5 second countdown runs before capture starts.
- **Open Folder from Save Notification**: The recording-saved notification offers to reveal the file through the file manager.

### Bug Fixes

- **KDE Recording on NVIDIA**: KWin cannot export usable DMA-BUF buffers on the NVIDIA proprietary driver, which made recording fail immediately on those systems. Single-GPU KDE sessions with that driver now negotiate shared memory automatically, hybrid-GPU machines keep DMA-BUF, `MARK_SHOT_FORCE_DMABUF` overrides the avoidance, and import failures now say how to work around the problem.
- **Static Screen Duration**: Event-driven capture backends emit no frames while the screen is unchanged, and the catch-up limit compressed those spans in the output. A heartbeat now writes repeat frames during idle periods so the recording duration matches real time.
- **GIF Capture Backend**: GIF recording was excluded from wlroots screencopy and fell back to full-path polling captures on niri, sway, and other wlroots compositors. It now shares the video capture path, with the writer dropping frames above the target rate.
- **Odd Frame Sizes**: Odd capture widths and heights are cropped rather than scaled, removing the slight distortion in the encoded video.
- **Recording Queue Depth**: The pending-frame queue is sized from frame memory and frame rate instead of a fixed one or two frames, so encoding jitter no longer drops frames unnecessarily.

## 0.1.42 - 2026-07-26

### Features & Enhancements

- **Shape Marker Tool**: Added a toolbar shape-marker group with triangle, star, check, cross, diamond/heart/spade/club, plus, ban, and other filled markers. Re-clicking the tool opens a shape palette.
- **Curve Marker Glyphs**: Spade, club, and ban markers now use cubic curves and rounded strokes instead of hard polygons, improving icon readability.
- **Dual Debian Packages**: Official `.deb` builds keep separate Debian 12 and Ubuntu 26.04 packages so FFmpeg/Qt shared-library generations remain installable on both baselines.
- **Plugin Dependency Packaging**: Deb packaging now runs `dpkg-shlibdeps` against the main binary and installed plugin modules, so OCR/zxing/layer-shell shared libraries are declared when present.

### Bug Fixes

- **Wayland Exclusive Zone Overlay**: Restored `exclusive_zone=-1` for layer-shell overlays so niri no longer leaves a live waybar gap above the frozen capture frame.
- **Invert Rectangle Border**: Removed the outer stroke from invert rectangles so inverted regions no longer show a red frame.
- **FFmpeg Arm64 Channel Layout**: Added FFmpeg 4.4-compatible channel-layout fallbacks for Ubuntu 22.04 arm64 CI and packaging builders.
- **Settings About Page**: Moved About out of Advanced into its own navigation page and tightened settings sidebar item spacing/alignment.

## 0.1.41 - 2026-07-16

### Bug Fixes

- **Windows Build Compatibility**: Restricted PipeWire SPA buffer helpers and their dedicated tests to Linux builds, restoring Windows compilation and packaging without changing Linux PipeWire capture behavior.

## 0.1.40 - 2026-07-16

### Features & Enhancements

- **Draggable Editing Toolbars**: Added dedicated drag grips to the annotation and action toolbars so both panels can be repositioned after a region capture.
- **Default Move Tool**: Changed the default annotation tool from Pen to Move for new installations and users without an explicit tool preference.

### Bug Fixes

- **Runtime Capture Settings**: Read cursor inclusion, freeze scope, and default tool settings for every capture so configuration changes take effect without restarting the tray process.
- **Annotation Cursor Feedback**: Restored the arrow cursor for the Move tool outside the selection and across toolbars, property panels, color controls, font lists, extension panels, and combo box popups.
- **Text Annotation Wrapping**: Added layout padding to text backgrounds so annotation text no longer wraps prematurely while editing.
- **KDE Capture Compatibility**: Improved KWin own-window policy handling and Wayland screenshot behavior for KDE capture sessions.
- **PipeWire Buffer Handling**: Added explicit PipeWire buffer data-type handling and tests to improve capture compatibility across shared-memory and DMA-BUF frame paths.

## Earlier releases

See [versions 0.1.12–0.1.39](docs/changelog/0.1.12-0.1.39.md).
