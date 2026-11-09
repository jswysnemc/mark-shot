# Linux 桌面兼容

Linux 提供原生 X11 与 Wayland 后端。GNOME Wayland 使用门户捕获、普通窗口和 Xwayland 剪贴板桥接；KDE、niri 等桌面按协议能力选择后端。以下记录实现、实际验证与平台限制。

## 协议选择

Linux 入口根据 `XDG_SESSION_TYPE`、`WAYLAND_DISPLAY`、`WAYLAND_SOCKET` 和 `DISPLAY` 选择连接。Wayland 会话同时提供 `DISPLAY` 时仍使用 Wayland，不能通过 Xwayland 根窗口获取整个 Wayland 桌面。

`MARK_SHOT_BACKEND=auto|wayland|x11` 可显式指定界面协议。强制指定后连接失败会报告错误；不会静默切换到另一个显示服务。`MARK_SHOT_CAPTURE` 继续用于选择 Wayland 捕获实现。

| 环境 | 捕获 | 窗口 | 剪贴板 |
| --- | --- | --- | --- |
| X11 | XGetImage、RandR、XFixes 指针 | 原生 X11 覆盖窗口及普通窗口 | CLIPBOARD、PRIMARY、PNG、UTF-8、INCR |
| GNOME Wayland | ScreenCast 门户与 PipeWire | xdg-shell；缺少 layer-shell 时自动回退 | 缺少数据控制协议时使用 Xwayland 选择区 |
| KDE Wayland | KDE 接口，后续按协议能力回退 | layer-shell 或 xdg-shell | ext/wlr 数据控制，缺少时使用 Xwayland |
| niri、Sway 等 | image-copy、screencopy、门户，按实际协议选择 | layer-shell 或 xdg-shell | ext/wlr 数据控制 |

X11 使用物理像素坐标，截图与导出不按显示器 DPI 重采样。支持 16、24、32 位 TrueColor、服务器字节序和扫描行对齐。活动窗口截图使用前台窗口可见客户区，不包含被其他窗口遮挡的内容。

X11 输入使用 XKB 动态布局，支持 Shift、Caps Lock、Ctrl、AltGr、非拉丁布局及 compose/dead key。输入法通过 XIM 交付预编辑、确认文字与候选位置，输入目标关闭后丢弃旧候选。普通窗口支持拖动、八方向缩放、模态窗口和菜单；配置主题变化会触发更新。贴图共用原生窗口业务，支持 OCR、文字选择、翻译及原文/译文切换，复制图片保留原始 PNG。

GNOME 首次捕获需要用户在系统门户选择显示器。授权恢复令牌仅在成功收到图像后保存。首次取帧为授权窗口关闭动画留出 350 毫秒，持续会话后续取帧不重复等待。每路 PipeWire 客户端独立获取门户连接，避免复制同一个套接字后发生协议竞争；多个连接复用同一次授权。

GNOME 剪贴板兼容路径需要桌面提供 Xwayland `DISPLAY` 和选择区桥接。无数据控制协议且没有 Xwayland 时返回明确错误。应用运行时不依赖 `xclip`、`xdotool`、Qt 或 GTK。

## 已执行验证

2026-11-09，在当前工作区执行：

