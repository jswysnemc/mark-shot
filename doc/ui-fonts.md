# Linux UI Fonts and Rendering

This document describes how Mark Shot matches interface fonts, applies rendering rules, and reloads configuration on Linux.

## Font Matching Rules

Mark Shot interface controls (including settings, hint overlays, OCR and translation panels, and pinned image context menus) handle fonts as follows:

1. **Fontconfig Default Sans-Serif**: The application queries Fontconfig for the system default sans-serif font family. This reflects Fontconfig configuration and is not guaranteed to match desktop UI font settings stored independently by KDE or GNOME.
2. **Interface Exceptions**: Controls requiring dedicated layout constraints retain explicitly specified families; for example, OCR table headers continue using DejaVu Sans.
3. **Built-in Fallback**: If font matching fails or no suitable font is found, the application falls back to the bundled Inter font to keep interface text readable.
4. **Independent Annotation Text**: Canvas text annotations in screenshot captures use an independent layout and export path; canvas text does not automatically follow the Fontconfig default font.

## Rendering Parameters

On Linux, Mark Shot checks the matched font weight and pixel size against Fontconfig rules, honoring:

- **Antialias**: Smooth contour rendering.
- **Hinting and Hintstyle**: None, Slight, Medium, and Full hinting levels.
- **Autohint**: System automatic hinting flags.
- **Subpixel Geometry (RGBA)**: RGB, BGR, VRGB, and VBGR subpixel orders.
- **LCD Filter**: Configured subpixel filtering algorithms.
- **Embedded Bitmap**: Size-dependent embedded bitmaps where configured.

### Transparent Background Optimization

When rendering text over translucent surfaces, Mark Shot automatically applies grayscale antialiasing to reduce color fringing from subpixel layouts.

## Reloading Font Configuration

Mark Shot does not watch Fontconfig configuration files at runtime.

When updating Fontconfig rules or installing new fonts:

1. **Standard restart**: Exit and launch Mark Shot again.
2. **Background standby**: If background standby or tray residency is enabled, quit the standby process completely from the tray menu before relaunching.
