# 文字识别与翻译

[English](../ocr-and-translation.md) | **简体中文**

## 文字识别

Mark Shot 使用运行在 ONNX Runtime 上的 PP-OCRv5 模型在本地识别文字。图片不会离开本机，
识别过程也不调用 Python 或外部识别程序。普通截图不会加载模型。

### 首次使用

文字识别需要两项资源，程序不会自动下载。文字识别面板为每项资源提供下载按钮，并显示体积和进度。

| 资源 | 内容 |
| --- | --- |
| 推理库 | ONNX Runtime 1.28.0，下载后校验 SHA-256 |
| 语言模型 | 拉丁文、西里尔文、阿拉伯文、希腊文、中文与日文、韩文、泰文 |

下载可以取消，也可以断点续传。安装完成后可以离线使用。启用 `bundled-ocr`
特性的构建内嵌了推理库，见[安装说明](installation.md)。

### 开始识别

| 入口 | 方法 |
| --- | --- |
| 命令行 | 运行 `mark-shot --ocr` 后框选区域 |
| 编辑器 | 按 `O`，或点击动作工具栏的文字识别按钮 |
| 贴图 | 右键菜单中的**复制图片文字**或**拖选复制文字** |

### 结果窗口

识别完成后打开独立的结果窗口。

- **文字页。** 识别出的文字可以编辑，支持撤销和重做。`Ctrl+C` 复制所选内容，`Ctrl+Shift+C` 复制全文。
- **原图页。** 显示原始区域，滚轮缩放，拖动平移，双击复位。
- **表格页。** 文字构成多行多列时显示。单元格可以编辑，表格可以复制为 TSV、CSV、Markdown 或 HTML。
- **翻译。** 把文字翻译为目标语言，译文显示在原文旁。

结果窗口就绪后截图进程随即退出，模型不会继续占用内存。

如果希望直接在截图上选择文字，不打开结果窗口：

```toml
[ocr]
result_panel = false
```

### 引擎选项

```toml
[ocr]
threads = 4
mode = "on-demand"        # on-demand | at-launch | daemon
device = "auto"           # auto | cpu | gpu，仅常驻进程使用
daemon_idle_secs = 900    # 设为 0 时常驻进程不退出
```

| 模式 | 行为 |
| --- | --- |
| `on-demand` | 在一次截图中首次使用文字识别时创建引擎，默认值 |
| `at-launch` | 每次启动时创建引擎 |
| `daemon` | 由后台进程保持引擎就绪，空闲超过设定时间后退出 |

在 Windows 上，首次识别时程序会询问使用哪种引擎：**Windows 系统 OCR** 使用系统已安装的语言包，
不下载任何内容；**本地模型 OCR** 的工作方式与上文一致。选择保存在 `ocr.engine`
（`ask`、`system` 或 `paddle`），可以在设置的文字识别页面修改。

## 翻译

翻译有三个入口：编辑器中的翻译按钮，用于翻译选区内的文字；文字识别结果窗口；贴图的右键菜单。

只有主动发起翻译时，文字才会发送到翻译服务。打开设置窗口不会发送任何内容。

### 翻译服务

| 服务 | 所需信息 |
| --- | --- |
| 哔哩哔哩 Index-Translate | 无；未配置其他服务时默认使用该服务 |
| OpenAI Chat Completions 及兼容接口 | 地址、密钥、模型 |
| OpenAI Responses | 地址、密钥、模型 |
| Anthropic Messages | 密钥、模型 |
| Gemini | 密钥、模型 |
| 腾讯机器翻译 | Secret ID、Secret Key、地域 |
| 百度翻译 | 应用编号、密钥 |
| 有道翻译 | 应用 Key、应用密钥 |
| 自定义命令 | 一条执行翻译的命令 |

### 配置服务

1. 打开**设置 → 翻译服务**。
2. 选择目标语言和服务。
3. 填写地址和凭据，每个字段单独保存。
4. 对于基于模型的服务，点击**查询模型**可以列出该接口提供的模型，也可以直接输入模型名称。

密钥在界面上以遮蔽形式显示，但在配置文件中以明文保存。如果不希望把密钥写入文件，可以改用环境变量：

| 服务 | 变量 |
| --- | --- |
| OpenAI 兼容接口 | `MARK_SHOT_LLM_API_BASE`、`MARK_SHOT_LLM_API_KEY`、`MARK_SHOT_LLM_MODEL` |
| Gemini | `MARK_SHOT_GEMINI_API_BASE`、`MARK_SHOT_GEMINI_API_KEY`、`MARK_SHOT_GEMINI_MODEL` |
| Anthropic | `MARK_SHOT_ANTHROPIC_API_KEY`、`MARK_SHOT_ANTHROPIC_MODEL` |
| 腾讯 | `MARK_SHOT_TENCENT_SECRET_ID`、`MARK_SHOT_TENCENT_SECRET_KEY`、`MARK_SHOT_TENCENT_REGION` |
| 百度 | `MARK_SHOT_BAIDU_APP_ID`、`MARK_SHOT_BAIDU_APP_KEY` |
| 有道 | `MARK_SHOT_YOUDAO_APP_KEY`、`MARK_SHOT_YOUDAO_APP_SECRET` |

配置文件的 `[translation]` 保存通用选择：

```toml
[translation]
target_language = "Simplified Chinese"
provider = "auto"
command = ""
```

### 模型选项

基于模型的服务有两个开关：

- **请求关闭思考**要求模型不经推理直接作答。并非所有模型都支持该参数；服务拒绝请求时请关闭这个开关。
- **过滤译文中的思考标签**移除回答中的 `<think>` 块，默认开启。

**自定义正文**接受一个 JSON 对象，并将其合并到请求中，可用于设置 `temperature` 等参数。
模型、待翻译的文字和响应模式不能通过这种方式替换。

```json
{ "temperature": 0.1 }
```
