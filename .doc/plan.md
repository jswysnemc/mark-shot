# Mark Shot 开发进度与计划

更新日期：2026-07-05
当前分支：`feature/plugin-runtime`（基于 main，用于插件运行时开发）

## 一、已完成

### 1. 录制管线性能优化（已合入 main，commit 9109f9a）

| 改动 | 说明 |
|---|---|
| 编码器多线程 | `thread_count=0` 自动分配，修复 libx264 单线程瓶颈；移除 `tune=zerolatency` |
| 补帧零转换 | `writeRepeatFrame` 复用已转换 AVFrame，只推进 pts，消除慢机器上的补帧雪崩 |
| 背压前置 | PipeWire/WGC 采集回调在拷贝与 GPU 读回之前丢帧，过载时不再为丢弃帧付全款 |
| BGRA 缓冲池 | `RecordingBgraBufferPool` 4 槽环形复用，消除每帧 8-33MB 新分配 |
| DMA-BUF 直读 | 着色器输出 BGRA 字节序 + GPU 裁剪 + 单次读回，每帧 4-5 次全帧拷贝降为 1 次；失败自动回退 QImage 链 |
| 硬件编码候选 | Linux h264_nvenc；Windows h264_nvenc/h264_amf/h264_mf；失败回退 libx264；`MARK_SHOT_RECORDING_SW_ENCODER=1` 可禁用 |
| 顺带修复 | GIF 链路忽略 yInverted 导致直读帧上下颠倒 |

### 2. 插件化架构（本分支，60 文件 +4367 行）

- **插件 SDK**（`plugin-sdk/markshot/`）：OCR/翻译/扫码三个自包含接口头，
  IID `dev.mark-shot.*ProviderPlugin/1.0`，接口不绑定推理运行时
- **provider 框架**（`src/providers/`）：`ProviderTask` 统一异步任务抽象，
  输出沿用 `{backend,tokens,errors}` JSON 契约；插件注册器沿用 layer-shell
  目录约定并新增用户级 `~/.local/share/mark-shot/plugins`
- **去 Python 化内置实现**：tesseract CLI 直调（TSV 解析 C++ 移植）、
  openai-compatible 翻译（QNetworkAccessManager）、zxing-cpp 扫码（编译期可选）
- **优先链**：自定义命令 > 显式 provider（`plugin`/`plugin:<id>`/`builtin`/`helper`）>
  auto；旧 rapidocr venv 用户在 auto 下保持 helper，升级零感知；
  Python helper 全部保留为兜底
- **ocr-rapid 插件**（`plugins/ocr-rapid/`）：ONNX Runtime 完整推理
  （DBNet 检测 + CTC 识别，PP-OCRv5），模型复用旧 venv 已下载文件；
  onnxruntime 缺失时构建自动跳过
- **配置与可观测**：`ocr/translation/codeScan.provider` 配置字段；
  设置页 Provider Status 卡片；`MARK_SHOT_DEBUG_LOG=providers` 日志
- **文档**：`docs/plugin-development.md`（接口、目录、优先链、最小骨架、兼容承诺）

### 验证状态

| 项目 | 状态 |
|---|---|
| 全量构建（含插件） | 通过，零警告 |
| 单元测试 | 29/29 通过 |
| ocr-rapid 真实模型推理 | 通过（"HELLO 123"/"你好世界" 渲染图识别，240ms） |
| 内置 tesseract / zxing | 编译验证通过（本机 5.5.2 / 3.0.2） |
| 录制 PipeWire 路径 | **未运行时验证**（本机 niri 走 wlroots 后端） |
| Windows 全部改动 | **仅代码审查**，未在 Windows 编译运行 |

## 二、待办计划

### 近期（发布前必做）

1. 发起 `feature/plugin-runtime` 分支 PR
2. Windows 构建验证：providers 层、WGC 录制改动、h264_mf 硬件编码、
   插件在 Windows 的加载（`.dll` + vcpkg onnxruntime/zxing）
3. PipeWire 录制路径实测：KDE 虚拟机（192.168.122.43）或
   `MARK_SHOT_RECORDING_BACKEND=pipewire` 强制后端，确认 DMA-BUF
   直读无 `direct-read fallback` 日志
4. 更新 CHANGELOG 与 README（插件使用说明、新配置字段、新环境变量）

### 中期（插件生态 v1 收尾）

5. 发行版打包：`mark-shot-ocr-rapid` 独立包（AUR 先行），主包 optdepends 声明
6. 设置页缺插件时的安装指引链接（Provider Status 卡片补充）
7. ocr-rapid 词级 token 切分（当前行级，贴图窗口划选粒度粗于旧 helper）
8. 录制 VAAPI 硬件编码（需 hwframes 上下文，Linux Intel/AMD 用户收益）

### 远期（v2 方向）

9. 应用内插件下载与更新（签名校验、模型分发），产品决策：v1 不做
10. 方向分类模型接入（非直立文字场景）
11. 翻译/扫码示例插件（验证多能力接口的生态可用性）
12. 旧 Python helper 的退役评估（观察插件采用率后再定，不主动迁移用户）

## 三、关键约定备忘

- 单文件上限 800 行；providers 层最大文件 267 行
- 插件接口不兼容变更必须提升 IID 版本号
- auto 链兼容规则：老用户已有配置的行为在升级后不得改变
- 调试日志域：`recording`、`screencast`、`providers`
