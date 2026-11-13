# 桌面集成

[English](../desktop-integration.md) | **简体中文**

用按键启动截图有两种方式：在合成器中绑定 `mark-shot` 命令，或由 Mark Shot 自行注册全局快捷键。
前者在平铺式合成器上最简单，后者不需要修改合成器配置。

## 在合成器中绑定按键

**niri**（`~/.config/niri/config.kdl`）

```kdl
binds {
    Print { spawn "mark-shot"; }
    Shift+Print { spawn "mark-shot" "--scroll"; }
}
```

**Sway**（`~/.config/sway/config`）

```text
bindsym Print exec mark-shot
bindsym Shift+Print exec mark-shot --scroll
```

**Hyprland**（`~/.config/hypr/hyprland.conf`）

```text
bind = , Print, exec, mark-shot
bind = SHIFT, Print, exec, mark-shot --scroll
```

在 GNOME 和 KDE 上，请在系统的键盘设置中添加自定义快捷键，命令填写 `mark-shot`。

## 内置的全局快捷键与托盘图标

两项功能默认关闭，在设置窗口中启用：

- **设置 → 快捷键**用于启用全局截图快捷键。点击按键字段后按下组合键即可录入，默认值为 `PrintScreen`。
- **设置 → 外观与语言**用于启用托盘图标。托盘菜单提供截图、剪贴板贴图、屏幕录制、设置和退出。

```toml
[desktop]
tray_enabled = true
shortcuts_enabled = true
screenshot_shortcut = "Ctrl+Shift+S"
```

| 平台 | 全局快捷键 | 托盘 |
| --- | --- | --- |
| Linux X11 | 直接注册，按键冲突时报告失败 | StatusNotifierItem |
| Linux Wayland | XDG GlobalShortcuts 门户；桌面可能要求确认，并决定最终的组合键 | StatusNotifierItem |
| Windows | 直接注册，按键冲突时报告失败 | 通知区域 |

设置窗口会显示注册是否成功。补充说明：

- 在 Wayland 上，全局快捷键需要桌面门户实现 GlobalShortcuts 接口。缺少该接口时注册失败，
  可以改用合成器按键绑定。
- GNOME 默认没有托盘区域，需要安装支持 StatusNotifierItem 的扩展，例如 AppIndicator 支持扩展。
- 桌面已占用 `PrintScreen` 时，请选择其他未被占用的组合键。

## 后台服务

启用托盘或全局快捷键后，由一个轻量的后台服务持有注册。打开设置窗口或编辑器时服务自动启动，
窗口关闭后继续运行。在托盘菜单中选择**退出**会停止服务，关闭两个开关也会停止服务。

Mark Shot 不会把自身加入登录启动项。如果希望登录后立即使用托盘和快捷键，请在会话启动时运行服务：

```sh
mark-shot --background
```

以 niri 为例：

```kdl
spawn-at-startup "mark-shot" "--background"
```

两项功能都关闭时，该命令立即退出。

## 编辑器窗口类型

在 Wayland 上，编辑器可以使用 layer-shell 覆盖层，也可以使用普通的全屏窗口：

```toml
[general]
overlay = "auto"    # auto | window | layer
```

| 取值 | 行为 |
| --- | --- |
| `window` | 全屏窗口，可以移动到其他工作区或最小化 |
| `layer` | 截图结束前始终绘制在所有内容上方 |
| `auto` | 在 niri 和 COSMIC 上使用 `layer`，在其他桌面使用 `window` |

`mark-shot --xdg-window` 仅对本次运行强制使用普通窗口。

## 打开图片

桌面入口文件把 Mark Shot 注册为 PNG、JPEG 和 WebP 文件的可选程序，因此在文件管理器中选择
“使用 Mark Shot 打开”即可在编辑器中打开图片。在终端中同样可以：

```sh
mark-shot picture.png
```

## 通知

截图保存成功或失败时，Mark Shot 会发送桌面通知。可以在设置窗口或 `[notifications]`
中关闭通知，也可以用 `--nonotify` 仅对本次运行关闭。
