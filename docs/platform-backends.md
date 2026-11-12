# 平台后端与窗口识别边界

本项目当前实现 Windows 与 Linux X11/Wayland。窗口枚举、UI 区域识别、像素捕获和置顶
属于独立能力；能截图不代表能读取所有窗口几何或控制窗口层级。

## 窗口与 UI 识别

| 环境 | 窗口几何来源 | 控件/区域来源 | 边界 |
| --- | --- | --- | --- |
| Windows | `EnumWindows`、DWM 可见边框，失败时 `GetWindowRect` | 有界 UI Automation 查询及共用图像算法 | 隐藏、最小化、cloaked 和穿透窗口过滤；自绘控件、权限边界仍可能无语义信息 |
| Linux X11 | `_NET_CLIENT_LIST_STACKING`，缺失时 `QueryTree`；`GetGeometry` 和 `TranslateCoordinates` | 共用图像边缘/轮廓分析 | 按当前桌面、可见状态与层级过滤；目前没有 AT-SPI 控件语义实现 |
| niri Wayland | niri IPC 的窗口、工作区、输出、图层和布局信息 | 共用图像分析 | 优先精确 tile 几何，旧版本需要按布局推算 |
| Sway Wayland | IPC 可见工作区的平铺和浮动树 | 共用图像分析 | 500 毫秒总期限，失败后保留手动框选 |
| GNOME/KDE/Hyprland 等其他 Wayland | 当前没有接入窗口快照适配器 | 可捕获画面上的共用图像分析 | Portal 和截图协议不提供通用全局窗口枚举；不能宣称精确窗口吸附已支持 |

入口为 `src/backend/window_detection/mod.rs`，快照在截图覆盖层映射之前取得。
桌面逻辑坐标、输出原点、像素尺寸及缩放分别保留，不能用主屏比例处理全部显示器。
图像算法只计算一次，悬停和滚轮查询缓存候选；无法识别时使用手动框选。

X11 上应直接连接真实 X server，而不是把 XWayland 的窗口集合当成整个 Wayland 桌面。
当前 `Session` 根据显式后端选择和会话变量作出选择，不在 Wayland 失败时偷偷改用 XWayland。
协议测试要覆盖重父窗口、负坐标、多输出、工作区、置顶窗口、遮挡、最大化及不同 DPI；
再用 Cinnamon/Muffin、KWin、Xfce 等真实窗口管理器验证。Xvfb 只能验证其中的协议和像素行为。

UI 的矩形识别不等于识别“按钮”“输入框”等语义。若要求后者，Linux 需要另外接入 AT-SPI，
并保留图像和手动框选；AT-SPI 也不能保证每个自绘应用都会公开控件。
现有像素算法和限制详见 [窗口与 UI 区域识别对照](region-detection-compatibility.md)。

## 捕获与录制接口

截图入口在 `src/backend.rs`：

- `CaptureMethod::capture_frame(outputs)` 返回按输出标识关联的 `CaptureResult`，保留宽、高、步长、位置和像素
- `CaptureMethod::open_session(outputs)` 建立持续会话，`CaptureSession::next_frame()`/`close()` 复用并释放资源
- `capture_active_window()` 是独立能力，未支持时返回错误，不用整屏冒充窗口
- `ScreenOverlay` 负责编辑覆盖层、输入及显示，不负责媒体编码

| 平台 | 截图 | 录制 |
| --- | --- | --- |
| Windows | GDI 捕获、BGRA 转 RGBA，原生显示器坐标 | WGC → D3D11/NV12 → Media Foundation H.264；软件路径为 GDI → CPU BGRA → 系统编码器；音频 WASAPI |
| X11 | RandR 输出发现、XGetImage、XFixes 光标 | 进程内 MIT-SHM / GetImage → VAAPI/NVENC/OpenH264，无硬件 OpenGL 要求 |
| Wayland | 按实际协议选择 KDE、ext-image-copy、wlr-screencopy 或 Portal/PipeWire | 进程内 ext-image-copy / wlr-screencopy，兼容时 DMA-BUF → VAAPI；否则 SHM；其他桌面 Portal/PipeWire SHM |

录制入口为 `features/recording/backend::run(config, target, path, commands, status)`。
`Target` 包含显示器标识和相对物理裁剪区域，命令为暂停、继续、停止。UI 只接收状态，
原始视频留在后端，不通过 UI 逐帧传输。

Portal 预选显示器通过返回位置与逻辑尺寸匹配，预选区域还要求实际流的物理尺寸匹配。
缺少元数据或不匹配时拒绝套用坐标；默认完整来源直接使用授权结果。
后端按实际协议能力和编码器初始化结果选择，不把发行版名称作为能力保证。
录制数据与资源契约见 [进程内架构](recording/native-architecture.md)。
音视频封装通过 `crates/shot-media` 的私有 C ABI，库可单独替换，不跨 ABI 暴露 FFmpeg 内部结构。
Linux 不需要下载或启动 wf-recorder。wf-recorder 的适用范围由 compositor 捕获协议决定，
并不是某份发行版名单；本项目仅参考其协议和缓冲策略。

## 贴图置顶

- Windows：原生 `SetWindowPos(HWND_TOPMOST)`，关闭置顶使用 `HWND_NOTOPMOST`
- X11：向窗口管理器发送 EWMH `_NET_WM_STATE_ABOVE`，由窗口管理器执行堆叠请求
- Wayland：支持 layer-shell 时使用 `Top` 图层；普通 xdg-shell 没有通用强制置顶请求，缺失能力时按普通窗口运行

置顶贴图发起扫码时，结果窗口使用 `Overlay` 图层，避免被原图的 `Top` 图层遮挡；
普通扫码窗口仍使用普通窗口角色。贴图在 layer-shell 下拖动时固定输入表面，仅移动无输入
预览层，释放后再提交真实位置，避免将移动后的局部坐标累加进拖动距离。
