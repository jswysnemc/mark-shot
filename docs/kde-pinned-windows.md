# KDE 置顶窗口分析与验证

验证日期：2026-09-13。实际环境为 KWin 6.7.5、Qt 6.11.2，使用隔离的 KWin 虚拟输出和 D-Bus 会话。

## 已复现的问题

| 触发方式 | 原有行为 | 原因 |
| --- | --- | --- |
| 同时打开两个 OCR 窗口，取消其中一个置顶 | 另一个窗口也失去置顶 | 脚本只按标题匹配，未区分窗口和进程 |
| OCR 与钉图同时存在，取消 OCR 置顶后隐藏并重新显示钉图 | 钉图重显后不再置顶 | 两类窗口共用同一个可替换脚本，取消操作卸载了其他窗口需要的监听 |
| 在 KDE Wayland 中切换 Qt 窗口标志 | 窗口会隐藏并重新映射 | `QWidget::setWindowFlags()` 的行为会影响窗口身份与放置，Qt 标志不能作为 KWin 实际层级的证据 |
| 多个常驻脚本之间卸载、重新加载 | 返回的脚本编号可能指向仍在运行的其他脚本 | KWin 使用当前脚本数量生成编号；卸载较早的脚本会留下编号空缺 |
| XWayland 窗口隐藏后重新显示 | Qt 仍持有置顶标志，KWin 的 `keepAbove` 已变为 false | 需要在重新映射后恢复合成器中的实际状态 |
| KDE Wayland 下拖动钉图左边缘或上边缘 | 窗口尺寸变化，但对侧边缘也跟着移动 | 普通 Wayland 窗口不接受应用通过 `setGeometry()` 请求的绝对位置 |

## 实现边界

- KDE 中的 OCR 和钉图窗口采用带编号的标题，例如 `OCR 结果 (1)`，并同时校验进程号。同名窗口与其他进程互不影响。
- 每个应用只加载一个 KWin 脚本。D-Bus 状态通道保存各窗口的目标状态，按钮切换和窗口关闭只更新状态表。
- KWin 脚本通过异步请求等待状态变化；空闲期间每 10 秒回复一次，保持订阅。连续切换最终收敛到最新状态。
- 新窗口映射或标题就绪时重新应用对应状态。销毁窗口会删除其状态，应用退出时卸载应用脚本。
- KDE Wayland 使用普通窗口和 KWin `keepAbove`，切换置顶时不修改 Qt 窗口标志。窗口管理器继续负责移动、缩放与焦点。
- KDE XWayland 保留 Qt 窗口标志，并由同一状态通道处理重新映射后的置顶恢复。
- KDE Wayland 的钉图边缘缩放使用 `QWindow::startSystemResize()`，由 KWin 保持对侧锚点，客户端继续约束图片比例。首次显示前就解除固定尺寸，避免合成器拒绝第一次缩放。

代码分工：

- `src/pinned_window/pinned_kde_keep_above.cpp`：桌面识别、窗口编号及窗口状态生命周期。
- `src/pinned_window/pinned_kde_keep_above_bridge.cpp`：应用级 D-Bus 订阅、心跳及脚本生命周期。
- `src/pinned_window/pinned_kde_keep_above_script.cpp`：KWin 窗口匹配、状态应用及映射监听。
- `src/pinned_window_top.cpp`：平台置顶入口，保留其他桌面的处理路径。
- `src/pinned_window/pinned_native_resize.cpp`：原生边缘缩放、比例约束及尺寸回调。
- `src/pinned_window/pinned_image_window_geometry.cpp`：为 KDE 钉图选择原生缩放，并同步图片绘制尺寸。

## 自动回归

脚本单元测试实际执行 JavaScript，检查窗口与进程隔离、Plasma 5 接口形式、标题晚到和特殊字符。缺少 Qt Qml 时会明确跳过脚本执行测试。

原生缩放的单元检查还验证开放尺寸约束时不会放大初始小图，例如 12 × 8 的图片保持原始尺寸。

KWin 集成测试直接读取合成器中的 `keepAbove`、窗口身份、几何和堆叠顺序，覆盖：

- 两个 OCR 窗口与钉图同时存在时独立开启、取消置顶。
- 普通窗口取得焦点后，置顶窗口仍位于其上方。
- 隐藏再显示、连续 20 次切换、窗口缩放及编辑撤销历史。
- 相同标题来自其他进程时保持独立。
- 窗口关闭后删除过期状态，应用退出后卸载脚本。
- 原生 Wayland 下空闲超过常见 D-Bus 超时后继续切换。
- 英文 Wayland、XWayland，以及中文标题、1.25 倍缩放、双虚拟输出。
- 使用 KWin EIS 发送真实鼠标输入，检查四条边和四个角的对侧锚点、图片比例、置顶状态及原生窗口身份。

