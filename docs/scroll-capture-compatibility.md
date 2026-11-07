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

## 捕获范围与浏览范围双框

2026-11-07 当前源码审计确认此前仅绘制浏览框，历史操作流程通过不代表黄色捕获框完成。
本次新增 `src/scroll/panel/capture_marker.rs`，采用原版黄色 `(250,204,21,245)` 两像素边线
与透明填充，青色 `(45,212,191,235)` 浏览框独立绘制。最近匹配的原图位置及帧长度保存在
`ScrollSession.capture_range`；返回已知内容即使没有新增像素也更新范围并通知面板重绘。
概览框不参与原图拼接或导出，不增加完整长图副本。

候选为 `target/scroll-markers-candidate-r1/mark-shot`，SHA-256：
`6c08c5e4c8db4a11ecce3f2b6b2a6d00dfdb401a6c8b9c740e5cb600f33d6f99`。
同一文件在私有Sway一倍与一点五倍输出通过：
`target/scroll-markers-r8/result.json`、`target/scroll-markers-scale-r1/result.json`。
两方向均检查黄色/青色边线、反向移动及返回原位置；预览浏览不改变黄色捕获起点，青色浏览框移动。
全部导出逐像素一致：一倍纵向280×712、横向1240×280；一点五倍纵向420×1068、横向1860×420。
同时覆盖静止恢复、大选区避让、暂停无持续捕获、另存为取消、复制与实际保存。

实际像素检测位于 `scripts/wayland_scroll_markers.py`，区分横纵连续边线并拒绝随机噪声和单色框。
原版青色框可能遮住黄色尾边，浏览保持断言使用未遮挡的捕获起点；横向场景延长到二十次滚动，
确保真正出现概览。旧候选 `target/current-linux-regression-candidate/mark-shot` 的两方向画面
均无法通过黄色框检测。最初验证的构建就绪竞态、颜色叠加、遮挡和不足概览长度记录均保留，
没有删除失败证据或将其表述为产品通过。

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
