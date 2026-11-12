# 录制与截图界面需求核对

2026-11-12，当前分支 `feat/native-barcode-scanning`。原 UI 改动已通过 `e1042cf` 提交，并通过 `7f8018a` 合并到当前分支；本轮继续补齐同一份需求。

## 原始资料

在本机 Claude Code 会话 `0006bcc1-a594-41ca-9037-22f810e502eb` 中找到了提示词和两张原图。对应消息时间为 `2026-11-11T21:33:33.199Z`，消息编号为 `bcbcc099-7ba8-4a55-8245-9abbdadc98ee`。

- [原快捷键图片](../images/recording-ui/reference-shortcuts.png)：仅六条提示，包括拖动、全屏、确认、退出、上一选区和下一选区。
- [原录制设置图片](../images/recording-ui/reference-recording.png)：范围、格式、音频、质量、帧率、倒计时和指针同时铺成按钮组。
- [本轮修改前的截图界面](../images/recording-ui/capture-before.png)。
- [本轮修改前的录制设置](../images/recording-ui/setup-before.png)。该实拍使用 Niri 默认平铺布局，窗口由合成器放大；不能据此推断代码默认尺寸。

原会话确实修改了起始界面、录制窗口和统一控件，但结束时没有留下可核验的最终完成记录。评估以当前代码、修改前实拍和本轮流程验证为依据，没有把会话中的计划当作已实现功能。

## 完成情况

| 需求 | 找回改动时的状态 | 本轮补齐后的行为 |
| --- | --- | --- |
| 增加快捷键提示 | 已有录制、放大、主题、历史和设置动作，但常见小屏会裁掉后半组 | 常见界面显示十二条真实提示；区域分层识别可用时额外显示滚轮提示。键位来自有效绑定，增加工具栏切换提示，低逻辑高度收紧行距 |
| 优化紧凑录制设置 | 已从原图改为 400×372 分段表单，仍把参数同时铺在首屏 | 默认 400×300，仅显示范围、格式、声音和倒计时；画质、帧率、指针按需展开到 400×396。每行只显示当前值，开始录制是唯一实心主按钮 |
| 开始后关闭设置窗口 | 已实现 | 保留并实测确认；原生窗口尺寸请求在 Wayland、X11 与内容布局之间同步 |
| 按需显示倒计时小组件 | 已有 272×56 小组件，但没有绑定对外状态 | 倒计时发布格式和剩余秒数，截图界面可查询和取消。取消返回设置，不创建成品；小组件销毁后才开始采集 |
| 后台持续录制 | 已实现 | 保留并实测确认；录制阶段没有设置窗口或独立控制窗口 |
| 进入截图界面后显示状态 | 起始界面可显示，选区完成后停止查询并隐藏 | 完成截图选区后继续显示 360×38 的录制栏，包含类型、阶段、时长、暂停或继续、停止保存；按显示器避开编辑工具栏，移动时清理旧位置 |
| 简单控制录制与保存 | 起始阶段已有控制，选区完成或切换部分工具后失效 | 鼠标和快捷键均可控制，独立于截图工具、识别和上传任务；倒计时阶段显示取消，保存阶段锁定重复停止 |

## 验证方式

自动验证入口为 [ui-workflow.py](../../scripts/recording/ui-workflow.py)，使用独立 Xvfb、Niri、消息总线、配置、录制目录和输入设备。验证不连接用户当前录制会话。

流程覆盖展开与收起、外部缩小窗口后自动收起高级区、GIF 倒计时取消、重新选择 MP4、后台录制、截图选区完成后的暂停与继续、停止保存。控件位置由真实布局日志确认，避免录制栏遮挡编辑工具栏。暂停期间确认有效时长不增长；成品必须同时满足状态为 MP4、文件后缀为 `.mp4`、完整解码无错误。首帧额外检查倒计时小组件没有进入画面。

最终发布构建在两组隔离输出中均通过完整流程，结果见 [验证数据](ui-workflow-validation.json)。两个 MP4 均完整解码四十帧，帧时间戳严格递增；暂停期间有效时长保持不变。正常尺寸下录制栏按钮中心位于逻辑纵坐标 77，1.5 倍缩放下位于 109，均避开编辑工具栏。

| 检查 | 结果 |
| --- | --- |
| 工作区测试 | 87 组，1607 通过、0 失败、34 忽略 |
| Linux 全目标 Clippy | 通过，按零警告要求检查 |
| Windows MSVC 全目标检查 | 通过 |
| Cargo 格式与 Python Ruff 检查 | 通过 |
| 1270×720、缩放 1.0 | 完整流程通过，生成 MP4 |
| 853×533、缩放 1.5 | 完整流程通过，生成 MP4 |

被测程序为 `target/release/mark-shot`，两组流程使用相同 SHA-256：

```text
dc1ed6fcf767a58bd676680bf06b636efc9de0099ef23d59c2cbd2aed1b01f52
```

## 最终实拍

- [常用参数表单](../images/recording-ui/setup-compact.png)、[高级参数展开](../images/recording-ui/setup-expanded.png)。
- [倒计时小组件](../images/recording-ui/countdown-widget.png)、[截图界面的倒计时状态](../images/recording-ui/countdown-status.png)。
- [起始界面与十二条快捷提示](../images/recording-ui/recording-intro.png)。
- [选区完成后继续显示录制控制](../images/recording-ui/recording-selected.png)、[暂停后的控制栏](../images/recording-ui/recording-paused.png)。
- [1.5 倍缩放下的快捷提示](../images/recording-ui/scale-15-recording-intro.png)、[1.5 倍缩放下的暂停界面](../images/recording-ui/scale-15-recording-paused.png)。
- [首帧录制画面](../images/recording-ui/first-recorded-frame.png)：不包含倒计时小组件。

本地录制成品分别保存在 `/home/snemc/.cache/ms-ui-delivery-1/videos/` 和 `/home/snemc/.cache/ms-ui-delivery-15/videos/`。截图和验证摘要已放入仓库，视频保留在本地验证目录。

## 平台范围

Linux 真实界面验证采用隔离 Niri 环境。录制设置的初始尺寸与高级区尺寸在浮动窗口中验证；平铺窗口的最终大小由合成器决定，内容在高度不足时自动收起高级区。

Windows 使用 MSVC 目标交叉编译检查。本轮没有实际 Windows 设备，不能把交叉编译成功当作 GPU 或 Media Foundation 录制的实机验证。
