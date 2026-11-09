# 全局截图、系统托盘与设置颜色

## 启用与生命周期

在 `mark-shot --settings` 的「快捷键」页启用全局截图快捷键，点击按键字段录入单个组合。
该组合独立于编辑器内部快捷键；录入期间暂停全局监听，确认或取消后恢复。
「禁用」关闭全局监听，「恢复」恢复默认组合。
在「外观」页启用系统托盘图标。两个功能分别保存并更新注册；调整托盘不会重建快捷键会话。

```toml
[desktop]
tray_enabled = true
shortcuts_enabled = true
screenshot_shortcut = "Ctrl+Shift+S"
```

两个功能默认关闭，默认组合为 `PrintScreen`。启用时自动启动独立后台服务，关闭设置或编辑器后继续运行。
同一配置和显示会话仅运行一个服务。托盘菜单提供截图、剪贴板贴图、设置及退出；退出会注销按键和托盘，
保留配置，下次启动图形功能时重新注册。关闭两个开关也会结束服务。

`mark-shot --background` 可单独启动已配置的服务，也可用于桌面自启项；程序不自动修改登录启动项。
命令本身保持运行，两个功能都关闭时立即退出。`--config` 可用于隔离另一套配置。

## 平台行为

| 平台 | 全局截图快捷键 | 托盘 |
| --- | --- | --- |
| Linux X11 | 原生 XGrabKey；支持 Caps Lock、Num Lock 和 Scroll Lock；冲突报告失败 | StatusNotifierItem 与 DBusMenu |
| Linux Wayland | XDG GlobalShortcuts 门户；系统可能要求确认并决定最终组合 | StatusNotifierItem 与 DBusMenu |
| Windows | RegisterHotKey；冲突报告失败 | Shell_NotifyIconW；Explorer 重启后重新注册 |

设置显示注册中、成功、失败、暂停或后台服务停止状态。Wayland 显示系统实际批准的组合，
缺少门户时报告失败，不借助 Xwayland 假装完成全局注册。应用将系统激活令牌交给新窗口请求焦点。
GNOME 默认没有托盘区域，需要支持 StatusNotifierItem 的扩展；托盘宿主缺失时显示等待，宿主恢复后重新登记。
系统已占用 `PrintScreen` 时，应在设置中选择空闲组合。

独立二进制首次启用 Wayland 全局键时，会在用户数据目录补充隐藏的应用身份文件和图标，
以便门户识别 `io.github.jswysnemc.MarkShot`。已有桌面身份文件保持不变，该文件不属于登录自启项。

## 颜色设置

默认标注颜色和贴图边框颜色显示真实色块及 RGBA 数值。透明色使用棋盘背景，
点击后使用标注工具同一套取色面板、HSV 轨道、透明度轨道和颜色板。
十六进制支持 `#RRGGBB` 与 `#RRGGBBAA`；确认保存，取消不修改，恢复删除显式配置并继承默认颜色。

## 验证

隔离 X11 与私有 D-Bus 工作流：

```sh
cargo build --bin mark-shot --locked
uv run --no-project --python /usr/bin/python3 python scripts/desktop-integration-workflow.py
uv run --no-project --python /usr/bin/python3 python scripts/desktop-settings-workflow.py
uv run --no-project --python /usr/bin/python3 python scripts/gnome-desktop-workflow.py --desktop
```

依赖 Xvfb、xdotool、Python D-Bus、PyGObject 和 Pillow，仅用于测试。
2026-11-09 已执行验证：

- 隔离 X11 与私有 D-Bus：14 项检查通过，包括真实按键、锁定修饰键、旧键注销、冲突恢复、录入暂停、服务单实例、托盘图标、菜单与退出注销
- 设置原生窗口：6 项检查通过，覆盖实际色块、共用颜色窗口、RGBA 输入保存、取消、恢复和桌面设置项，并保存帧缓冲截图
- 私有 GNOME Shell 51.1：6 项检查通过，覆盖系统授权、真实全局键截图、窗口焦点、缺少托盘宿主状态、修改托盘不重建按键、退出后门户会话释放
- 工作区库测试：1078 项通过，15 项按已有条件忽略；新增设置用例包含实际色块像素断言
- Linux 与 Windows MSVC 的全部目标 Clippy 检查通过，使用 `-D warnings`；格式与补丁空白检查通过

GNOME 工作流还需要 GNOME Settings 提供的 GlobalShortcutsProvider、门户、PipeWire 和 AT-SPI；
测试运行在独立 D-Bus 与无头合成器内，批准的是私有测试会话。
未安装桌面身份时使用 [主机应用登记接口](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.host.portal.Registry.html)，
登记须先于该连接上的其他门户调用。

Windows 实现通过 MSVC 交叉编译检查；本次未在真实 Windows 通知区域执行交互验证。
