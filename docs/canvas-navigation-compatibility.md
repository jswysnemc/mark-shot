# 画布导航与图片初始布局对照

当前源码已接入持续画布导航：锚点滚轮缩放、中键直接平移、Ctrl 双击复位、边缘滚动条、
九百毫秒倍率提示、稳定画布输入、冻结原图重采样和矢量正文合成。导航状态不属于标注或
撤销记录。完整 Rust 重写目标保持未完成，原生验证和完整 Qt 像素审计分别记录。

## 原版行为

只读原版位于 `/home/snemc/workspace/mark-shot/qt/src/`。实际生产函数明确如下规则：

- `imageNavigationAvailable()` 在启用图片导航，或进入普通截图编辑阶段时返回真
- `wheelZoomsImage()` 对图片编辑的所有工具开放；普通截图只有选择工具优先缩放
- 选择工具或尺子已经选中标注，以及已有文字正在编辑时，粗细滚轮优先于图片缩放
- 绘制、图片平移和工具栏拖动期间忽略缩放；每刻度倍率为 `1.12`
- 角度轴优先于像素轴，纯像素输入以 80 单位为一刻度；水平轴不改变纵向缩放
- 倍率为适配窗口尺寸的 `0.25–64` 倍，不是原图像素的一比一倍率
- 锚点在图内时保留对应原图点；图外锚点使用已存中心，再执行原版中心约束
- 图像某一轴能完整显示时，该轴重新居中；超出窗口时限制可见中心，避免露出空白
- 360 毫秒内两次非重复 Ctrl 按下复位缩放和原图中心
- 编辑阶段中键按下先提交文字、结束旧指针交互，再按完整初始位移平移
- 中键单击动作被配置且满足触发条件时才保留单击候选；候选使用平台曼哈顿拖动阈值
- 默认中键没有单击候选，因此直接平移；超过阈值的候选不能因返回原位置而恢复

滚动条占 14 逻辑像素。内容范围与页长按完整窗口尺寸计算，两条滚动条只在几何长度中
互相避让。范围和值采用原版整数舍入，单步为窗口尺寸除以 12，最少为 1。
内容溢出并且指针靠近边缘 32 像素以内，或滑块正在拖动时才显示。悬停样式、箭头光标
及停止画布悬停预览来自完整 `ui/canvas_scrollbar_style.cpp`。

## 当前模块

```text
features/canvas_navigation/
  model.rs          双精度缩放、原图中心、有界平移和滚动中心
  coordinates.rs    原图与窗口映射、最后原图像素约束
  policy.rs         工具、选中成员与滚轮职责
  scrollbars.rs     原生整数范围、页长、几何与按需显隐
editor/file_canvas.rs  真实图片入口的初始适配、原图存储和裁切
tests/canvas_navigation.rs  完整生产函数与图片入口对照
tests/fixtures/qt-canvas-navigation/  原始字段和原生控件样本
scripts/canvas-navigation-reference.py  只读采集器
scripts/wayland-canvas-layout.py  私有图片入口验证
```

导航模型不持有像素、标注或撤销数组。缩放和视图变化不得重写持久标注几何；输入
与绘制在稳定原图、既有逻辑坐标和窗口坐标之间映射，界面控件保留屏幕坐标。
图片入口仍使用模型生成稳定初始矩形；持续导航保存独立投影，不改变该矩形或原始像素。
普通截图共享完整工作区视图，各输出按其原点和设备比例合成，不能用单输出假设替代多输出支持。

原版 `updateFrozenImageRect()` 等比适配整个窗口，小图也会放大。此前图片入口额外扣除
64 像素宽度和 96 像素高度，并限制最大比例为 1；现已移除这些差异。原始图像仍独立
保存，预览缩放不替代原图，完整裁切须恢复原图尺寸及全部预乘像素。

## 参考与检查

`scripts/canvas-navigation-reference.py` 完整提取生产视图、投影、滚轮分派和倍率函数，
同时编译完整画布滚动条样式。采集前后检查原版源文件和宿主指纹。Qt 宿主使用 offscreen，
明确关闭 GLX，并移除真实桌面的显示器与合成器连接变量。其滚动条为实际 Qt 控件。