运行条件：Linux、Qt DBus、Qt Test、支持 EIS 的 KWin、libei、Xwayland、Python 3 和 `dbus-run-session`。测试默认关闭，通过构建选项启用：

```sh
cmake -S . -B build -DMARK_SHOT_KDE_INTEGRATION_TESTS=ON
cmake --build build --target mark-shot-kde-pinned-windows-test mark-shot-pinned-kde-keep-above-test -j 6
ctest --test-dir build -R '^(pinned-kde-keep-above|kde-pinned-windows-)' --output-on-failure -j 3
```

测试会话不设置 `KWIN_COMPOSE`，由 KWin 自行选择合成方式。KWin 6.8 的虚拟后端不再支持 QPainter，
强制 `KWIN_COMPOSE=Q` 时合成器初始化失败，窗口收不到帧回调，Qt 不再提交新尺寸，XWayland 也会因
dmabuf 导入失败断开。

每次检查创建独立显示服务、会话总线和临时配置，结束时回收测试进程。运行日志及环境记录保存在 `build/kde-test-results/`。

本轮完整执行 `QT_QPA_PLATFORM=offscreen ctest --test-dir build --output-on-failure -j 6`，82 项全部通过。其中三项 KWin 集成检查的结果如下：

| 场景 | 通过的行为检查 | 日志目录 |
| --- | --- | --- |
| 原生 Wayland、英文、单输出，含 27 秒空闲订阅 | 39 | `build/kde-test-results/wayland/` |
| XWayland、英文、单输出 | 27 | `build/kde-test-results/xwayland/` |
| 原生 Wayland、中文、1.25 倍缩放、双输出 | 37 | `build/kde-test-results/scaled-zh/` |

## 生产窗口复核

本轮另使用真实 OCR 和钉图类，借助 KWin EIS 输入接口发送鼠标事件，验证置顶按钮、钉图菜单、标题栏移动、OCR 缩放握柄、钉图边缘缩放及释放后的光标恢复。英文单输出、中文 1.25 倍缩放双输出均通过，编辑内容及撤销历史也保持完整。

左边缘问题的最小复现只有一张 310 × 200 的钉图：向左拖动 40 个逻辑像素，修复前 KWin 的横坐标仍为 485，右边缘从 795 移到 835；修复后横坐标为 445、宽度为 350，右边缘保持 795。前后记录分别保存在 `/tmp/mark-shot-kde-check/left-resize-minimal/` 和 `/tmp/mark-shot-kde-check/left-resize-fixed/`。

生产窗口验证日志保存在 `/tmp/mark-shot-kde-check/production-ui-final/` 和 `/tmp/mark-shot-kde-check/production-ui-zh/`。

人工复核可以按以下顺序操作：

1. 打开两个 OCR 结果和一个钉图，再激活普通窗口，确认三者的置顶状态。
2. 取消其中一个 OCR 置顶，确认另外两个窗口仍置顶；切到原图页重新开启置顶。
3. 在 OCR 中编辑文本，切页并切换置顶，检查窗口尺寸、位置、原文和撤销历史。
4. 拖动标题栏与钉图，使用 OCR 缩放握柄和钉图边缘缩放，确认操作结束后的光标与层级。
5. 在钉图菜单中取消并恢复置顶，隐藏再显示窗口，确认状态仍符合开关。

## 验证范围

上述运行证据针对 KWin 6.7.5。Plasma 5 的脚本接口形式已有单元检查，但没有在 Plasma 5 桌面实测；原生 X11 会话也未实测，不能把 XWayland 结果直接等同于原生 X11。

KDE 置顶通道需要 Qt DBus 与 KWin Scripting 接口。关闭该接口或禁止运行脚本的桌面策略不在本轮通过范围内。

KWin 按脚本数量分配编号仍是上游限制。本实现避免了应用内反复装卸脚本造成的冲突；启动前其他脚本已经留下编号空缺的组合未实测。

## 官方依据

- [KWin 脚本 API](https://develop.kde.org/docs/plasma/kwin/api/)：`keepAbove`、窗口编号、进程号、窗口映射信号与异步 `callDBus`。
- [KWin 6.7 脚本实现](https://github.com/KDE/kwin/blob/Plasma/6.7/src/scripting/scripting.cpp)：`loadScript()` 通过脚本数量分配编号，`unloadScript()` 延迟删除对象。
- [QWidget 窗口标志](https://doc.qt.io/qt-6/qwidget.html#windowFlags-prop)：切换窗口标志会隐藏窗口，需要重新显示。
- [QWindow 原生缩放](https://doc.qt.io/qt-6/qwindow.html#startSystemResize)：支持的平台应优先让窗口管理器处理交互缩放。
- [KWin EIS 输入后端](https://github.com/KDE/kwin/blob/Plasma/6.7/src/plugins/eis/eisbackend.cpp)：生产窗口鼠标检查使用的合成器输入接口。
