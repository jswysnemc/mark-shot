# 长截图工作流与验证范围

原版依据为 `qt/src/scroll/scroll_session_window_lifecycle.cpp`、
`scroll_session_window_events.cpp`、`scroll_session_window_capture.cpp` 和
`scroll_session_window_preview.cpp`。Rust 的捕获循环、输入、绘制与交接分别位于
`src/app/scroll/`；控制布局和预览位于 `src/scroll/panel/`。

控制面板提供方向、暂停、标注、保存、复制和取消。按钮只由同一按钮内的左键按下和
释放激活，右键与中键不交付操作。方向按钮拖动沿选定轴调整捕获区域，并限制到输出
边界；单击切换方向，已经增长的长图遵循拼接器的方向锁定。

捕获表面使用 Layer Shell 的按需键盘焦点，底层页面继续接收滚轮和键盘。选区必须
位于一个输出内，输出绑定来自原表面身份，不使用当前指针或全局输出数组索引猜测。
其他输出的冻结截图表面在长截图期间释放。缺少 Layer Shell 时使用标准 xdg-shell
的受约束控制窗口，具体范围见[普通窗口长截图](layerless-scroll-compatibility.md)。
持续 Portal 会话及原生后台取帧已实现，具体范围见
[持续捕获会话](continuous-capture-compatibility.md)，不能据此宣称全部 Wayland 环境
已经支持完整长截图。

捕获间隔为 45 毫秒，静止后降低频率。面板按内容、悬停、按钮或主题变化提交；暂停
后无周期捕获期限。边框位于裁切区域外五逻辑像素。面板尝试在左右上下避让选区，
没有安全位置时捕获期间隐藏，静止后自动暂停并显示操作。恢复捕获会先清除面板。

预览使用细节视图和完整概览，支持滚轮浏览和概览拖动，末端恢复跟随最新内容。只为
可见预览分配光栅缓冲，图像缓存按拼接修订更新。标注与 Enter 重新进入图片编辑器，
保存使用独立文件对话框，取消保存恢复同一拼接会话。复制只执行复制，保存只写一个
目标，取消不会按预设输出额外保存。结果来自原始长图，不取缩略图。

`scripts/wayland-scroll-capture.py` 使用私有 Sway 与原生测试页面。页面记录实际收到的
滚轮和键盘位移，期望图片逐条连接实际新增条带。独立
`scripts/wayland_scroll_raw_capture.py` 直接读取原始 wlr-screencopy 缓冲，避免外部
截图工具在分数比例下按逻辑尺寸重采样。原始像素比较不设置误差容限。

验证覆盖纵向、横向、按钮职责、暂停后捕获请求与缓冲提交停止、保存取消、标注交接、
双主题恢复、大选区捕获污染、实际保存和取消后页面恢复。首组完整工作流构建为
`target/light-scroll-candidate-r2/mark-shot`，SHA-256：
`2951c1398a927ecc4ab5ef546a36dc0a481b93bae39fb8749915e2c290409af5`。
最终清单为 `target/verification-light-scroll-r1/result.json`。同构建一倍和一点五倍
比例结果分别为 `target/light-scroll-complete-scale1-r2/checks/result.json` 与
`target/light-scroll-complete-scale15-r1/checks/result.json`。纵向、横向和大选区结果的
完整 RGB 像素比较均通过；一点五倍的三组尺寸分别为 420×1068、996×420 和
1256×920。其他合成器和多输出真实会话结论，以各自完成的记录为准。

这组证据证明上述原生工作流，不能证明控制栏、状态文字与 Qt 的全部光栅细节一致。
整体重写仍未完成。持续会话改动见[持续捕获会话](continuous-capture-compatibility.md)，
最新候选及浮层、普通窗口交叉矩阵见[普通窗口长截图](layerless-scroll-compatibility.md)。
