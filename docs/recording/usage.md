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
| 支持 ext-image-copy / wlr-screencopy 的 Wayland 桌面 | 进程内持久协议会话；兼容时 GBM DMA-BUF → VAAPI，其他情况共享内存 → VAAPI/NVENC/OpenH264 | GPU 路径要求同设备、支持格式和普通输出方向；旋转、格式或导入不兼容时使用 CPU 捕获；硬件模式不静默改成软件编码 |
| 其他 Wayland 桌面 | 自持 Portal 授权会话与持续 PipeWire 共享内存流 | 指定显示器必须匹配授权来源位置和逻辑尺寸；区域还需匹配实际物理像素尺寸，否则拒绝裁剪；授权可取消 |
| X11 | MIT-SHM 1.2，共享段不可用时 GetImage；VAAPI/NVENC 或 OpenH264 | 不需要硬件 OpenGL，纯软件 Xvfb 已验证；RandR 输出身份、位置或尺寸变化时停止 |
| Windows | WGC/D3D11、GPU NV12 转换、Media Foundation H.264/AAC | 自动模式初始化失败或首帧提交失败时回退 GDI 和系统软件编码；已有视频帧后不重启覆盖，显式硬件模式不回退 |

当前覆盖单显示器或其区域，不拼接多个显示器。音频选择系统声音或默认麦克风，不同时混音。
Linux 优先原生 PipeWire，连接不可用且尚未收到样本时尝试原生 PulseAudio；不会启动录音命令或音频服务。
音视频共用单调时间，暂停清理旧缓冲，采集间断按时间戳补静音，较长断流明确报错。
基线为 SDR、8 位、4:2:0，不宣称 HDR 支持。画面尺寸变化时需重新开始。关闭控制窗口先请求停止并等待保存。

“较小文件”降低码率目标，不保证 CPU 占用更低。实际资源消耗取决于分辨率、帧率、内容、捕获协议和编码器。
DMA-BUF 路径保持像素在显存中，编码前完成 GPU 裁剪和 RGB→NV12 转换，并等待输入读取结束再复用捕获缓冲。
共享内存路径仍有 CPU 颜色转换和可能的硬件上传，不能称作免回读。
静态画面延续时间轴，停止或暂停时保留准确的最后显示时刻；首帧未到达不会显示已开始录制。

## 媒体库与开发构建

完整包携带 `media/libmarkshot-media.so`（Linux）或 `media/markshot-media.dll`（Windows），录制和 MKV/GIF 导出在进程内完成。
不包含或调用 wf-recorder、GPU Screen Recorder、FFmpeg、FFprobe 可执行程序，不在运行时下载录制组件。
系统图形驱动、Wayland/X11、PipeWire 和可选 PulseAudio 仍由操作系统提供。
媒体库使用固定 FFmpeg 9.0 最小 LGPL 配置，Linux 软件编码使用 OpenH264 2.6.0；不启用 GPL、nonfree 或 libx264。

执行 `bash scripts/media/build-libraries.sh linux`；Windows 参数为 `windows`，需要 MinGW 交叉工具链。
把 `target/media-<平台>/` 放到可执行文件旁的 `media/`。`cargo build` 本身不构建媒体库。
开发测试可以设置绝对路径 `MARK_SHOT_MEDIA_LIBRARY`；`MARK_SHOT_RECORDING_CAPTURE=portal` 可强制验证 Portal。
Windows 实时 MP4 仍使用系统 Media Foundation，仅额外格式导出加载私有媒体库。

库旁附带许可证、来源、构建配置、摘要和可独立重建的源码归档。源码归档去除 OpenH264 测试视频，保留所有库源码及构建文件。
解压源码归档后直接运行同一 `scripts/media/build-libraries.sh` 即可重建；有校验清单时不重新下载源码。
`scripts/media/build-validation-tools.sh` 仅为 CI 构建独立解码器，其产物不进入发布包。

模块与资源所有权见[进程内架构](native-architecture.md)。源码调研与路径选择依据见[录制恢复调研](../recording-research.md)。构建和实际验证范围见[验证记录](validation.md)。
