pub(super) const HELP: &str = "\
Mark Shot - native Rust screenshot editor for Linux Wayland and Windows

Usage: mark-shot [OPTIONS] [IMAGE]

Capture and editing:
  --all-outputs, --all-output   Capture the complete desktop
  -m, --monitor                 Capture the monitor under the pointer (default)
  --fullscreen, --full-screen   Open the editor with the complete capture selected
  -f, --full                    Capture immediately without the editor
  -w, --window                  Capture the active window (KDE; niri requires copying)
  -r, --select-region           Drag a region and finish immediately
  --ocr                         Select a region for builtin OCR; first use offers models
  -S, --scroll                  Select a region and stitch a scrolling screenshot
  -d, --delay SECONDS           Delay capture 0–60 seconds; fractional values are accepted
  --display NAME                Select an output; repeat for multiple outputs
  --include-cursor              Include the pointer in supported capture backends
  --default-tool TOOL           Initial annotation tool after selection
  --fullscreen-default-tool TOOL  Initial fullscreen annotation tool
  --file-default-tool TOOL       Initial image-file tool; overrides fullscreen defaults
  --default-color COLOR         #RRGGBB or #RRGGBBAA
  --xdg-window                  Use an xdg-shell window instead of the overlay
  -t, --to LETTERS              Finish actions: c copy, s save, p pin, o stdout
  --output DIRECTORY            Save into this directory

Headless capture:
  --capture-to FILE_OR_DIR      Write PNG and print a compact JSON report
  --region X,Y,WIDTH,HEIGHT     Global logical region; requires --capture-to
  --output-name NAME           Basename when writing into a directory
  --list-displays               Print available output information as JSON

Pinning:
  --pin-image FILE              Open an image as a separate pinned window
  --pin-clipboard               Pin a clipboard image or plain text

Screenshot history:
  --history                     Browse, search and manage completed captures
                                Ctrl+H opens history from the editor

Configuration and diagnostics:
  --config FILE                 Override ~/.config/mark-shot/config.toml
  --print-default-config        Print the complete default configuration
  --debug / --no-debug          Enable or disable diagnostic logging
  --debug-log FILE              Write diagnostics to a chosen file
  -s, --speed                   Print startup stage timings to stderr
  -n, --nonotify                Suppress desktop notifications
  -h, --help                    Show this help
  -v, --version                 Show the application version

OCR runs locally with PP-OCRv5. Models are downloaded only after an explicit click.
QR/barcode scanning and screen recording are not included in this rewrite.
";
