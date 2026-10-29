# Hyprland 置顶贴图拖动分析与验证

对应 [issue #107](https://github.com/jswysnemc/mark-shot/issues/107)。验证日期：2026-09-13。
实际环境为 Hyprland 0.56.2、Qt 6.11.2、Aquamarine 0.15.0，使用隔离的 Wayland 会话和虚拟输出。

## 已确认的原因

Wayland 的 `wl_pointer.motion` 提供相对 surface 的局部位置。Qt 使用窗口原点构造
`QMouseEvent::globalPosition()`；layer-shell 贴图通过 margin 改变实际位置时，这个原点没有同步。
实测贴图位于 `(1240, 800)`，Qt 窗口原点仍为 `(0, 0)`。

原实现直接用上述全局坐标计算下一帧位置。移动窗口又改变后续输入的局部坐标，形成反馈；
Qt 还会忽略全局位置未变化的移动事件。因此，仅把局部位置加到当前贴图位置上，或者把位移乘二，
无法可靠处理异步提交、高频输入和反向拖动。

本轮在 100% 缩放下复现鼠标移动 300 个逻辑像素、贴图仅移动 150 个；
150% 和 200% 下的最小检查均为鼠标移动 100、贴图移动 50。关闭动画后仍然复现。

## 修复方式

- 拖动和边框缩放期间，原窗口继续接收鼠标输入，surface 的位置、尺寸和协议属性保持固定。
- 独立 layer-shell 预览负责移动、缩放与屏幕边缘裁切。原窗口和预览共享绘制代码，保留图片、边框、文字选择及翻译覆盖层。
- 预览开始绘制后清空原窗口的图像。松开鼠标后，原窗口应用最终几何，完成绘制后回收预览。
- 跨屏时只重建预览，原窗口保持鼠标抓取；松开后再把原窗口绑定到目标输出。预览使用独立顶层窗口，避免原窗口重建时连带销毁。
- 窗口隐藏或关闭时终止拖动状态并清理预览；跨屏重建的临时隐藏保留预览，直到原窗口恢复绘制。
- KDE 的普通窗口、原生移动和原生缩放继续使用原来的处理路径。

预览既设置 Qt 输入穿透和禁止激活，也在 layer-shell 协议中设置
`KeyboardInteractivity::None`。Hyprland 映射可交互 layer 时可能调用
`releaseAllMouseButtons()`；仅设置 Qt 窗口标志不足以避免拖动被中断。
内部插件接口升级到 `1.3`，防止旧插件把新枚举解释为独占键盘输入。安装时须使用同一构建的主程序与 layer-shell 插件。

实现不使用 Qt 私有接口，也没有增加应用运行时协议依赖。虚拟指针协议仅供可选集成测试使用。

## 自动回归

测试通过正式 `mark-shot --pin-image` 显示 400×200 的灰色 PNG，关闭自动 OCR 和合成器动画。
每个测试创建独立配置、D-Bus 会话、KWin 虚拟父显示及 Hyprland 输出，不连接现有桌面。
用 `hyprctl -j cursorpos` 和 `hyprctl -j layers` 比较实际读数，容许最多 2 个逻辑像素的取整误差。

可选依赖：Hyprland、hyprctl、kwin_wayland、dbus-run-session、Python 3、Wayland 客户端开发库和 wayland-scanner。

```sh
cmake -S . -B build -DMARK_SHOT_HYPRLAND_INTEGRATION_TESTS=ON
cmake --build build --target mark-shot mark-shot-hyprland-virtual-pointer --parallel 4
ctest --test-dir build -R '^hyprland-pinned-windows-' --output-on-failure --parallel 3
```

NVIDIA 专有驱动无法为 Hyprland 的 HEADLESS 输出分配 GBM 缓冲，输出尺寸会停留在 0×0。
多显卡机器上，脚本让 KWin 虚拟父显示使用第一个非 NVIDIA 渲染节点；也可以用
`MARK_SHOT_HYPRLAND_TEST_RENDER_NODE=/dev/dri/renderD129` 指定节点。

每档缩放执行 25 个行为检查：四向拖动采用 10 ms、1 ms 和 0 ms 额外间隔；
另测单次跳移、同一次抓取中的斜向反转、左侧和顶部裁切及恢复、边框缩放、滚轮锚点，
以及混合缩放跨屏往返。0 ms 输入仍逐次确认协议帧，不等待额外定时器。
拖动期间检查原输入窗口保持固定；松开后必须只剩原窗口，不能把残留预览当作通过结果。

| 输出缩放 | 修复后鼠标横向位移 | 修复后贴图横向位移 | 行为检查 |
| --- | --- | --- | --- |
| 100% | +300 | +300 | 25 项通过 |
| 150% | +300 | +300 | 25 项通过 |
| 200% | +300 | +300 | 25 项通过 |

运行记录保存在 `build/hyprland-test-results/`，包含实际几何、指针位移、输出配置和合成器版本。
修复前记录位于本次工作目录之外的 `/tmp/mark-shot-issue-107/artifacts/before-{100,150,200}/`。

安装 `grim` 后可以同时保存按住鼠标时的画面：

```sh
python3 scripts/test-hyprland-pinned-windows.py \
  --binary build/mark-shot \
  --pointer build/mark-shot-hyprland-virtual-pointer \
  --scale 2 --screenshots \
  --artifacts /tmp/mark-shot-hyprland-visual
```

本轮截图复核涵盖斜向反转、左上裁切和混合缩放跨屏：原窗口区域没有重复图像，预览尺寸与裁切位置正确。
另在按住鼠标拖动期间通过隔离会话的 `wtype -k Escape` 关闭正式贴图，确认原窗口和预览全部移除。
完整构建通过，`QT_QPA_PLATFORM=offscreen ctest --test-dir build --output-on-failure --parallel 4`
的 85 项检查全部通过，其中包含 KDE 原生 Wayland、XWayland、缩放双输出置顶检查，详见 [KDE 验证记录](kde-pinned-windows.md)。

## 验证范围与源码依据

上述运行结果来自虚拟输出，不等同于三种物理显示器或所有 Hyprland、Qt 版本的实测。

- [Qt Wayland 输入坐标构造](https://code.qt.io/cgit/qt/qtbase.git/tree/src/plugins/platforms/wayland/qwaylandinputdevice.cpp?h=v6.11.2)：`pointer_motion()` 使用窗口映射构造全局位置。
- [Qt 鼠标事件处理](https://code.qt.io/cgit/qt/qtbase.git/tree/src/gui/kernel/qguiapplication.cpp?h=v6.11.2)：`processMouseEvent()` 判断全局位置是否变化。
- [Hyprland layer 映射与焦点](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/view/LayerSurface.cpp)：`onMap()` 在聚焦可交互 layer 前释放鼠标按键。