- `cargo check --all-targets --locked` 通过
- `cargo clippy --all-targets --locked -- -D warnings` 通过
- `cargo test --workspace --lib --locked -- --test-threads=4`：主库 1074 项及拼接库 1 项通过，15 项按已有条件忽略
- `cargo fmt --all -- --check`、`git diff --check` 及四个新增 Python 工作流的 Ruff F 检查通过
- Windows MSVC 交叉编译检查通过，共享原生贴图实现可用于 Windows
- 私有 niri：无界面截图成功导出 1280×800 PNG
- 私有 Xvfb：十六项工作流检查通过，包括帧缓冲像素、区域裁切、包含指针、活动窗口客户区、UTF-8 大数据分块传输、PNG 剪贴板、所有权接管退出、剪贴板贴图、设置窗口、编辑器复制、矩形标注导出、连续取帧、滚动面板焦点与像素隔离、阻塞唤醒、超时及 XIM 候选/确认/回退/旧目标隔离
- X11 原生贴图：翻译交接、拖动后菜单定位、原图复制、原文/译文选择和菜单外部关闭通过；独立 Xvfb 与 Openbox 窗口管理器均执行验证
- X11 滚动截图：真实滚轮输入、连续拼接、暂停与复制通过，320×512 输出与独立期望图逐像素一致
- 私有 Openbox：十二项基础工作流通过，覆盖带窗口管理器的捕获、普通窗口、编辑器和剪贴板
- 私有 GNOME Shell 51.1：单显示器及双显示器工作流通过，包含输出枚举、设置窗口、门户授权与恢复、全屏编辑器、跨 Xwayland/Wayland 文本和 PNG 剪贴板、剪贴板贴图与原图复制；双屏导出 2048×768 图像。每块屏幕的独立颜色采样一致，覆盖首帧授权过渡及屏幕位置匹配
- 私有 KDE KWin Wayland：两个虚拟输出枚举及 1600×600 全屏捕获通过

X11 可复现命令：

```sh
cargo build --bin mark-shot --example scroll-page --locked
cargo test --test x11_backend --test x11_input --no-run --locked
uv run --no-project --python /usr/bin/python3 python scripts/x11-desktop-workflow.py
uv run --no-project --python /usr/bin/python3 python scripts/x11-pin-workflow.py
uv run --no-project --python /usr/bin/python3 python scripts/x11-scroll-workflow.py
```

完整十六项基础验证需要将构建输出的 `x11_backend-<hash>`、`x11_input-<hash>` 可执行文件路径分别传给基础脚本的 `--backend-tests`、`--input-tests` 参数。基础与贴图脚本支持 `--wm /path/to/openbox`。脚本自行启动私有 Xvfb、清除宿主 Wayland 变量、固定配置并在退出时回收所启动进程，结果保存在标准输出及测试目录 `result.json`。依赖 Xvfb、xdotool、xclip、xmessage、Python Pillow；这些仅是验证依赖。

使用临时解压的 Openbox 时，还需把其库目录加入 `LD_LIBRARY_PATH`，把共享数据目录加入 `XDG_DATA_DIRS`。脚本同时检查管理器进程存活，避免主题缺失导致管理器退出后，残留的根窗口属性造成误判。

GNOME 可复现命令：

```sh
uv run --no-project --python /usr/bin/python3 python scripts/gnome-desktop-workflow.py --monitors 1
uv run --no-project --python /usr/bin/python3 python scripts/gnome-desktop-workflow.py --monitors 2
```

GNOME 脚本创建私有 D-Bus 会话、无头 GNOME Shell 和 PipeWire 服务，需要 GNOME Shell、Mutter、GNOME 门户、PipeWire、WirePlumber、wl-paste、Python D-Bus、PyGObject、AT-SPI 与 Pillow。可用 `--shell` 指定独立安装的 GNOME Shell；本次验证使用临时运行时及软件渲染。Shell 调试与测试输入仅作用于脚本启动的私有会话。

## 平台限制与尚未覆盖的验证

- GNOME 原生 Wayland 活动窗口识别及精确活动窗口截图缺少公开通用接口，明确报告不支持；Xwayland 根窗口不能代替 Wayland 桌面
- X11 混合 DPI 不改变物理像素捕获；物理显示器热插拔、捕获过程中 RandR 布局变化及混合 DPI 硬件尚未验证
- KDE X11、Xfce、旧版 GNOME Xorg 尚未分别执行桌面专属回归；Openbox 验证不代表这些窗口管理器的全部行为
- KDE Wayland 尚未执行完整交互回归，GNOME 多显示器滚动截图也尚未覆盖
- 已验证应用配置主题同步；外部桌面主题切换、真实输入法候选窗口及实际 OCR/翻译服务仍需按用户环境核对

私有会话验证不替代上述硬件及桌面版本测试，未验证场景不列为已通过。
