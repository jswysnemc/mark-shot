# 标注正文命中与窗口容差

首次选择、已选正文与尺子专用入口使用当前窗口单位，导航倍率不再同时放大或缩小
点击容差。普通旋转形状首次使用旋转后的轴对齐边界，直线和箭头只判断骨架。
完整 Rust 重写继续进行，本页仅记录本次命中修正及其独立验证范围。

## 模块

```text
src/features/annotation_selection/
  body_hit.rs       正文边界、首次命中、专用操作及已选边缘操作
  chrome.rs         持久正文选框
  rotation.rs       原始连接点与当前窗口连接点
src/editor/canvas_navigation/
  body_hit.rs       当前视图的窗口容差、首次正文与尺子入口
  hit.rs            已选单项与群组入口
src/features/ruler/drag.rs
                    独立正文命中、窗口选框与角度缩放
src/app/input/tests/annotation_style/
  hit.rs            真实输入、实际控制点像素和历史回归
  hit_reference.rs  完整 Qt 生产输出的差分回归
scripts/
  annotation-hit-reference.py  只读生产采集
  wayland-annotation-hit.py    私有 Wayland 首次点击回归
```

序号选框与缩放共同使用气泡范围；线条绘制、缩放和损坏区域共同读取正文选框。
滤镜去除六个原图像素的额外选框留白。角度尺的绘制、命中、删除位置与缩放共同使用
十四个窗口像素留白，长度尺中点是否可用由当前命中半径决定。

## 生产依据

只读来源为 `/home/snemc/workspace/mark-shot/qt/src/`：

- `shot_window_hit_testing.cpp`：完整 `annotationAt`、`annotationDragAt`、正文边界及旋转函数
- `shot_window_canvas.cpp`：首次按下先执行 `annotationAt`，再执行 `annotationDragAt`
- `shot_window_selection_frame.cpp`：完整序号与放大镜专用命中
- `shot_window_line_anchors.cpp`：完整最近骨架点命中
- `shot_window_ruler_tool.cpp`、`shot_window_ruler_drag.cpp`：尺子内部职责和已选入口
- `ruler/ruler_geometry.cpp`：完整最近控制点、旋转长度框与角度边命中
- `shot_window_ruler_painter.cpp`：角度尺窗口选框及旋转间隔依赖

探针使用原版 `shot_window_types.h`，完整编译尺子几何、尺子绘制、骨架路径和主题文件。
`shot_window_internal.h` 的轻量声明只提供生产 `rulerTickStep` 声明，不替换其实现。
窗口映射使用真实冻结矩形比例与平移。文字分支执行即终止采集，不能解释为文字通过。

生产首次容差是八个窗口逻辑像素。线条额外加原图笔宽的一半；箭头沿用骨架命中，
不按箭头头部轮廓扩大首次范围。序号圆心优先于尖端，独立尖端半径为基础容差的一点六倍。
气泡与尖端重合时，该尖端半径也生效。Qt 生产代码把零尺寸尖端矩形与气泡相并，实际
返回的通用范围仅含气泡，尖端依靠独立命中。不能把气泡到尖端之间的空白纳入首次范围。

放大镜控制点使用基础容差的一点二倍，镜框优先于源框。圆形区域扩容后仍按椭圆命中；
矩形内部不扩容。两者专用命中失败后，首次通用入口继续判断旋转后的联合边界。
选择工具允许长度尺内部整体移动，尺子工具把内部留给新的测量。

首次找到标注后也会执行完整控制点判断。不能假设首次按下只允许移动，或忽略此时
可能命中的缩放点。已选旋转点优先，线条定点按最近距离选择，边缘使用十窗口像素容差。

## 采集范围

`target/annotation-hit-reference-r4/` 记录 Qt 6.12.0、78 个场景与9,852个实际样本。
原图固定640乘480，笔宽固定为六；图像与窗口比例为0.125、0.5和2，旋转为零或31度。
覆盖矩形、椭圆、标记、滤镜、画笔、自由荧光笔、两点直线与箭头、序号、两种放大镜、
长度尺和角度尺。每个场景只有一项标注。完整入口会实际执行，来源、函数、类型、依赖、
采集脚本、提取助手、探针和结果均登记 SHA-256，编译前后来源一致。

