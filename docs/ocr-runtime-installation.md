# OCR 引擎与资源

默认构建采用动态加载的 ONNX Runtime，主程序不再嵌入推理库，也不在启动或首次 OCR 时自动下载。

## 使用方式

- Linux：OCR 面板分别提供推理库、语言模型的下载入口，两项均安装后开始识别
- Windows：首次选择“Windows 系统 OCR”或“本地模型 OCR”；选择后保存到 `ocr.engine`
- Windows 系统 OCR 使用系统已有且符合用户语言列表的 OCR 语言包，不下载第三方模型和推理库；缺少语言包时显示安装指引，保留原选择，不自动切换引擎
- 本地模型模式支持分别下载、进度显示、取消和重试；关闭安装窗口取消本窗口的下载
- 设置窗口的“文字识别”页可切换引擎，下次截图生效；截图内资源面板可直接切换
- 已安装资源可离线复用。截图、贴图识别和框选翻译共用资源检查和引导

Windows 系统接口提供词级坐标；当前字符选区采用行内插值，不能视为逐字符精确定位。
系统引擎不使用本地模型模式的常驻进程、设备选择。

## 发行与完整性

运行库固定为 [ONNX Runtime v1.28.0](https://github.com/microsoft/onnxruntime/releases/tag/v1.28.0)，
与 `ort 2.0.0-rc.13` 的 API 27 配合使用。下载归档和提取出的两个库文件均校验 SHA-256。
只提取库文件、许可证与第三方声明，不执行安装脚本，不跟随归档符号链接，不使用库搜索路径中的同名文件。
下载与安装有独立锁；校验失败的临时目录不会进入加载路径。

| 平台 | 官方归档下载大小 | 两个库文件合计 |
| --- | ---: | ---: |
| Linux x86_64 | 9,125,960 字节 | 24,283,480 字节 |
| Linux aarch64 | 8,116,278 字节 | 20,790,504 字节 |
| Windows x86_64 | 78,796,801 字节 | 15,831,704 字节 |

Windows 官方 ZIP 包含其他文件，所以下载量明显大于安装后的库。应用只安装清单中的库和许可证。
默认 Windows 发布包不携带 ONNX Runtime 或 DirectML，保留可选推理库需要的 MSVC CRT。

库安装在系统用户数据目录的 `mark-shot/runtime/onnxruntime-1.28.0-<平台>/installed/`：
Linux 通常为 `~/.local/share/mark-shot/runtime/`，Windows 通常为 `%APPDATA%\mark-shot\runtime\`。
模型目录仍为用户数据目录下的 `mark-shot/models/`。

## 构建选项

```sh
# 默认：库由用户在界面明确下载
cargo build --release --locked

# 嵌入推理库：模型仍由用户选择下载
cargo build --release --locked --no-default-features --features bundled-ocr

# 原有 WebGPU 构建：使用嵌入推理库，Dawn 仍遵循原有初始化流程
cargo build --release --locked --no-default-features --features ocr-gpu
```

`ocr-dynamic` 与 `bundled-ocr` 互斥，不使用 `--all-features`。
GPU 构建沿用原有 Dawn 行为；默认 CPU 构建的手动安装承诺不扩展到该可选构建。

## 验证方式

`cargo test --workspace --locked -- --test-threads=4` 覆盖默认行为、安装安全和现有业务回归。
`tests/ocr_runtime.rs` 的默认测试在独立数据目录中验证首次使用不创建下载任务。
实际离线安装和推理测试需要预先准备官方归档及三个模型文件：

```sh
MARK_SHOT_TEST_ORT_ARCHIVE=/path/onnxruntime-linux-x64-1.28.0.tgz \
MARK_SHOT_TEST_OCR_MODELS=/path/models \
cargo test --test ocr_runtime -- --include-ignored --nocapture
```

Windows 系统 OCR 需要真实 Windows 和匹配语言包验证。交叉编译通过不代表已经完成系统识别实测，
已有 Wine 和历史内置模型报告不能代替本次验证。