滚轮宿主只观察实际缩放或宽度 setter 调用；宽度 setter 返回未修改，使后续宽度提交
业务不可到达。它没有执行完整 ShotWindow 初始化，也不替代应用内指针、历史或焦点验证。

最新只读参考为 `target/canvas-navigation-r1/qt-reference-r5/`：

- 28 组原图与窗口组合，420 次连续缩放、平移、整数滚动、停用恢复、窗口变化和复位
- 720 组完整 `wheelEvent()` 分派，包含工具、成员数量、已有文字、阶段和拖动阻断
- 56 组角度、纯像素与优先级倍率
- 96 张原生滚动条样式图，记录实际样式状态和控制范围；目前为 1 倍像素图

固定 JSON、来源清单及样式 PNG 同时保存于 `tests/fixtures/qt-canvas-navigation/`。
四项 Rust 专项测试对照这些真实结果，并通过实际 `FileCanvas` 检查原图尺寸、
透明度与完整裁切。样式图目前仅作为后续滚动条绘制依据，尚未与 Rust 控件逐像素比较。

32×20000 长图在 1000×700 窗口中，Qt 实际初始上边界为约 `-5.68×10⁻¹⁴`，高度为
`700.0000000000001`。边界测试采用明确浮点容差，同时要求完整原图裁切逐字节相同。
旧标记测试的固定“源图外”坐标以及长图固定扣减 96 像素的假设已改为真实边界检查。
修正前失败日志保留在 `target/canvas-navigation-r1/workspace-r1.log`、`workspace-r2.log`
和 `file-canvas-r3.log`。

旧 R3 多选候选执行同一新预览检查时，三个位于旧小图之外的采样均为灰色，现场位于
`target/canvas-layout-negative-sway-r1/`。这个结果确认预览检查能拒绝旧的初始布局。

## 初始布局候选

候选为 `target/canvas-navigation-layout-candidate-r1/mark-shot`，大小 48,572,808 字节，
SHA-256 为 `4bf8f6f9509d75e049faf9b6e20b560ac530946d011d591882db84e31c776fdf`。
它包含导航领域模型和图片初始适配修正，持续导航输入与渲染尚未连接，仍未发布。
构建前后核对源文件，快照与清单位于 `target/verification-canvas-navigation-layout-r1/`。

1157 项 Rust 工作区测试通过、10 项忽略；四项导航专项、201 项 Python 测试、严格
Clippy、格式及两份脚本 Ruff 检查通过。Rust 文件最大 741 行，`src/app/input.rs` 保持
741 行。动态依赖均可解析，没有 Qt 动态依赖。

同一候选在私有 Sway 和不可见 Xvfb 中的 niri、1.5 倍输出上通过实际图片入口，分别测试
不透明和半透明原图，共四例。三个实际采样确认小图覆盖此前灰色留边；复制出口恢复
384×240 原图，四个角点和全部预乘 RGBA 字节与原图相同。此检查不证明 PNG 元数据、
色彩空间或任意隐藏 RGB 数据全部兼容。

结果位于 `target/canvas-layout-sway-r1/check/result.json` 和
`target/canfit-niri-r1/check/result.json`。32 个静止样本保持单线程，CPU 计数与缓冲提交
不增长，RSS 为 57,588–57,936 KiB。它们属于图片打开后的空闲样本，不代表冷启动性能。

## 持续导航接入

新增模块按职责分开：

```text
editor/canvas_navigation/
  state.rs       共享投影、平移手势、滚动条、Ctrl 计时和输出背景身份
  transform.rs   双精度稳定画布及全局屏幕映射
  hit.rs         单项与群组控制点的屏幕命中容差
app/canvas_navigation/
  input.rs       控件保护、滚轮、中键、滑块、Ctrl 边沿和焦点中断
  damage.rs      全视图变化及屏幕控件位置同步
  preview.rs     九百毫秒倍率提示期限和旧新损坏
renderer/frame_scene.rs       恒等视图的原有缓存场景
renderer/canvas_navigation.rs 原图背景重采样、完整正文及草稿、画布动态叠层
ui/canvas_navigation/         滚动条几何、状态样式和屏幕倍率气泡
app/input/tests/canvas_navigation.rs  实际分派、局部像素、历史、导出和多输出测试
scripts/wayland-canvas-navigation.py  私有原生导航流程
```

