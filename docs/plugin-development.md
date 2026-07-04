# Mark Shot 插件开发指南

Mark Shot 通过 Qt 插件机制扩展 OCR、翻译与扫码能力。插件是普通动态库，
放入插件搜索目录即可被自动发现，无需修改主程序。

## 能力与接口

| 能力 | 接口头（plugin-sdk/markshot/） | IID |
|---|---|---|
| OCR | `ocr_provider_plugin.h` | `dev.mark-shot.OcrProviderPlugin/1.0` |
| 翻译 | `translate_provider_plugin.h` | `dev.mark-shot.TranslateProviderPlugin/1.0` |
| 扫码 | `code_scan_provider_plugin.h` | `dev.mark-shot.CodeScanProviderPlugin/1.0` |

接口约定：

- 接口头自包含，只依赖 Qt Core/Gui，可独立于主程序源码分发。
- `recognize`/`translate`/`scan` 在工作线程调用，实现必须线程安全，禁止访问 GUI。
- 一个插件动态库可同时实现多个能力接口（`qobject_cast` 逐一匹配）。
- `isAvailable` 用于运行时可用性探测（模型文件、设备等），不可用时主程序自动回退。

## 插件搜索目录

按顺序扫描，全部目录的匹配插件都会加载：

1. `<应用目录>/plugins`
2. `<应用目录>/../lib/mark-shot/plugins`、`../lib64/mark-shot/plugins`
3. Qt libraryPaths 下的 `mark-shot/plugins`
4. `$XDG_DATA_HOME/mark-shot/plugins`（默认 `~/.local/share/mark-shot/plugins`）

用户级目录（第 4 项）适合免打包试用插件。

## provider 解析优先链

每个能力按以下顺序解析实际执行方：

1. 用户自定义命令（配置 `command` 字段，行为与旧版本一致）
2. 显式 provider 偏好（配置 `provider` 字段：`plugin` / `plugin:<id>` / `builtin` / `helper`）
3. `auto`（默认）：
   - OCR：旧 rapidocr venv 存在时保持 helper（老用户零感知）→ 插件 → 内置 tesseract → helper
   - 翻译：插件 → 内置 openai-compatible → helper
   - 扫码：插件 → 内置 zxing-cpp → helper

配置示例（应用配置 JSON）：

```json
{
  "ocr": { "provider": "plugin:rapid-onnx" },
  "translation": { "provider": "builtin" },
  "codeScan": { "provider": "auto" }
}
```

设置页「集成」标签的 Provider Status 卡片展示各能力当前实际生效的 provider。

## 最小插件骨架

```cpp
// my_ocr_plugin.h
#pragma once
#include "markshot/ocr_provider_plugin.h"
#include <QObject>

class MyOcrPlugin final : public QObject, public markshot::plugin::OcrProviderPlugin {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_OCR_PROVIDER_PLUGIN_IID FILE "metadata.json")
    Q_INTERFACES(markshot::plugin::OcrProviderPlugin)

public:
    QString providerId() const override { return QStringLiteral("my-ocr"); }
    QString displayName() const override { return QStringLiteral("My OCR"); }
    bool isAvailable(QString *error) const override;
    bool recognize(const QImage &image,
                   QVector<markshot::plugin::OcrToken> *tokens,
                   QString *error) override;
};
```

```cmake
# CMakeLists.txt
qt_add_plugin(my-ocr-plugin SHARED my_ocr_plugin.cpp my_ocr_plugin.h)
target_include_directories(my-ocr-plugin PRIVATE ${MARK_SHOT_PLUGIN_SDK_DIR})
target_link_libraries(my-ocr-plugin PRIVATE Qt6::Core Qt6::Gui)
```

`metadata.json` 提供名称与版本信息，供后续插件管理使用：

```json
{ "name": "my-ocr-plugin", "version": "1.0.0", "vendor": "example" }
```

## 参考实现：ocr-rapid

`plugins/ocr-rapid/` 是完整的 OCR 模型运行时插件，基于 ONNX Runtime 执行
PP-OCR 检测与识别模型：

- 模型发现：`MARK_SHOT_OCR_MODEL_DIR` → `~/.local/share/mark-shot/models` →
  旧 rapidocr venv 模型目录（老用户已下载的模型直接复用）
- 环境变量可逐项覆盖：`MARK_SHOT_RAPID_DET_MODEL`、`MARK_SHOT_RAPID_REC_MODEL`、
  `MARK_SHOT_RAPID_REC_DICT`
- 构建依赖 onnxruntime（pkg-config `libonnxruntime` 或 CMake config 包），
  缺失时该插件目标自动跳过，不影响主程序构建
- 已知限制：输出行级 token（无词级切分）；不含方向分类模型，
  假定文字直立（截图场景成立）

## 兼容性承诺

- 旧 Python helper（`mark-shot-ocr`/`mark-shot-translate`/`mark-shot-code-scan`）
  完整保留，永远作为兜底
- 自定义命令优先级最高，升级不改变既有配置的行为
- 已配置 rapidocr venv 的用户在 `auto` 模式下保持原有 helper 链路
- 接口不兼容变更会提升 IID 版本号，旧插件不会被误加载
