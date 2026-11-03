# Mark Shot 原生 Wayland 预览版

这是 Rust 原生重写的首个公开预览版，不是 Qt 稳定版的替代发布。完整重写与 Qt 界面
一致性仍在补齐；二维码扫描和录像不在本次范围内。

## 运行范围

- Linux Wayland x86_64，在 Ubuntu 24.04 执行器上构建；需要 glibc 2.39 或更新版本，以及兼容的 Wayland、
  xkbcommon、PipeWire、Fontconfig 和 OpenSSL 运行库。
- 不依赖 Qt、GTK、浏览器或 WebView。归档包含可执行文件、使用文档、依赖许可、
  ONNX Runtime 许可和固定的源提交标记。
- OCR 模型不在归档中。首次使用文字识别时显示下载引导，用户确认后才下载；普通
  截图和文字编辑不初始化 OCR 模型。

下载 `.tar.gz` 和对应 `.sha256` 文件，放在同一目录后校验并解包：

```sh
sha256sum --check mark-shot-native-linux-x86_64.tar.gz.sha256
tar -xzf mark-shot-native-linux-x86_64.tar.gz
./mark-shot-native/mark-shot --help
./mark-shot-native/mark-shot
```

本次发布与验证不修改用户现有配置或 Qt 源码，不影响原版发布渠道，也不更新 GitHub Latest。

## 已实现与本次验证

- 截图、标注、贴图右键菜单、截图历史、OCR 和滚动截图提供基础工作流；文字、
  滤镜、曲线箭头、独立序号、测量与放大镜提供对应编辑功能。
- 截图正文以及字体搜索、点字号、十六进制颜色、属性步进字段接入原生
  text-input-v3。候选不写业务值，确认和撤销遵循已经冻结的 Qt 对照状态。
- 发布资产直接复用成功远程任务的归档，不重新编译替换。下载后的同一二进制在
  私有 niri、Sway 的层表面和普通窗口中验证 21 个原生输入法场景，另验证两组
  正文光标场景；具体摘要、来源和结果见附带的 `native-preview-verification.json`。
- 远程构建通过 753 项 Rust 测试和 141 项 Python 测试；另有 10 项 Rust 测试忽略。
  格式、严格 Clippy、脚本导入检查、动态链接和许可校验全部通过。

## 已知限制

- 尚未证明 Qt 全界面逐像素一致；字形、部分 Unicode、文字缩放换行、完整鼠标
  选择、字段光标闪烁和独立窗口输入法仍需补齐。
- 亮色主题、完整设置、翻译、上传、扩展、托盘和配置迁移尚未完成。
- Fcitx、IBus 以及完整合成器、输出布局和缩放矩阵尚未验证。
- niri 原生菜单关闭后的首次鼠标移动存在已复现偏差；原版 Qt 同样复现，不通过
  补发鼠标移动掩盖此边界。
- 当前只提供原生 Linux 归档，不发布 AppImage、Deb、RPM、Flatpak、AUR 或 PPA。

完整范围和差异见归档中的 `docs/rewrite-status.md` 及各功能兼容性文档。
