# 桌面兼容性修复验证

本次修复覆盖 Windows 启动、截图取消、贴图边框、OCR 与快捷截图准备，
以及 X11 鼠标积压、分数缩放、Wayland 图片格式和 RPM 稳定版依赖。

## 根因与修正

| 问题 | 根因与修正 |
| --- | --- |
| Windows 双击出现控制台或立即截图 | 使用 Windows GUI 子系统，无参数进入后台；显式 `--capture` 保留截图入口 |
| Windows Escape 不能取消待选截图 | 首帧主动申请焦点，截图期间临时注册 Escape，隐藏和销毁时注销 |
| 贴图边框设置失效 | 原生贴图预览没有应用保存的边框设置；新增预览装饰，四边在内部绘制，原图保持独立 |
| Windows 系统 OCR 慢或失败 | 默认推荐本地模型；系统后端复用引擎、回退已安装语言、小选区放大、十五秒超时取消 |
| Windows 空闲后快捷截图启动慢 | 后台准备一次性待命进程及字体，触发后才初始化最新配置；截图退出后替换待命进程 |
| X11 选区跟随滞后 | 绘制后逐个处理积压位置；现在有界合并连续移动，保留轨迹和按钮、键盘顺序 |
| X11 分数缩放无效 | 从 Xft.dpi/GDK 读取比例，输入、窗口、菜单、输入法和面板统一使用逻辑坐标与物理缓冲 |
| 非整除缩放增加边缘像素 | X11/Windows 保留原生网格和精确逻辑边缘；Wayland 保留原有缓冲契约与不兼容回退 |
| VirtualBox 接收不到图片 | Wayland 来源只提供 PNG；新增按需 BMP 与 x-bmp，缓存匿名文件并保留原 PNG |
| RPM 要求 OpenSSL 4 / glibc 2.44 | 不再使用滚动的 Fedora latest；固定 Fedora 43，实际产物检查 OpenSSL 3 / glibc ≤2.42 |
| PR #162 缩放后的阴影偏移错误 | 阴影偏移统一换算到窗口单位，使用非透明阴影验证不同绘制比例下的等价结果 |

待命进程不会预先固定配置或保存的标注偏好。准备线程就绪等待最多十秒，失败回收子进程；动作线程不等待准备，未就绪时立即使用普通启动；
任务开始之后发生错误不会自动重复截图。待命期间不捕获屏幕，不进行周期绘制。

## 已执行验证

- Rust 工作区 1705 项通过，包含主库 1291 项；另有需要特定环境的忽略项。
- Linux 和 Windows 全目标 Clippy，开启 `-D warnings`。
- Python 验证工具 238 项、打包合同测试 69 项通过。
- 私有 Xvfb 在 1280×800 和 1366×768 下验证 125%、175% 与 110 DPI 的真实截图场景。
- X11 真实事件验证连续位置使用最新采样，按下、移动、松开保持坐标和分派顺序。
- 1366×768、175% 全屏导出逐字节一致，没有多出的透明列或丢失的末列。
- 私有 Wayland 会话实际报价 PNG、BMP、x-bmp；各格式重复接收两次，核对尺寸与像素。
- 私有 Wine 窗口验证 GUI 子系统、四边配置颜色、原图中心像素，以及其他贴图持有焦点时取消截图。
- Windows Paddle 使用官方摘要验证的 ONNX Runtime 1.28.0，12、16、24 像素中英文样本逐字匹配，原图坐标有效。
- 私有 Wine 的合成 WM_HOTKEY 动作到窗口显示为 543 毫秒，静默十五秒后为 385 毫秒；不等待未就绪的准备任务。

主要证据保存在 `target/desktop-*.log`、`target/desktop-x11-validation.json`、
`target/desktop-wayland-clipboard/clipboard-validation.json`、
`target/desktop-regressions-final/desktop-regressions.json` 和
`target/windows-ocr-fixtures-final/ocr-fixtures.json` 和
`target/background-regressions-final/background-regressions.json`。

## 验证范围

Wine 使用隔离前缀和私有 Xvfb，没有连接用户桌面。Wine 不能代表真实 Windows 系统 OCR、
系统睡眠、长期空闲后的工作集回收或实际桌面快捷键冲突。合成 WM_HOTKEY 验证动作分派，
没有声称 Wine 的 PrintScreen 按键注入已通过。中英文样本计时包含进程和模型初始化，
不能作为 Windows 系统 OCR 与 Paddle 的速度对比。

VirtualBox Windows 客户机尚未直接验证；已验证它所需的 BMP 来源及重复传输。
RPM 的共享库门槛需要在 Fedora 构建工作流中对真实产物检查，字符串回归不能替代实际安装验证。

## PR 与 Issue 处理依据

- #159 保留完整非阻塞 FD 传输修复；#162 保留 Direct 箭头及形状样式并修正缩放；
  #160 的重复工具提示采用已有本地实现，其点击、遮挡、失焦和窄屏处理更完整。
- #163、#164 对应本次 X11 修复。
- #144 对应本次图片格式扩充和真实协议验证。
- #161 的聚光灯由已有矩形与椭圆 Focus 工具实现，像素与交互测试已覆盖。
- #156 的多显示器工具栏位置已有按显示器边界限制的实现与回归。
- #158 的跨 PID 命名空间身份检查已有修复，保留 UID 校验。
- #155、#157 来自旧 Qt 包；Rust 原生版不依赖该 Qt 插件或旧 FFmpeg soname。
- #154 的官方政策与替代服务比较见[翻译替代方案](zh-CN/translation-alternatives.md)。
