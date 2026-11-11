# 屏幕录制

从托盘“屏幕录制 → 开始录制”或 `mark-shot --record` 打开控制窗口。选择格式、音频、质量、帧率和倒计时后点击“开始录制”。“选择区域”进入截图选区界面，选区必须位于一个显示器内。录制控制窗口与截图编辑器分别运行，录制期间不保留截图编辑器的像素缓存。

## 控制与保存

- `--record-select`：直接选择录制区域
- `--recording-status`：输出当前会话状态 JSON
- `--pause-recording`、`--resume-recording`：暂停、继续
- `--stop-recording`：停止并保存，完成后状态变为 `finished`
- 同一配置和显示会话只允许一个录制控制窗口；命令行控制需使用相同 `--config`

设置页“屏幕录制”可配置保存目录和编码器。留空目录使用系统视频目录中的 `Mark Shot`。MP4 使用 H.264；有音频时使用 AAC。MKV 在停止后无损重封装；GIF 在停止后进行调色板转换，不包含音频。暂停时间不计入成品时间轴。倒计时中停止会取消本次录制。

成品只在正常收尾后发布，不覆盖已有文件。失败时窗口显示原因与恢复目录，工作视频保留在保存目录的隐藏 `.Recording_*.recording` 子目录。Linux 使用分片 MP4 提高异常退出后的恢复机会；Windows MP4 依赖 Media Foundation 正常收尾，不能保证强制终止后的文件可恢复。

## 平台路径

| 平台 | 捕获和编码 | 限制 |
| --- | --- | --- |
| Niri、Sway 等支持 wlr-screencopy 的 Wayland 桌面 | 随包 wf-recorder；自动尝试 VAAPI，启动失败后使用 libx264 | 支持区域、指针和单路音频；VAAPI 可用性取决于驱动，NVIDIA 设备可能使用软件回退 |
| 其他 Wayland 桌面 | Portal/PipeWire 与随包 GPU Screen Recorder | 系统授权窗口选择来源；区域录制必须选对应显示器；旋转来源的区域裁剪会明确报错 |
| X11 | 无特权 XImage 捕获，GPU Screen Recorder 编码 | 捕获含 CPU 像素搬运；需要硬件 OpenGL，软件编码也不支持纯 llvmpipe；不安装 KMS 特权辅助程序 |
| Windows | WGC/D3D11、GPU NV12 转换、Media Foundation H.264/AAC | 自动模式初始化失败时回退 GDI 和系统软件编码；显式硬件模式要求确认硬件编码器 |

当前覆盖单显示器或其区域，不拼接多个显示器；Portal 可额外提供窗口来源。音频选择系统声音或默认麦克风，不同时混音。基线为 SDR、8 位、4:2:0，不宣称 HDR 支持。画面尺寸变化时需重新开始。关闭控制窗口会先请求停止并等待保存。

“较小文件”降低编码质量目标或码率，不保证 CPU 占用更低。实际资源消耗还取决于分辨率、帧率、内容、捕获协议和硬件编码器。GPU 路径避免主程序搬运原始帧；软件回退会增加 CPU 工作。

## 组件与开发构建

完整 CI 包含 `recording/`，请与主程序一同保留。Linux 包含 wf-recorder、GPU Screen Recorder、裁剪 FFmpeg/FFprobe；Windows 实时录制使用系统组件，随包媒体工具负责额外格式导出。对应源码、补丁、构建参数、许可证和 SHA256 摘要位于该目录。

开发构建可执行 `bash scripts/recording/build-components.sh linux`，Windows 媒体工具使用参数 `windows` 并需要 MinGW 交叉工具链。把 `target/recording-components-<平台>/` 放到可执行文件同目录的 `recording/`。单独 `cargo build` 不构建媒体组件。FFmpeg 与第三方依赖版本固定在脚本中，构建需要对应开发库和网络。

源码调研与路径选择依据见[录制恢复调研](../recording-research.md)。构建和实际验证范围见[验证记录](validation.md)。
