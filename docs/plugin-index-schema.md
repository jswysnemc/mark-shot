# 插件市场索引 Schema

插件市场索引文件名推荐为 `plugins.json`。索引由 GitHub 托管，Mark Shot 使用 C++/Qt 内置解析器读取，不依赖 Python。

## 顶层结构

```json
{
    "schemaVersion": 1,
    "generatedAt": "2026-07-06T00:00:00Z",
    "plugins": []
}
```

字段说明：

| 字段 | 必填 | 说明 |
|---|---:|---|
| `schemaVersion` | 是 | 当前固定为 `1` |
| `generatedAt` | 否 | 索引生成时间，建议使用 UTC ISO 8601 |
| `plugins` | 是 | 插件条目数组 |

## 插件条目

```json
{
    "id": "mark-shot-sample-ocr",
    "name": "mark-shot-sample-ocr",
    "version": "0.1.0",
    "vendor": "example",
    "description": "Sample OCR provider for Mark Shot.",
    "markShotMinVersion": "0.1.38",
    "homepage": "https://github.com/example/mark-shot-sample-ocr",
    "license": "MIT",
    "capabilities": [],
    "assets": []
}
```

字段说明：

| 字段 | 必填 | 说明 |
|---|---:|---|
| `id` | 是 | 市场中的稳定插件标识，使用小写字母、数字、点和连字符 |
| `name` | 是 | 插件名称，建议与插件仓库或构建 target 对齐 |
| `version` | 是 | 插件版本，使用 SemVer |
| `vendor` | 是 | 发布方标识 |
| `description` | 否 | 插件简介 |
| `markShotMinVersion` | 是 | 最低兼容 Mark Shot 版本 |
| `homepage` | 否 | 插件主页，必须是 HTTP 或 HTTPS |
| `license` | 否 | 许可证标识 |
| `capabilities` | 是 | 插件能力数组 |
| `assets` | 是 | 平台资产数组 |

## 能力条目

```json
{
    "type": "ocr",
    "providerId": "sample-ocr",
    "displayName": "Sample OCR"
}
```

`type` 支持：

| 类型 | 说明 |
|---|---|
| `ocr` | OCR Provider |
| `translation` | 翻译 Provider |
| `code-scan` | 扫码 Provider |

`providerId` 必须与 C++ 插件返回的 `providerId()` 完全一致。发布后不要修改，否则用户配置中的 `plugin:<providerId>` 会失效。

## 资产条目

```json
{
    "platform": "windows",
    "architecture": "x86_64",
    "fileName": "mark-shot-sample-ocr.dll",
    "downloadUrl": "https://github.com/example/mark-shot-sample-ocr/releases/download/v0.1.0/mark-shot-sample-ocr.dll",
    "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "size": 1048576
}
```

字段说明：

| 字段 | 必填 | 说明 |
|---|---:|---|
| `platform` | 是 | `windows`、`linux`、`macos` |
| `architecture` | 是 | 推荐 `x86_64` 或 `aarch64` |
| `fileName` | 是 | Release 资产文件名，也是默认安装文件名 |
| `libraryFileName` | 否 | 资产名和安装库名不一致时使用 |
| `downloadUrl` | 是 | GitHub Release 资产 HTTPS URL |
| `sha256` | 是 | 资产 SHA-256 摘要 |
| `size` | 是 | 资产字节数 |

当前市场一键安装只支持动态库资产：Windows `.dll`、Linux `.so`、macOS `.dylib`。

## 校验方式

Mark Shot 构建测试会校验索引解析器和 fixture：

```bash
cmake -S . -B build -DBUILD_TESTING=ON
cmake --build build --parallel
ctest --test-dir build -R plugin-index-parser --output-on-failure
```

第三方市场仓库不需要 Python 校验脚本。可以把 `examples/plugin-index.example.json` 作为起点，保持字段类型和命名规则一致。
