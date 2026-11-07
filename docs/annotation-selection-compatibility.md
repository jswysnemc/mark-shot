# 标注选中框与删除按钮对照

选择工具中的普通标注、文字、线条、滤镜、序号、双框放大镜和尺子使用共用选框组件。
原来的分段角线与黑色光晕已移除。正文编辑器仍使用自身编辑边界，选区范围边框保持独立职责。

后续首次窗口容差、序号圆框范围、线条和滤镜留白及角度尺选框修正见
[annotation-hit-compatibility.md](annotation-hit-compatibility.md)。本页此前构建结果保持原始范围，
不能解释为后续源码已重新完成相同检查。

## 原版依据与范围

只读参考仓库为 `/home/snemc/workspace/mark-shot`。选框依据为
`qt/src/shot_window_selection_frame.cpp` 的 `drawSelectedAnnotationFrame`、
`selectedAnnotationDeleteButtonRect` 和 `selectionHandlePoints`；旋转几何依据为
`qt/src/shot_window_hit_testing.cpp` 中的尺寸比例、旋转中心、旋转点和控制点函数。

`scripts/annotation-selection-reference.py` 提取完整生产函数，在小型 Qt 宿主中调用。
宿主提供单个矩形及坐标上下文，并保留生产调用方启用抗锯齿的设置。四种矩形、六种角度和
1、1.5、2 倍像素比例共生成 72 张实际 Qt 图像；原始结果、函数和文件指纹位于
`tests/fixtures/qt-annotation-selection/`。宿主没有执行完整 Qt 应用、群组选中或全部专用工具分支。

| 项目 | 当前共用组件 |
| --- | --- |
| 颜色 | 固定橙色 RGB(251,146,60)，明暗主题使用同色 |
| 外框 | 完整虚线，线宽 2，虚线与间隔为 8、4 |
| 缩放点 | 9×9 方块，圆角半径 2；专用工具保留原图样式比例 |
| 旋转点 | 直径 12 的橙色圆点，白色边宽 1.5，连接线宽 1.8 |
| 删除按钮 | 20×20 红色圆按钮，RGB(239,68,68)，白色边宽 2 和白色叉号 |
| 删除位置 | 单选沿右上角向外延伸 24，普通外框回退向右上外移；窗口四边保留 8 像素 |

普通矩形、椭圆和形状标记按正文边界绘制选框，不把描边或阴影留量加入缩放几何。
画笔与荧光笔按笔宽扩大边界。滤镜、序号、放大镜、线条和尺子继续提供各自控制点位置，
共用组件负责颜色、描边和控制点绘制；这些位置尚未逐项完成原版几何对照。

## 输入、历史与局部重绘

删除按钮在左键按下时删除整个标注。右键不执行删除；右键释放不结束按钮的左键持有。
若线条已有选中定点，红色按钮仍删除整条标注，键盘 Delete 保留原有定点删除职责。
保存历史发生在清除选择之前，因此撤销恢复正文及选中身份，重做再次删除。

已经选中的标注优先检查控制点，粗画笔位于正文初始命中区域之外的缩放点也能启动拖动。
普通缩放按可见几何映射，避免阴影留量造成第一次缩放偏移。

删除按钮采用约 24×24 逻辑像素的短期缓存，位置或输出比例变化时更新；它不进入历史或导出。
圆角方块与圆形控制点按原始像素相位绘入固定小块，再向本次损坏缓冲合成。完全位于损坏
范围之外的控制点不分配缓冲；父缓冲裁切不会重新改变圆角采样。静止状态不创建周期唤醒。

真实画笔撤销曾出现 19 个边缘像素差异，固定几何的局部恢复回归进一步重现了问题。
修复前后测试日志分别为 `target/annotation-selection-r1/pen-partial-before-r2.log` 和
`target/annotation-selection-r1/pen-partial-after-r1.log`；旧删除按钮的差异现场也保留在
`target/annotation-debug-sway-r1/after/`。验证继续要求同构建撤销前后逐像素一致。

## 自动验证与边界

