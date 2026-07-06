# Translation Provider Template

This template builds a minimal Mark Shot translation provider plugin. Replace the sample passthrough implementation with your model or API client.

## Build

```bash
cmake -S . -B build -DMARK_SHOT_PLUGIN_SDK_DIR=/path/to/mark-shot/plugin-sdk
cmake --build build --parallel
```

## Install For Local Testing

```bash
mkdir -p ~/.local/share/mark-shot/plugins
cp build/libmark-shot-sample-translate.so ~/.local/share/mark-shot/plugins/
```

Set `translation.provider` to `plugin:sample-translate` in Mark Shot settings, or choose it from the Plugins settings page.
