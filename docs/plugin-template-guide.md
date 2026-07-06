# 插件模板使用指南

本指南说明如何基于 `plugin-templates/` 创建第三方 Provider 插件。模板面向独立仓库使用，也可以在本仓内直接构建验证。

## 选择模板

| 能力 | 模板目录 | Provider 配置示例 |
|---|---|---|
| OCR | `plugin-templates/ocr-provider` | `ocr.provider = plugin:sample-ocr` |
| 翻译 | `plugin-templates/translate-provider` | `translation.provider = plugin:sample-translate` |
| 扫码 | `plugin-templates/code-scan-provider` | `codeScan.provider = plugin:sample-code-scan` |

## 改名清单

复制模板后必须修改以下位置：

- `metadata.json` 中的 `name`、`version`、`vendor`、`capabilities[].providerId`、`capabilities[].displayName`
- C++ 类名和文件名，例如 `SampleOcrPlugin`
- `providerId()` 返回值，该值必须长期稳定
- `displayName()` 返回值，该值会显示在设置页
- `CMakeLists.txt` 中的 target 名称

`providerId()` 应使用小写短标识，例如 `paddle-ocr`、`deepl-api`、`zbar-native`。发布后不要随意修改，否则用户配置中的 `plugin:<id>` 会失效。

## 构建

在模板目录中运行：

```bash
cmake -S . -B build -DMARK_SHOT_PLUGIN_SDK_DIR=/path/to/mark-shot/plugin-sdk
cmake --build build --parallel
```

如果模板仍在本仓内，`MARK_SHOT_PLUGIN_SDK_DIR` 可以省略，模板会默认使用 `../../plugin-sdk`。

## 本地安装

Linux 用户级插件目录：

```bash
mkdir -p ~/.local/share/mark-shot/plugins
cp build/libmark-shot-sample-ocr.so ~/.local/share/mark-shot/plugins/
```

替换上面的库名为你的实际插件库名。重启 Mark Shot 后，插件会出现在设置页的 `Plugins` 页面中。

## 实现要求

- `isAvailable()` 只做依赖、模型、配置等轻量检查，不执行耗时任务。
- `recognize()`、`translate()`、`scan()` 在工作线程调用，禁止访问 GUI。
- 输出坐标必须使用输入图像的像素坐标。
- 无识别结果也属于成功，返回 `true` 并输出空数组。
- 真实错误必须返回 `false` 并写入 `error`。

## 调试流程

1. 构建插件动态库。
2. 将动态库复制到用户插件目录。
3. 打开设置页 `Plugins`，确认插件显示为 `Available`。
4. 将对应能力设置为 `plugin:<providerId>`。
5. 如果要发布到插件市场，按 `docs/plugin-index-schema.md` 添加 Release 资产索引。