`src/renderer/annotations/selection_chrome_tests.rs` 核对 72 个删除矩形，允许的逻辑坐标误差
小于 0.0001；绘制组件使用原版提供的角点与旋转点，对照完整选框、控制点和删除按钮。
绘制检查分别比较总 alpha 误差和四邻域一致的实色内部，后者 RGBA 容差为 2。
当前 72 张输出的 alpha 误差均值为 1.0452%，最大值为 3.2298%，低于 5% 上限。
当前输出与索引位于 `target/annotation-chrome-render-r2/`，原始像素差异仍保留。
上述比较没有证明 Qt 与 Rust 栅格化逐像素一致。

`scripts/wayland-annotation-selection.py` 验证普通矩形的完整选框、按钮实际位置、左键按下
删除、右键隔离、撤销、重做、明暗主题、空闲资源及原始 PNG。旧构建执行同一完整界面
流程时因缺少统一控制点而失败，现场为 `target/annotation-old-negative-sway-r1/`。

`scripts/wayland-annotation-selection-tools.py` 通过实际输入创建矩形、椭圆、画笔、荧光笔、
形状标记、线条、箭头、带编号箭头、滤镜、序号、双框放大镜、长度尺、角度尺和文字。
每项检查完整标注删除及撤销画面；使用 Sway 或不可见 Xvfb 中的 niri 私有会话。
专用工具按原图像素比例设置颜色数量判据，撤销比较始终要求逐像素一致。
每次原生捕获前先核对私有空桌面区域已恢复纯色，避免 niri 的窗口关闭动画进入下一张原图。
窗口创建后继续等待真实键盘进入与指针坐标投递；画面检查等待实际提交，仍使用相同像素判据。
最初包含关闭动画的工具矩阵保留在 `target/annotation-rel-niri-r1/tools/`，
初始拖选早于表面指针进入的现场保留在 `target/annotation-tools-rel-sway-r2/tools/number/`。

六类普通标注已接入旋转状态、鼠标操作与正文绘制，当前构建、生产函数对照及验证范围见
[annotation-rotation-compatibility.md](annotation-rotation-compatibility.md)。多选、群组变换、
所有缩放策略、跨输出选框、混合比例和完整界面一致性仍需验证。

## 此前选框发布构建

此前选框发布构建保留在
`target/annotation-selection-candidate-r3/mark-shot`；大小为48,455,304字节，SHA-256为
`e436867e506983f0237127ad48b037dd02be3c9496c6f3d42464a832d1fd1895`。
完整清单为 `target/verification-annotation-selection-r1/result.json`，源文件指纹另存为
`source-snapshot.json`。工作区1095项 Rust 测试通过、10项忽略，201项 Python 测试、
严格 Clippy、格式和新增脚本 Ruff F 类检查通过；全部动态依赖可解析，没有 Qt 依赖。

同一候选在私有 Sway 与 Xvfb 内的 niri、1.5倍输出上完成普通删除、十四类工具、字段、
尺寸、主选择及完整截图翻译六套流程，合计十二套。两个工具矩阵各十四项，背景分别
恢复纯黑与 RGB(64,64,64)，所有整项删除和撤销画面检查通过。原始结果保存在
`target/annotation-rel-sway-r3/` 与 `target/annotation-rel-niri-r3/`；普通标注导出 PNG
与此前a1构建在同会话逐字节相同。68次空闲采样保持单线程，CPU时钟和缓冲提交不增长。

所有检查结束后，在同一个私有 niri 会话顺序测量此前a1构建和本次候选，各三次温启动。
首帧缓冲提交中位数为16.16→15.59毫秒，候选父进程观察中位数为21.19毫秒；
空闲 RSS 中位数为57,612→57,156 KiB，候选约55.8 MiB。三个样本不能证明确定的性能变化，
也不代表冷启动或合成器实际呈现时刻。完整分布位于
`target/annotation-performance-before-r1/report.json` 与
`target/annotation-performance-after-r1/report.json`。

此前a1发布文件与清单继续保留在 `target/selection-primary-candidate-r1/mark-shot` 和
`target/verification-selection-primary-r1/result.json`。本轮没有修改用户配置或原版 Qt 源文件，
完整 Rust 重写目标保持未完成状态。