UI 的 `PointerState.local/global` 继续使用屏幕坐标。工具创建、拖动、文字字符选择、
尺子及 OCR 使用稳定画布位置；正文、激光、尺子标签和 OCR 叠层使用同一视图。
删除按钮、共同控制框、尺寸 HUD、工具栏、输入法候选及翻译浮条分别映射至屏幕。

非恒等视图按原始层次直接绘制当前成员及完整草稿，笔迹不只绘制缓存尾段。原图背景在
视图或选区改变时重采样到已有背景缓冲，局部重绘只恢复对应像素。首次局部采样方案在
1.5 倍输出产生三个字节差异；倍率提示直接局部绘制也曾产生十二个字节差异。背景身份缓存
和控制点固定像素缓冲修正了这两处差异，失败日志继续保留。

R4 的 13 项接入测试通过，覆盖锚点、直接中键、Ctrl 重复边沿、文字提交与焦点中断、滑块、
拖动阻断、删除、稳定笔迹、导出不变性、负坐标混合比例损坏分派，以及群组和原图在
反复局部重绘后的字节一致性。气泡到期无需下一次指针事件，到期后不保留定时唤醒。

Qt 离屏 `Sans Serif` 十一点半粗体度量保存在
`target/canvas-navigation-preview-metrics-r1/`，此宿主只记录字体度量，没有执行完整
`ShotWindow::drawWheelPreview()`，不能据此证明气泡所有字形或边框逐像素一致。

## 持续导航 R3 候选与原生证据

候选为 `target/canvas-navigation-input-candidate-r3/mark-shot`，大小 48,647,176 字节，
SHA-256 为 `a010516eea33540f3124709b1e5d6fc1a9161b0397f8ea3032071a0e4b094401`。
818 个源码文件在构建前后逐一核对，源码快照、检查日志索引和结果位于
`target/verification-canvas-navigation-input-r3/`。R3 与 R2 二进制字节相同；R3 源码修正了
Clippy 指出的可避免 `unwrap`，并使用完成检查后的独立清单。两者保留各自源码与验证记录。

1169 项 Rust 工作区测试通过、10 项忽略，其中包含 12 项导航接入测试；201 项 Python
测试、严格 Clippy、格式与脚本 Ruff 均通过。Rust 文件最大 741 行，`src/app/input.rs`
保持 741 行。动态依赖可解析，不包含 Qt 动态库。

同一 R3 候选在私有 Sway 与不可见 Xvfb/niri、1.5 倍输出上验证不透明和半透明图片，
共四例。实际原图内容随锚点缩放和中键完整位移变化；边缘揭示、滑块拖动、Ctrl 复位均
到达合成器画面。复制出口保持 384×240、四角与全部预乘 RGBA 字节，导航不改变原图交付。
结果为 `target/nav-sway-r3/check/result.json` 和 `target/nav-niri-r3/check/result.json`。

另外在同一候选和两种合成器上执行普通组及混合工具组的完整多选脚本，共四例，覆盖
属性、共同变换、单项重新编辑、删除、历史与导出。结果为
`target/nav-group-sway-r3/check/result.json` 和 `target/nav-group-niri-r3/check/result.json`。
本轮八个场景的 64 个静止样本保持单线程，CPU 计数及缓冲提交不增长。导航 RSS 为
58,080–59,472 KiB，多选 RSS 为 66,548–68,500 KiB；这不是冷启动或所有工具帧率基准。

R1 构建期间继续修正局部提示绘制，没有源码稳定性验证，保留在
`target/canvas-navigation-input-unverified-r1/`。R2 首次严格 Clippy 与原生检查失败，记录
保留在 `target/verification-canvas-navigation-input-r2/`。Sway 测试最初发出十分之一刻度，
倍率只有 106%，边界约束把平移限制为约 20×14 像素；改为真实完整刻度后原判据通过。
niri 最初一次截图早于新的缓冲提交，单独及连续复测通过。最终脚本按原判据等待实际
像素到达，超时仍失败。过长的独立 niri 套接字路径曾使测试运行器启动失败，后续使用
短路径；这个记录不代表应用导航失败。上述失败目录、截图和协议日志均保留。

## 持续导航 R4 文字修正与原生复测

