# 屏幕录制

[English](../recording.md) | **简体中文**

Mark Shot 可以把一个显示器或其中的一块区域录制为 MP4、MKV 或 GIF。
采集和编码都在程序内部完成，不启动外部录制程序。

从源码构建时需要先构建媒体库，见[安装说明](installation.md#录制媒体库)。安装包已包含媒体库。

## 开始录制

| 入口 | 方法 |
| --- | --- |
| 命令行 | `mark-shot --record` 打开控制窗口；`mark-shot --record-select` 直接进入区域选择 |
| 托盘 | 屏幕录制 → 开始录制 |
| 编辑器 | 选择录屏工具，或使用 `Ctrl+Shift+R` 录制视频、`Ctrl+Shift+G` 录制 GIF |

在控制窗口中选择格式、音频来源、质量、帧率和倒计时，然后点击**开始录制**。
**选择区域**用于框选录制范围，该范围必须位于同一个显示器内。

编辑器中的录屏工具保留当前选区，在小工具栏中显示格式、帧率、倒计时、声音、画质、编码器和光标参数。
选择工具不会开始采集；点击**开始录制**或按 `Enter` 才提交区域与参数。
这些参数只用于本次录制，不改变默认设置。

## 控制录制

| 动作 | 截图界面或控制面板 | 命令 |
| --- | --- | --- |
| 暂停、继续 | 暂停按钮，`Ctrl+Shift+P` | `mark-shot --pause-recording`、`mark-shot --resume-recording` |
| 停止并保存 | 停止按钮，`Ctrl+Shift+E` | `mark-shot --stop-recording` |
| 查询状态 | | `mark-shot --recording-status` 输出 JSON |

开启**设置 → 屏幕录制 → 持续显示录制控制面板**后，录制期间会显示紧凑面板，提供时长、暂停或继续、停止和隐藏按钮。
该选项默认关闭，下次录制生效。确认开始后，准备窗口关闭。在 Wayland layer-shell 会话中，拖动面板时通过帧回调预览层实时移动，
松开鼠标后再提交最终坐标，避免边距更新往返造成的抖动与滞后。

点击隐藏、在面板内按 `Esc` 或关闭面板都不会停止录制。隐藏后显示四秒提示，说明可以再次进入截图界面，
或通过托盘暂停、继续和停止录制；关闭桌面通知时仍会显示该提示。表格中的快捷键用于截图界面。

暂停的时间不计入成品。倒计时期间停止会取消本次录制。
同一配置和桌面会话只能进行一个录制；控制命令必须使用与录制相同的 `--config`。

## 输出

| 格式 | 视频 | 音频 | 说明 |
| --- | --- | --- | --- |
| MP4 | H.264 | AAC | 录制过程中直接写入 |
| MKV | H.264 | AAC | 停止后重新封装，不重新编码 |
| GIF | 调色板 | 无 | 停止后转换 |

未配置其他目录时，文件保存到系统视频目录中的 `Mark Shot` 文件夹。程序不会覆盖已有文件。

录制失败时，控制窗口显示原因，工作视频保留在输出目录下隐藏的 `.Recording_*.recording`
文件夹中，以便恢复。

## 设置

可以在设置窗口的**屏幕录制**页面或配置文件中修改：

```toml
[recording]
format = "mp4"          # mp4 | mkv | gif
quality = "balanced"    # balanced | high | efficient
encoder = "auto"        # auto | hardware | software
fps = 30
gif_fps = 12
countdown = 3           # 秒
audio = "off"           # off | system | microphone
cursor = true
keep_controls_visible = false
directory = ""          # 留空使用视频目录
```

音频只能选择系统声音或默认麦克风，两者不会混合。`efficient`（界面显示为“较小文件”）
降低码率目标，但不一定降低 CPU 占用。

## 各平台的实现

| 平台 | 采集 | 编码 |
| --- | --- | --- |
| 支持 ext-image-copy-capture 或 wlr-screencopy 的 Wayland 桌面 | 进程内协议会话；GPU 路径兼容时使用 DMA-BUF，否则使用共享内存 | VAAPI、NVENC，或 OpenH264 软件编码 |
| 其他 Wayland 桌面 | 桌面门户与 PipeWire 流 | VAAPI、NVENC 或 OpenH264 |
| X11 | MIT-SHM，不可用时回退到 GetImage | VAAPI、NVENC 或 OpenH264 |
| Windows | Windows Graphics Capture 与 Direct3D 11，可回退到 GDI | Media Foundation H.264 与 AAC |

## 限制

- 只能录制一个显示器或其中的区域，不会把多个显示器合并为一个视频。
- 支持 SDR、8 位、4:2:0，不支持 HDR。
- 显示器布局或画面尺寸变化时录制停止，需要重新开始。
- 设置 `encoder = "hardware"` 后，硬件编码失败时程序报告错误，不会自行改用软件编码。
  `auto` 会选择可用的编码器。
- 在使用门户的桌面上，授权的录制来源必须与所选显示器一致；若桌面门户未返回坐标（如 KDE Plasma 6.3），程序会按逻辑或物理尺寸核对授权来源，来源不匹配时仍会拒绝裁剪区域，避免录到错误的范围。
- 当当前 GPU 驱动或格式无法直接导入 DMA-BUF 显存帧时，录制会自动回退到 CPU 帧配合硬件上传（`encoder = "auto"` 模式下若硬件路径首帧仍失败，再回退至软件编码）。
- 在 Windows 上，被强制终止而中断的 MP4 可能无法恢复。
- Windows 支持采集排除时，控制面板与隐藏提示不会进入采集画面。Linux 屏幕采集会包含可见窗口；
  如需成品不包含面板，可隐藏面板或把它移到录制区域之外。
