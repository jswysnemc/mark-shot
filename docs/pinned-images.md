# Pinned Images And Clipboard

## Entry Points

- Use the pin toolbar action or `Ctrl+P` after capturing or editing an image.
- Choose **Pin Clipboard** from the tray menu.
- Press `Ctrl+V` in a focused pinned image to create another sticker.
- Run `mark-shot --pin-clipboard` without starting a capture.
- Open a file with `mark-shot --pin-image path/to/image.png`.

Images take precedence when the clipboard also contains text. Text preserves line breaks and uses the configured theme. HTML is treated as plain text rather than rendered as rich content, and no remote resources are loaded. Limits are 20,000 UTF-16 code units, a 4,096-pixel text canvas height, and 64 MiB of image data. Oversized content reports an error instead of being truncated. Clipboard stickers are not automatically added to screenshot history.

## Movement And OCR

- OCR prepares text selection asynchronously after the sticker appears.
- Left-drag from recognized text selects it; no modifier key is required.
- Left-drag elsewhere moves the image immediately. Before recognition completes, dragging also moves the image; completion never changes an ongoing drag into text selection.
- `Ctrl+C` copies selected text.
- **Copy Image Text** and **Translate** explicitly request recognition or translation.
- The wheel scales the sticker. Double-click text to select its line; double-click elsewhere or press `Esc` to close.

The **Text Selection Copy** option enables background recognition for selection when OCR is enabled. This preparation neither triggers automatic translation nor displays missing-backend notifications. Explicit **Auto OCR** and **Auto Translate** settings retain their processing behavior. Disabling OCR prevents background recognition. Unselected recognition regions have no overlay background; only selected text is highlighted.

## Linux Desktop Support

On niri and other supported layer-shell compositors, pinned images use the existing overlay path. They do not require a tray process and remain alive after the editor closes. The desktop host displays the tray menu, so activating a tray action does not guarantee keyboard focus for Mark Shot. Unfocused tray and command-line processes bypass Qt's stale selection cache unless they own the clipboard, using `wl-paste` to read the current clipboard instead; install `wl-clipboard` for this path.

GNOME does not expose layer-shell to ordinary clients. Its pinned images remain regular windows, with the bundled `mark-shot-scroll-helper@snemc.org` extension maintaining the above state. Unique titles distinguish individual stickers; the extension tracks late window creation and changes to the above state without affecting unrelated application windows.

Update and re-enable the GNOME extension for the new tracking and scroll-preview theme support. Older extensions retain the original preview interface but not the new behavior. Above state does not guarantee visibility over lock screens, shell UI, or every fullscreen application. GNOME may deny clipboard access to an unfocused process, and `wl-paste` is not a guaranteed fallback there; focus an existing sticker and press `Ctrl+V` instead. Signal-level extension tests pass, but validation in a real GNOME session is still required.
