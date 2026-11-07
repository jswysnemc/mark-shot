# 尺子测量说明对照

本轮修正测量说明的字体、完整单行尺寸、窗口投影、避让及实时调整顺序。
完整 Rust 重写目标继续进行；本说明只记录测量说明的实现和验证范围。

## 原版行为与实现

- 原版 `rulerFont` 使用界面无衬线半粗体，字号为 `round(13 × unit)`，最低六个绘制目标像素。
- `rulerLabelSize` 使用完整单行字宽和行高，分别向上取整，再增加二十和十个样式单位。
  窄选区只平移说明，不能改为多行、缩小字号或缩小标签。
- 三点角度说明先放在角平分线方向，退化角度仍显示零度或平角；两点草稿不显示角度说明。
- 长度说明避开全部长度尺实际刻度范围及已经放置的说明，候选位置依次为下、上、右、左。
  约束后的候选仍无法避开时，原版使用第一个候选。
- 原版调整持久标注时直接更新原始数组项，说明顺序不变。
  Rust 调整入口暂时移除原件，因此说明来源按 `dragged_index()` 插回原始位置；新建草稿追加。
- 字体取整、窗口平移和刻度条带均参与当前绘制目标布局，导出重新计算字号和字体度量。
  导出的完整说明尺寸不等于把窗口标签尺寸连续放大。

`Measurement::style_scale` 保存双精度比例。真实文件创建入口直接计算原图尺寸和稳定视口之比；
普通笔宽与其他标注继续使用既有单精度画布单位。布局运算保留原版逐项乘除和加减顺序，
`QRectF::adjusted` 的 `width + gap + gap` 不能合并为 `width + 2 × gap`，否则临界边界可能改变避让。

实现按职责分布在 `features/ruler/label_model.rs`、`label_geometry.rs`、`layout_rect.rs`、
`tick_layout.rs`、`editor/ruler_labels.rs` 和 `renderer/ruler_labels.rs`。测量说明在全部标注正文之后绘制。

## 采集与回归

`scripts/ruler-label-reference.py` 只读编译原版 `qt/src/` 的完整 `ShotWindowRulerPainter::drawLabels`、
`ruler_painter.cpp`、`angle_painter.cpp` 和 `ruler_geometry.cpp`，并使用生产窗口单位、允许范围及映射函数。
采集使用不可见 Qt 平台和固定九十六 DPI，不连接用户桌面。

共 2,880 个场景覆盖四个投影倍率、四个截图比例、三个笔宽、三个选区和平移上下文、
十类持久、草稿、混合、平直和退化组合，分别执行窗口和导出目标。
每个场景都用生产几何报告重画标签，并与完整 Qt 入口逐像素比较，确保采集报告可靠。
Rust 回归逐项比较文字、顺序、字号、样式单位、完整尺寸和位置；渲染坐标转换允许小于 0.03 像素误差。

完整采集与前期失败日志分别保留在 `target/ruler-label-reference-r1/`、
`target/ruler-label-reference-r2/` 以及 `target/ruler-label-before-*`、`target/ruler-label-after-*`。
R1、R2 不覆盖历史采集。`scripts/ruler-label-fixture.py` 校验完整采集 SHA-256 后提取 99 个固定场景，
保存到 `tests/fixtures/qt-ruler-labels.json`，包含此前字体和临界避让失败样本。

普通回归还通过真实输入创建两个尺子并调整第一项，验证说明来源保持原顺序，
以及单项实时说明数量和生产损坏范围重绘像素。
`tests/ruler_label_render.rs` 检查六、十三、一百八十二像素字号在分数输出比例及负原点下的局部绘制。

## 性能与光栅范围

说明背景和字形使用固定 128×128 物理网格，损坏范围只改变最终交集。
公共字形绘制先排除当前缓冲外的字形，只为相交的遮罩部分创建像素图，
原始遮罩、颜色运算和整数像素定位保持不变。

独立优化构建的性能入口为：

```sh
CARGO_TARGET_DIR=target/ruler-label-performance cargo test --release --locked \
  --test ruler_label_render measure_ruler_label_render_cost \
  -- --ignored --nocapture --test-threads=1
```

分配计数记录预热后的请求次数和累计请求字节，不等于同时驻留内存。
性能矩阵覆盖两种设备比例、三种样式单位、完整叠加及九十六乘四十八窗口损坏范围。
优化前的大字号完整叠加在 1.2 倍设备比例下，中位耗时 81.28 毫秒，每帧请求 93,403,311 字节。
基线及优化前的完整帧保留在 `target/ruler-label-render-baseline-r1/`，
测量日志为 `target/ruler-label-render-before-r2.log`。

两次独立优化的记录保留在 `target/ruler-label-render-after-r1/` 和 `after-r2/`。
第二次优化把没有整幅十字线的叠加缓冲限制到可见说明范围。
在 1.2 倍设备比例下，十三像素字号完整叠加中位耗时从 6.21 降至 0.26 毫秒，
一百八十二像素字号从 81.28 降至 9.44 毫秒，请求字节从 93,403,311 降至 3,715,723。
三个字号的完整帧 PNG 在优化前后具有相同 SHA-256，局部生产绘制回归也通过。
这些耗时只覆盖说明叠加，不代表完整应用帧率或冷启动；计数也不代表 RSS。

本轮默认候选与验证记录分别使用 `target/ruler-label-candidate-r1/` 和
`target/verification-ruler-labels-r1/`，保留尺子窗口 R3 候选及现有发布文件。

三十张 Qt 和 Rust 同尺寸透明说明层保留在 `target/ruler-label-reference-r2/` 与
`target/ruler-label-frames-r1/`。六例并排画面已人工检查，整体排布符合采集几何，
但字形及圆角抗锯齿仍有差异；两个完全不可见场景没有像素差异，其余场景不能报告完全一致。
详细差异范围和计数见后者的 `pixel-comparison.json`。

尺子属性栏避让、悬停坐标与颜色说明、完整界面及字体光栅对照、原生混合输出、
所有截图比例的导出映射和最终冷启动审计仍待完成。
