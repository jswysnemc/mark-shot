# Code Scan Provider Template

This template builds a minimal Mark Shot QR code and barcode scan provider plugin. Replace the sample empty result with your scanner implementation.

## Build

```bash
cmake -S . -B build -DMARK_SHOT_PLUGIN_SDK_DIR=/path/to/mark-shot/plugin-sdk
cmake --build build --parallel
```

## Install For Local Testing

```bash
mkdir -p ~/.local/share/mark-shot/plugins
cp build/libmark-shot-sample-code-scan.so ~/.local/share/mark-shot/plugins/
```

Set `codeScan.provider` to `plugin:sample-code-scan` in Mark Shot settings, or choose it from the Plugins settings page.