三个 Rust 差分入口分别比较首次命中、尺子工具未选中正文，以及非尺子已选正文。
最后一项明确排除生产结果为 Rotate 的样本；尺子已选结果虽已采集，尚未纳入该矩阵。
原版完整入口记录不等于全部记录都已在 Rust 中逐项比较。

```sh
MARK_SHOT_HIT_REFERENCE=target/annotation-hit-reference-r4/reference.json \
  cargo test --lib --locked annotation_style::hit -- --include-ignored
```

## 失败与回归

`target/annotation-hit-before-r3.log` 记录两个真实输入失败：缩小后线外七窗口像素漏选，
放大后线外九窗口像素错误选中。原图笔宽保持六，中心命中正确。
`target/annotation-hit-reference-before-r2.log` 记录生产矩形扩容范围的差分失败。

真实输入回归还覆盖矩形边缘、旋转矩形外接框、分离尖端的序号、长度尺内部职责，
以及角度尺实际控制点像素、命中、缩放后三点和撤销。细线端点与边角重合时，原版优先
操作端点；外框九像素位置继续可以缩放。粗线损坏回归验证所有实际控制点像素均在
声明范围内，避免新选框沿用旧路径边界而留下残影。

第一版原生脚本使用非中心锚点，缩小后图像重新居中，线条约在(399,260)，点击却在
(320,248)。`target/annotation-hit-negative-sway-r1/` 和
`target/annotation-hit-sway-r3-r1/` 不属于有效命中证据，全部现场保留。
修正脚本在实际窗口中心绘图，同时核对缩放后的白色原图范围、笔迹宽度、真实刻度，
并确认按下和释放两个事件到达。有效旧候选负例保留在
`target/annotation-hit-negative-sway-r2/`，上一样式候选在七像素首次点击处失败。

早期命中 R1 至 R4 候选及所有失败日志继续保留，不替换为后续结果。

## 最终验证

候选为 `target/annotation-hit-candidate-r5/mark-shot`，大小48,663,048字节，SHA-256为
`c3cde79662dd73f20c308dc4c784cf163f2c693823080af8c9dc0e8d65a762a9`。
独立清单位于 `target/verification-annotation-hit-r5/result.json`，SHA-256为
`7064608d102b5c38ee20d4ba4cd93babcf43ffc0b5aa6130e1be16b198d3771e`。
一千三百八十八个 Rust、Cargo、资产、测试与两份新脚本输入在构建及验证前后一致。
Rust 文件最大741行，动态依赖全部可解析且没有 Qt。候选仍未发布。

默认工作区1,195项、全特性1,196项测试通过，各忽略13项。严格 Clippy、格式、两份
脚本 Ruff F 类检查与 Python 编译通过。37项相关渲染和几何专项通过，属于工作区结果。
三项 Qt 差分矩阵另行从忽略项目中执行；它们与七项真实输入回归合计十项通过，
七项真实输入已计入工作区数量，不能再次相加。

同一 R5 候选在私有 Sway、单输出1.5倍比例下顺序完成十八个场景：四个放大或缩小后
的首次命中场景，以及十四类工具整项删除与逐像素撤销流程。首次场景确认真实按下、
释放及刻度到达，七窗口像素接受、九窗口像素拒绝，四份1280乘960原图导出字节一致。
结果位于 `target/annotation-hit-sway-r5-r1/checks/result.json` 和
`target/annotation-hit-tools-sway-r5-r1/checks/result.json`。工具流程比较自身删除和撤销
画面，不证明完整 Qt 界面像素一致。

32个编辑后空闲样本全部为单线程，CPU计数与缓冲提交不增长，RSS为64,764至67,468 KiB。
这些样本不代表冷启动性能。R4首次命中正例继续单独保留，宽线选框损坏范围随后修正，
因此最终结果只计入重新运行的 R5。当前 `target/release/mark-shot` 仍保持原始指纹
`27866fbec304ab3d6afe52f5ecf632031bfb9b5386e61399b1a52e1cd097e175`。

## 尚未完成

此采集不包含文字、直线荧光笔、更多笔宽和样式、曲线骨架、平直尺子的完整边界、
多个重叠图层、多输出及混合比例。尺子旋转间隔仍需按完整刻度条带避让规则对照；
全部专用控制点、窗口损坏范围和 Qt 界面像素仍需完成原生审计。
矩形高亮、聚光和反色模式、尺子属性栏避让、文字导航字号和最终冷启动仍属于完整目标。