R3 在非恒等视图重新编辑已有文字时，仍绘制持久原件，再叠加独立预览。新增回归先
确认 11,852 个差异字节，失败日志保留于 `target/canvas-navigation-text-original-negative.log`。
R4 显式传递 `text_session.original_id`，导航正文迭代排除这个原件，继续绘制预览；
标注数组和历史保持不变。修正后差异为零，日志位于
`target/canvas-navigation-text-original-fixed.log`。R3 候选和清单保留这一后续已知问题。

最新独立候选为 `target/canvas-navigation-input-candidate-r4/mark-shot`，大小 48,647,432
字节，SHA-256 为 `0e3cd41b0054d910e42fbb126edd61fa1dfd9eff5fd5e31efeb800c206594516`。
818 个源码文件与构建前快照一致，完整结果及日志索引位于
`target/verification-canvas-navigation-input-r4/result.json`。1170 项 Rust 工作区测试通过、
10 项忽略，其中导航接入为 13 项；201 项 Python 测试、严格 Clippy、格式及脚本 Ruff
通过。Rust 文件最大 741 行；动态依赖均可解析，不包含 Qt 动态库。

同一 R4 候选在私有 Sway 与不可见 Xvfb/niri、单输出 1.5 倍条件下完成四个不透明及
半透明导航场景、四个普通及混合多选场景。导航、边缘揭示、滑块拖动、Ctrl 复位及
完整 384×240 原图预乘像素检查通过。多选属性、共同变换、重新编辑、删除、历史和
导出通过。四份原生结果分别位于 `target/nav-sway-r4/check/result.json`、
`target/nav-niri-r4/check/result.json`、`target/nav-group-sway-r4/check/result.json` 和
`target/nav-group-niri-r4/check/result.json`。

64 个静止样本保持单线程，CPU 计数和缓冲提交不增长。导航 RSS 为 58,652–59,152 KiB，
多选 RSS 为 65,628–68,292 KiB。这些流程不包含完整 Qt 像素对照、所有工具、原生多输出
或冷启动基准。R4 尚未发布，现有发布文件及 R1–R3 证据保留。

## 导航后的旋转窗口间隔

后续源码把旋转连接点集中到 `features/annotation_selection/rotation.rs`。绘制与命中
使用同一反向比例调整外移向量，连接起点仍随正文投影。普通标注和群组保持原版固定
26 个窗口逻辑像素；专用工具暂时保留各自既有连接间隔，避免持续导航再放大这个间隔。
序号连接起点计入外框六像素留量，与实际绘制一致。

新增四项实际回归，导航接入共 17 项：普通标注覆盖六种倍率和四个角度，单项与群组
使用真实按下；三种设备比例下检查圆心像素和重复局部重绘；线条、滤镜、序号、放大镜、
长度及角度尺检查选择工具命中与各绘制器实际圆心。最初三项负向回归保留于
`target/canvas-navigation-rotation-negative-r2.log`。专用回归发现序号遗漏六像素留量，
失败及修正日志分别为 `target/canvas-navigation-rotation-fixed-r3.log` 和
`target/canvas-navigation-rotation-fixed-r4.log`。这不是专用工具完整 Qt 几何对照，也没有
覆盖尺子工具自身的独立命中入口。

`scripts/wayland-canvas-rotation.py` 通过私有座席创建、缩放及选择标注。它从实际四角和
旋转圆点测量窗口间隔，再执行旋转、精确历史恢复及原图尺寸导出；不读取应用内部模型。
同一脚本在旧 R4 候选上测得约 44.53 像素，固定 26 像素断言失败，日志位于
`target/canvas-navigation-native-rotation-negative-r4.log`，截图和协议现场保留于
`target/nrot-negative-r4/`。

## 旋转间隔候选与同指纹原生验证

最新默认构建为 `target/canvas-navigation-rotation-candidate-r1/mark-shot`，大小 48,649,352
字节，SHA-256 为 `d6719fb78ef67ec32cac59f26a512974832027a5897d52cb0d92a0a5d9c1ee07`。
895 个 Rust、测试及 Cargo 文件在构建前后相同，内置 `src/config/default.toml` 另外
核对 R4 基线指纹并保存。快照、配置及完整索引位于
`target/verification-canvas-navigation-rotation-r1/result.json`。默认工作区 1174 项 Rust
测试通过，全特性为 1175 项，各有 10 项忽略；导航接入 17 项、严格 Clippy、格式、
最终脚本 Ruff 与 Python 编译通过。Rust 文件最大 741 行，动态依赖可解析且没有 Qt
动态库。全特性 GPU 构建另存于 `target/canvas-navigation-rotation-gpu-candidate-r1/`，
原生矩阵使用默认构建。

