# 平台支持

[English](../platform-support.md) | **简体中文**

Mark Shot 根据当前桌面实际提供的协议选择后端，不依据发行版或桌面的名称。
获取像素、枚举窗口和保持窗口置顶是相互独立的能力，因此可能出现其中一项可用而另一项不可用的情况。

## 概览

| 环境 | 截图 | 编辑器窗口 | 剪贴板 |
| --- | --- | --- | --- |
| niri、Sway 等 wlroots 合成器 | ext-image-copy-capture、wlr-screencopy，可回退到门户 | layer-shell 或 xdg-shell | ext 或 wlr 数据控制协议 |
| KDE Plasma（Wayland） | KWin ScreenShot2，其次为上述协议 | layer-shell 或 xdg-shell | 数据控制协议，可回退到 Xwayland |
| GNOME（Wayland） | ScreenCast 门户与 PipeWire | xdg-shell | Xwayland 选择区桥接 |
| X11 | XGetImage、RandR，指针使用 XFixes | 原生 X11 窗口 | CLIPBOARD 与 PRIMARY |
| Windows | GDI | 原生窗口 | 系统剪贴板：PNG、DIBV5、Unicode 文本 |

在 Linux 上，会话类型决定使用的协议。Wayland 会话即使设置了 `DISPLAY` 也使用 Wayland，
因为 Xwayland 无法获取 Wayland 桌面的画面。`MARK_SHOT_BACKEND` 和 `MARK_SHOT_CAPTURE`
可以覆盖自动选择，见[配置](configuration.md#环境变量)。

## 功能对照

| 功能 | wlroots、niri | KDE Wayland | GNOME Wayland | X11 | Windows |
| --- | --- | --- | --- | --- | --- |
| 区域与全屏截图 | 支持 | 支持 | 支持，通过门户 | 支持 | 支持 |
| 多显示器截图 | 支持 | 支持 | 支持 | 支持 | 支持 |
| 活动窗口截图（`-w`） | 仅 niri，且必须复制 | 支持 | 不支持 | 支持 | 支持 |
| 框选时吸附窗口 | niri 与 Sway | 不支持 | 不支持 | 支持 | 支持 |
| 界面区域识别 | 支持 | 支持 | 支持 | 支持 | 支持 |
| 贴图置顶 | 需要 layer-shell | 需要 layer-shell | 不支持 | 支持 | 支持 |
| 全局快捷键 | 门户 | 门户 | 门户 | 支持 | 支持 |
| 托盘图标 | 支持 | 支持 | 需要扩展 | 支持 | 支持 |
| 屏幕录制 | 支持 | 支持 | 通过门户 | 支持 | 支持 |

文字输入法在 Wayland 上通过 text-input-v3 支持，在 X11 上通过 XIM 支持。

“界面区域识别”在冻结的画面中查找矩形区域，在所有平台上都可用，但无法判断某个区域是哪种控件。
“吸附窗口”使用桌面提供的真实窗口几何信息，需要针对具体桌面的数据来源。

## 各环境说明

### niri、Sway 与 wlroots 合成器

- 窗口几何信息来自 niri 或 Sway 的 IPC。其他合成器目前没有窗口数据来源，只能使用图像识别和手动框选。
- 在 niri 上，活动窗口截图通过合成器自身的截图动作完成，该动作会写入剪贴板。
  因此完成动作必须包含复制：`mark-shot -w -t c`。
- 在 niri 上，编辑器默认使用 layer-shell 覆盖层，见[桌面集成](desktop-integration.md#编辑器窗口类型)。

### KDE Plasma

- 截图使用 KWin 的 ScreenShot2 接口。在 `[capture]` 中设置 `kde_kwin_screenshot = false`
  后改用桌面门户。
- KDE Wayland 尚未完成完整的交互回归测试。

### GNOME

- 首次截图时门户会弹出对话框，由用户选择显示器。截图成功后授权会被记住。
- GNOME 不提供 layer-shell 和数据控制协议。Mark Shot 使用普通窗口，并通过 Xwayland 复制内容，
  因此剪贴板功能要求 Xwayland 可用。
- GNOME 没有公开接口用于获取活动窗口和窗口几何信息，`-w` 与窗口吸附会报告不支持。
- 托盘需要提供 StatusNotifierItem 的扩展。

### X11

- 截图使用物理像素，不按各显示器的 DPI 重新采样。
- 活动窗口截图包含前台窗口客户区的可见部分。
- 贴图置顶使用 `_NET_WM_STATE_ABOVE`，是否生效由窗口管理器决定。
- 截图过程中的显示器热插拔和混合 DPI 硬件尚未验证。KDE 的 X11 会话和 Xfce 尚未进行针对该桌面的回归测试。

### Windows

- 截图、编辑、剪贴板、保存、贴图、文字识别和录制均已使用 Windows 原生接口实现。
- 窗口吸附使用窗口列表、DWM 边框范围和 UI Automation。
- 目前的验证依赖 Wine 中的自动化运行和交叉编译检查。真实 Windows 设备、混合 DPI
  的多显示器环境以及系统输入法尚未完成全部验证，请把 Windows 版本视为实验性版本。

## 报告平台问题

报告缺陷时请附上以下信息：

```sh
mark-shot --version
echo "$XDG_SESSION_TYPE $XDG_CURRENT_DESKTOP"
mark-shot --list-displays
mark-shot --debug-log /tmp/mark-shot.log     # 重现问题后附上该文件
```
