# 优先界面问题处理

本轮处理置顶图中文菜单缺字和工具滚轮数值提示，复核默认浅色、主题切换及尺子绘制成本。
完整 Rust 重写继续推进；扫码、录像和设置窗口仍按要求排除。

## 置顶图菜单

用户截图中的“始终置顶”等中文显示为缺字方框。置顶图进程仅创建内置西文字库，
菜单绘制没有准备中文回退字体。真实菜单入口回归复现字形编号为零，失败记录保留在
`target/pin-menu-font-before-r1.log`。

`fonts/ui_fallback.rs` 按菜单实际文字定向补充缺字，排除专用序号字体，不扫描全部系统目录。
`backend/wayland/pin/menu_draw.rs` 在排版前准备主菜单及语言子菜单的文字，
重新绘制时复用已加载文件。英文菜单不额外加载回退字体。
`ui/language/pinned_menu.rs` 补齐 `qt/src/ui/i18n.cpp` 和 `i18n_task_windows.cpp` 的原版文案，
修正同一中文菜单中只有部分项目得到翻译的情况。

实际菜单入口验证全部菜单和目标语言字形有效。独立绘制截图保留在
`target/pin-menu-font-frames-r1/chinese-menu.png`。
私有 Sway 的中文主菜单浅色与深色画面已经人工检查，见
`target/priority-ui-sway-r1/checks/pin-menu-light-crop.png` 和 `pin-menu-dark-crop.png`。

## 滚轮数值提示

此前只有图片缩放显示气泡，普通画布工具滚轮仅修改属性。
`app/input/wheel_preview.rs` 在原有分派完成后读取实际数值，
`features/wheel_preview/model.rs` 分别计算像素尺寸、文字点字号、序号气泡直径和倍率。
`layout.rs` 按原版右下、右上、左下、左上四个候选位置选择重叠最少的位置。

`ui/canvas_navigation/size_preview.rs` 绘制实际颜色示意、数值、固定热点十字，
以及二十八至九十六窗口像素内的尺寸轮廓。马赛克使用矩形，其他尺寸使用圆形。
气泡使用现有浅深色主题令牌，字体和边框保持窗口尺寸，不进入原图导出。
提示跟随指针，不延长九百毫秒期限；绘图和拖动期间不显示。
到期清除复用现有事件循环期限，不创建持续轮询线程。

实现以 `shot_window_cursor_preview.cpp`、`annotation_size_preview.cpp` 和
`capture_cross_cursor.cpp` 的行为为依据。十类工具的实际输入回归验证提示出现，
文字点字号、序号直径和不延长期限另有回归。分数设备比例下的局部像素回归
覆盖气泡、颜色和轮廓，不能以这些场景报告所有工具和所有原版像素已完全一致。

## 主题与原生证据

默认浅色和 `Ctrl+Shift+L` 切换已存在于当前源码。私有 Sway 本轮实际验证
编辑器及活动置顶图菜单的浅色、深色和浅色恢复，读取保存后的 TOML 并检查其他配置保留。
原生画笔、编号和文字流程验证提示显示、移动、独立到期；最终复制的原图像素保持不变。
置顶图菜单关闭后八个空闲样本保持单线程，CPU 计数和缓冲提交稳定。
上述结果见 `target/priority-ui-sway-r1/checks/result.json`。

系统 `mark-shot` 和 `mark-shot-rs` 命令指向另一目录
`/home/snemc/workspace/mark-shot-rs-port/build/mark-shot`，当前重写源码位于
`/home/snemc/workspace/mark-shot-little`。本轮保留系统入口和此前发布文件，
通过独立候选提供新构建；不能把其他目录中的程序当作本轮验证对象。

## 性能证据范围

此前测量说明优化把一百八十二像素字号的完整说明叠加从约八十一毫秒降至约九毫秒，
记录见 [尺子测量说明对照](ruler-label-compatibility.md)。该数据只覆盖说明层。

`app/input/tests/annotation_style/ruler_performance.rs` 新增完整生产帧测量，
包括真实指针分派、连续长度或三点角度草稿、悬停层、说明缓存、损坏范围及实际帧渲染。
它采用两种设备比例，预热二十帧后记录六十帧耗时，不把测量结果等同合成器呈现帧率。
本轮一倍设备比例下，长度尺中位耗时 4.25 毫秒，角度尺为 3.25 毫秒；
一点五倍比例下分别为 8.87 和 6.89 毫秒，百分之九十五分位分别为 10.54 和 8.37 毫秒。
记录见 `target/priority-ruler-full-frame-r1.log`，不能由此推断所有图像尺寸和所有合成器的帧率。
完整界面光栅、悬停说明细节、多合成器和冷启动审计留到完整功能推进之后继续处理。