同一默认候选在私有 Sway 与不可见 Xvfb/niri、单输出 1.5 倍下通过十二个场景：四个
不透明及半透明导航流程、四个普通及混合多选流程、四个放大图片的单项及群组旋转流程。
导航复制保持 384×240 原图全部预乘像素；旋转完成实际左键输入、记录区域的精确历史
恢复及原图尺寸导出，选框不进入图像。导航和多选结果为
`target/nav-sway-r5/check/result.json`、`target/nav-niri-r5/check/result.json`、
`target/ngs-r5/check/result.json`、`target/ngn-r5/check/result.json`；旋转结果为
`target/nrs-r5-r2/check/result.json` 和 `target/nrn-r5-r2/check/result.json`。

实际截图测得单项及群组旋转间隔为 25.13–26.67 逻辑像素，符合归一化截图的 1.2
像素采样容差；代码及实际输入回归仍要求固定 26 窗口单位。最终同一脚本拒绝旧 R4
候选，测得约 46.30 像素，失败保留于
`target/canvas-navigation-rotation-native-negative-complete-r4.log` 和 `target/nrot-neg-r2/`。
原生旋转历史区域为 `(150, 110, 510, 380)`；完整窗口与 Qt 的逐像素对照仍未覆盖。

96 个静止样本均保持单线程，CPU 计数和缓冲提交不增长。导航 RSS 为 58,904–59,704
KiB，多选为 66,520–68,128 KiB，旋转为 64,504–68,476 KiB。空闲采样与冷启动、所有
工具的帧率基准分别验证。候选仍未发布，现有发布文件及历次候选均保留原始指纹。

原生脚本诊断也保留全部失败现场。快速 Ctrl+Z、Ctrl+Y 在 360 毫秒内触发合法的
双击 Ctrl 复位，最终历史快捷键间隔为 400 毫秒。niri 测试座席五次 xdotool 上滚实际
产生十个完整 `value120=-120` 事件，Sway 为五个；最终投影读取原生协议刻度。原群组
框选起点位于工具栏，随后圆点又受到群组属性栏遮挡，测试群组下移到无遮挡位置。
中央裁切区不足以容纳放大群组四角，最终分别等待实际四个控制点；圆心按真实橙色
像素中心测量，移除固定 `min_y+5.5` 在归一化截图中的约 1.73 像素误差。
这些输入和测量修正没有修改已构建的 Rust 源码；原版完整属性栏避让仍待核对。

## 尚未完成

可配置中键单击动作与平台拖动阈值、滚动条分页保持及按键行为、所有工具与专用控制点的
完整原版几何、视图下所有文字焦点及输入法流程、原生多输出矩阵、输出变化、完整 Qt 图片
重采样、滚动条和倍率提示像素审计仍需覆盖。缩放后的尺子说明尺寸及专用旋转控件位置
也需与完整窗口对照。普通标注和群组的旋转外移已修正持续倍率影响，专用起点、间隔及
独立工具入口仍需完整对照。原有宽度数值预览仍属于单独的完整界面兼容工作。

生产 `annotationSizeScale()` 的图像与窗口单位不同，旋转把手按窗口坐标外移 26 像素，
首次命中也随当前图像投影反解容差。当前稳定逻辑坐标体系还需对照这些完整窗口行为：
等比放大图片后的通用和专用笔宽、阴影单位、首次命中、专用旋转外移，以及滚轮自动选择
当前工具刚画完对象的衔接。当前测试证明导航前后几何、历史和导出稳定，不证明这些
参数已经与 Qt 全部一致。

生产 `drawAnnotation()` 的通用笔宽为 `max(1.5, annotation.width * annotationSizeScale(...))`。
后续样式单位对照须同时考虑图片初始适配、冻结原图的像素比例以及窗口最小笔宽，
不能只调整持续导航倍率。

现有发布文件、图片初始布局候选和此前 R1–R3 多选候选继续使用各自原始指纹及清单。
