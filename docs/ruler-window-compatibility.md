# 尺子窗口布局对照

本次修正长度尺数字条带、刻度间隔、旋转连接点，以及旋转控件的局部栅格一致性。
完整 Rust 重写目标仍在进行。本文不代表完整 Qt 界面、文字像素或冷启动审计已经完成。

## 原版依据与反馈循环

只读来源为原版 `qt/src/ruler/ruler_painter.cpp`、`shot_window_ruler_painter.cpp`、
`shot_window_hit_testing.cpp`、`ruler/ruler_geometry.cpp` 及界面字体实现。
采集器保留完整尺子绘制翻译单元，读取内部数字条带布局和字体度量；窗口旋转入口、
允许范围和选区归一化执行完整生产函数。图像定点映射及窗口边界使用明确的状态提供器。
探针不创建可见窗口，文字与放大镜依赖分支执行即失败。

原先旋转测试在固定笔宽六、原图与窗口比例八比一时失败：Qt 把手为 `(320, -40)`，
旧 Rust 为约 `(320, -42.66667)`。扩大矩阵后，旧实现还把部分二十窗口像素连线
缩短到一点二五像素。首轮失败保留在 `target/ruler-window-before-r1.log`、
`target/ruler-window-before-r2.log` 和 `target/ruler-layout-before-r1.log`。

三个排序假设分别为固定刻度留量没有包含完整条带、图像比例重复影响连线长度，
以及轴心或窗口投影不一致。完整 Qt 输出确认前两项；后续窗口状态对照又确认，
临界贴边时必须按当前窗口坐标和原版双精度运算顺序决定换侧。

## 实现

`src/features/ruler/tick_layout.rs` 计算两条完整数字条带、换侧、单轴约束、
顶部留量及旋转后的刻度范围。横向条带左右伸出最长数字半宽，纵向条带宽度向上取整。
未旋转时只判断垂直轴，旋转时判断整条条带。主刻度从原版八段估算开始，
沿一、二、五系列增加；刻度结束值按原图像素四舍五入。

`src/fonts/ruler.rs` 按需定向匹配 `Sans Serif` 半粗体，字形前进量使用六十四分之一
窗口像素精度。字号先在窗口中取整，再应用六像素下限。度量缓存最多二百五十六项，
首次使用尺子之前不会为此读取系统字体。其他界面仍使用各自已有字体。

窗口投影随绘制上下文进入子缓冲，局部缓冲原点不参与换侧。数字绘制、旋转把手、
编辑入口和损坏范围共享布局。顶部刻度留量受测量样式影响，旋转连线固定为二十
窗口逻辑像素。平直尺命中框补足一个原图像素，正文保留真实测量尺寸。

尺子选中虚线、旋转连线及控制点使用固定物理网格。真实旋转输入曾在一点五倍
设备比例下出现局部与完整帧差异；改为固定网格后，相关逐像素回归通过。
失败图像保留在 `target/ruler-partial-failure-r1.png`、
`target/ruler-whole-failure-r1.png`，日志为 `target/ruler-frame-after-r1.log`。

## 验证范围

独立 Qt 采集为 `target/ruler-chrome-reference-r2/`，来源和输出指纹见其中 `index.json`。
一万零二百四十场景涵盖四种投影、四种截图补偿、五种笔宽、四种旋转、四种
窗口/选区/平移上下文和八种矩形，包含边缘、分数、平直及退化尺寸。
布局、字号、换侧、允许范围、刻度间隔和旋转连接点逐项比较，矩形及连接点
误差阈值为零点零二窗口像素。相同矩阵还通过真实编辑入口读取允许范围与连接点。

八十一个带来源指纹的 Qt 样本保存到 `tests/fixtures/qt-ruler-chrome/samples.json`，
普通 Rust 测试无需启动 Qt 即可检查这些样本。完整矩阵使用：

```sh
MARK_SHOT_RULER_REFERENCE=target/ruler-chrome-reference-r2/reference.json \
MARK_SHOT_HIT_REFERENCE=target/annotation-hit-reference-r4/reference.json \
cargo test --lib --locked annotation_style::ruler -- --include-ignored
```

六种真实长度尺输入组合覆盖大小图片、一至二倍设备比例、放大、缩小和水平尺。
测试先检查实际绘制圆心，再发送旋转输入，使用 `monitor_dirty_rect()` 返回范围
进行局部合成，并与同一状态完整生产帧逐像素比较。角度尺另有真实缩放与范围调整回归。

私有 Sway 脚本为 `scripts/wayland-ruler-chrome.py`，只验证普通、完整容纳的数字条带。
它检查实际把手圆心、旋转手势、撤销恢复像素、原图尺寸导出和空闲资源。上一命中
候选 R5 在同一脚本中未能于 Qt 参考位置显示把手，失败现场保留于
`target/ruler-negative-sway-r2/`。首次 Python 环境准备失败单独保留于 R1，不计作负对照。

最终候选及完整验证清单分别使用 `target/ruler-window-candidate-r3/mark-shot` 和
`target/verification-ruler-window-r3/result.json`。R1、R2 候选、原始 Qt 输出、
失败日志和已有发布文件均保留；候选尚未发布。

## 未覆盖项目

- 完整刻度数字及所有控件的 Qt 光栅像素对照
- 测量说明的字体、窄范围行为、全部避让及工具属性栏避让
- 原生边缘换侧、平直尺、混合输出及其他合成器
- 持续绘制帧率分布、最终完整版本冷启动和关闭资源审计

空闲样本不能证明冷启动性能，也不能证明持续绘制帧率。完整功能、界面和性能目标继续保留。
