# 插件分发规范

本文定义第三方 Mark Shot Provider 插件的发布格式。该规范面向 GitHub Release、系统包、用户手工安装三种分发方式。

## 包命名

推荐格式：

```text
mark-shot-plugin-<capability>-<provider>-<version>-<platform>.<ext>
```

示例：

```text
mark-shot-plugin-ocr-paddle-0.1.0-linux-x86_64.tar.gz
mark-shot-plugin-translate-deepl-0.2.1-windows-x86_64.zip
```

## 包目录结构

```text
mark-shot-plugin-example/
├── README.md
├── metadata.json
├── LICENSE
└── libmark-shot-example.so
```

Linux 安装目标目录为 `~/.local/share/mark-shot/plugins`。系统包可以安装到 `${libdir}/mark-shot/plugins`。

## metadata.json

必填字段：

```json
{
    "name": "mark-shot-example",
    "version": "0.1.0",
    "vendor": "example",
    "markShotMinVersion": "0.1.38",
    "capabilities": [
        {
            "type": "ocr",
            "providerId": "example-ocr",
            "displayName": "Example OCR"
        }
    ]
}
```

字段约定：

| 字段 | 必填 | 说明 |
|---|---:|---|
| `name` | 是 | 插件包名，建议与动态库 target 对齐 |
| `version` | 是 | 插件版本，建议 SemVer |
| `vendor` | 是 | 发布方标识 |
| `markShotMinVersion` | 是 | 最低兼容 Mark Shot 版本 |
| `capabilities` | 是 | 能力列表 |
| `capabilities[].type` | 是 | `ocr`、`translation`、`code-scan` |
| `capabilities[].providerId` | 是 | 与 C++ `providerId()` 完全一致 |
| `capabilities[].displayName` | 是 | 与 C++ `displayName()` 建议一致 |
| `homepage` | 否 | 项目主页 |
| `license` | 否 | 许可证标识 |
| `dependencies` | 否 | 运行时依赖说明 |

## 兼容策略

- Provider 接口通过 IID 版本区分，例如 `dev.mark-shot.OcrProviderPlugin/1.0`。
- 如果接口发生不兼容变更，Mark Shot 会提升 IID 版本号。
- 插件必须声明 `markShotMinVersion`，避免用户安装到过旧版本后无法诊断。
- `providerId` 发布后应保持稳定，不要把品牌名、模型版本或地区写成会频繁变化的值。

## 发布检查

发布前至少执行：

```bash
cmake -S . -B build -DMARK_SHOT_PLUGIN_SDK_DIR=/path/to/plugin-sdk
cmake --build build --parallel
ctest --test-dir build --output-on-failure
```

如果插件依赖外部模型或服务，`README.md` 必须说明：

- 依赖安装方式
- 环境变量
- 模型文件目录
- API Key 配置方式
- 常见不可用原因

如果插件要进入 GitHub 插件市场，还需要把 Release 动态库资产写入市场索引。
索引格式见 `docs/plugin-index-schema.md`，示例见 `examples/plugin-index.example.json`。
