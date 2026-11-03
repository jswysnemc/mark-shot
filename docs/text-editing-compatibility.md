# 内联文字事务与字符菜单对照

参考原版 `qt/src/shot_window_text_editor.cpp` 与 `qt/src/shot_window_modes.cpp`。
开发探针只读链接原版 395 个对象；源码、构建对象和用户配置均保持不变。

## 事务边界

`editor/text_session.rs` 保存独立预览，使用专用编辑器身份，不消耗持久标注编号。
编辑期间已有标注正文不变，持久图层隐藏正在编辑的原对象，屏幕动态层显示预览。

- Esc、Ctrl+Enter 提交；Enter 换行；外部左键按原版入口提交或重新编辑其他文字
- 提交裁掉尾部空白，保留开头缩进，并规范不换行空格、段落与行分隔符
- 新内容非空才分配标注编号并保存一次全局历史；空草稿和全空白不产生标注
- 重新编辑保留原身份；内容未变不记录提交历史；清空已有内容后提交删除
- 选择工具的编辑按钮不切换工具；重新编辑先隐藏属性栏，独立样式变化后重新显示
- 字符区域左键拖动选择文字，不启动标注拖动
- 保存、另存为、贴图、无字符选区的截图复制先提交；有字符选区的复制不提交

`editor/text_session_history.rs` 分开保存正文增量与字体格式操作。正文记录保留输入时的字体、
光标和字符选区；格式记录只保存前后字体，不复制整篇正文。新文档初始化有一项格式记录，
已有文档有两项，与真实 Qt 初始化操作对应。
连续字符输入合并；换行、选区替换和独立编辑操作结束输入组。
Ctrl+Z、Ctrl+Y、Ctrl+Shift+Z 只使用局部历史，不回退工具默认值或持久对象样式。

`editor/text_session_format.rs` 将当前文档字体与实际属性来源分开保存。字体家族、字号、
粗体和斜体可逐项撤销与重做，恢复实际字形和排版，而不是只改变属性按钮。格式动作清除
字符选区，局部撤销不恢复动作前的选区；撤销格式后在原位置继续输入仍可接续原输入组，
移动光标后的插入则单独记录。空文档使用最新来源字体，重做正文恢复输入时的字体。

普通面板刷新、字号字段导航和边界同值不追加格式记录；显式字号 setter 保留原版格式操作。
属性取色重新设置文档格式，独立透明度滑条只改变颜色，不插入字体历史。提交使用最后一次
实际属性来源字体，局部撤销字体不覆盖原版提交元数据。字号字段 Return 确认后提交文字，
但保留字体面板和截图会话，不触发截图导出；字段以外的 Enter 仍用于文字换行。

## 字符菜单

原版菜单在文字视口右键按下时打开，不移动光标或改变字符选区。
顺序为撤销、重做、分隔线、剪切、复制、粘贴、删除、分隔线、全选。
撤销和重做依据局部历史；剪切、复制和删除要求非空选区；粘贴要求非空文字剪贴板；
全选要求非空文档。菜单删除不执行向后删除，无选区时禁用。

菜单在关闭后交付动作。Esc 只关闭菜单，第一次外部点击只关闭菜单；菜单存在时
全部按键与滚轮不进入文字或截图快捷键。方向键和 Tab 跳过分隔线、禁用行并循环导航，
Home、End、PageUp、PageDown 定位首尾，Enter 或 Space 执行当前可用行。
没有高亮时 Enter 不执行动作。右键不触发菜单动作，左键释放才触发。
仅打开并取消菜单不打断连续输入撤销组；只有实际编辑操作或光标、选区变化结束输入组。
打开菜单会清除此前鼠标双击记录；关闭后首次左键只定位光标，不沿用旧序列选中整词。

菜单使用 `ui/context_menu/` 共用组件，原版深色令牌为背景 `(24,27,35,246)`、
边框 `(255,255,255,30)`、圆角八像素、行圆角五像素、外边距四像素、分隔行九像素。
当前原版菜单字体为 DejaVu LGC Sans 十二点；按字体定向加载与整数度量得到
225×229 菜单，普通行二十九像素。菜单宽度由文案、快捷键和留白计算，不固定为贴图宽度。
菜单先沿锚点向左或向上翻转，再约束到当前输出；分数缩放按实际输出比例绘制。
缓存仅在布局、比例或高亮变化时重建，局部重绘只合成已经恢复的像素，避免透明背景叠加。

## 正文光标与焦点

原版实际 `QApplication::cursorFlashTime()` 为 1000 ms，正文光标每 500 ms 切换。
光标使用文字前景颜色和透明度，不采用字段光标的白色。原版不透明红色采样为
`(255,77,77,255)`；128 透明度的 PNG 解预乘采样为 `(255,78,78,128)`，后者包含
八位预乘舍入，不能直接以 RGB 相同判定透明度正确。

- 键盘输入恢复可见相位，但不推迟已经存在的闪烁截止
- 正文鼠标按下与释放重新建立半周期，按住期间仍可闪烁
- 选中文字时插入光标继续闪烁，字符选区与光标分别绘制
- 字体字段、父窗口失活和字符菜单焦点隐藏正文光标，并取消周期唤醒
- 返回正文焦点立即显示光标并重新建立周期，不改变正文、字符选区或局部历史
- 提交文字后结束光标计时，导出图像不包含光标

`tests/fixtures/qt-text-caret.json` 冻结十一组真实 Qt 场景、103 个采样。原版只读链接
395 个对象，过期依赖为零；最新探针构建与原始画面位于 `target/qt-text-caret-final/`，
冻结样本来自 `target/qt-text-caret-r2/`。最初探针错误地按白色识别光标，结果保留在
`target/qt-text-caret/`，不把其中零计数作为正文没有光标的证据。

Rust 将光标状态独立于正文编辑与字符选区。计时只通知窄动态区域，持久标注图层不重建。
Wayland 主表面离开事件携带输出身份，旧输出的延迟事件不能覆盖新输出焦点。
普通窗口的私有回归通过独立历史窗口转移真实键盘焦点，检查失活期间没有光标重绘，
返回后原正文和选区保持不变。

局部背景恢复后曾重复绘制整个草稿，造成字形与半透明像素累积。当前只在已恢复的范围
复制背景并绘制，再直接覆盖对应像素；完整范围复用原画布。原始像素相位随子缓冲传递，
避免分数比例往返导致圆角覆盖变化。确定测试比较 0.75、1、1.2、2 倍比例的完整帧与
局部帧，字符选区和透明背景逐字节一致。

修改前失败证据保留在 `target/text-caret-color-before.log`、
`target/text-caret-gui-before.log`、`target/text-caret-dispatch.log` 和
`target/text-caret-fractional.log`。格式回归保存两个原始相位，只允许一至三像素宽的
红色光标差异；它选择其中一个完整原始帧，不拼接颜色通道，字形恢复仍执行单像素断言。
聚焦正文只允许低频窄光标更新；提交和失活状态继续要求 CPU 与帧数稳定。

## 验证与边界

### 此前光标构建

截图正文的后续原生输入法接入与当前发布指纹见 `docs/text-input-method-compatibility.md`。

2026-11-04 release 为 43075064 字节，SHA-256 为
`40043628704fa34d4be2e5ca07ebb2038e5ff620a9a0254760b5e8c630c73e89`。
Rust 工作区 709 项测试通过、10 项忽略，Python 109 项通过；严格 Clippy、格式、Ruff
和构建通过。当前索引为 `target/verification-text-caret/result.json`，详细 GUI 证据使用
`target/wayland-text-caret-optimized-*/`，各项记录同一构建指纹。

十五组实际回归覆盖 niri 层表面与普通窗口光标、Sway 光标与完整格式历史、四组原生
菜单、文字事务、字体、两组透明度、字号滚轮、贴图菜单及双输出几何。普通窗口通过
历史窗口转移真实键盘焦点，失活和提交均停止光标重绘；Sway 格式场景一秒仅提交两个
窄光标帧，单线程，CPU 时钟增量为零。原图导出仍为 1600×900。

整屏损坏原先分配第二份像素缓冲，释放后分配器保留空闲页。当前直接复用原画布，
部分损坏继续隔离绘制；同一私有会话各六次采样，普通编辑器空闲 RSS 中位数从 56510
降至 51502 KiB。诊断、修改前失败和实际分布均在索引中保留，不用实验内存回收替代
产品修复。本阶段未重新证明 Qt 与 Rust 整个界面逐像素一致。

### 原生弹出表面

文字菜单不再绘入截图画布。`backend/wayland/overlay_popup.rs` 管理 `xdg-popup`：
普通窗口使用所属 `xdg_surface`，层表面通过 `get_popup` 关联；首次提交前使用真实输入座席
与按钮按下序列号执行抓取。按下记录随实际事件分派清理，未来排队输入不会挤掉当前身份。

`types/popup.rs` 只传输会话编号、逻辑尺寸、输出比例和已经排版的像素。
`app/text_menu_surface.rs` 负责菜单模型与协议衔接；菜单本地指针不修改主画布指针或字符选区。
配置位置来自合成器，初始左上角与传入的实际指针点一致，不增加一像素偏移。
存在视口协议时支持正数分数比例；缺少视口协议时调用方使用合法整数缓冲比例。

菜单缓存与普通画布测试共用同一绘制组件。协议层最多保留三份当前尺寸缓冲，等待帧回调时
保留最新像素；空闲不轮询。关闭释放菜单像素与缓冲引用，共享内存池容量仍由覆盖层保留。
原生角色销毁请求先于菜单动作；关闭请求已经提交后才执行待交付的正文命令。

合成器取消或弹出表面失去键盘焦点时，只结束当前菜单，不提交文字、不取消截图。
所有弹出事件携带独立会话编号，过期指针、按键及关闭通知不影响新菜单。
原版 `ui/context_menu.cpp` 的 `ShortcutOverride` 也会消费全部快捷键，原生菜单对应保持输入独占。

### 此前文字样式状态对照

2026-11-03 的 release 为 43063416 字节，SHA-256 为
`9e6fe70688a35048162c2358623cfe4374e5d06dcce96fccf12544fb0d406581`。
工作区 Rust 测试 689 项通过、10 项忽略，Python 96 项通过；全部目标和功能的严格
Clippy、格式与 Ruff F 类检查通过。
当前构建日志使用 `target/text-format-*-audited.log`，不混用此前格式实现的构建指纹。

`tests/fixtures/qt-text-format-history.json` 保存二十五组真实 Qt 记录，包括新建、已有、
初始格式次数、四类字体动作、取色、背景、透明度、同值 setter、连续格式、空文档、
撤销后继续输入、中间插入、字段导航和字号字段 Return。生产输入路由逐项比较状态与
记录数量；固定样本来自实际输出，不以手写预期代替原版行为。

原版探针位于 `target/qt-text-format-history-r11/`，只读链接 395 个原版对象，过期依赖为零。
Rust 实际状态位于 `target/rust-text-format-history-audited/actual-formats.json`。
格式动作未清除选区、字段导航重复追加历史、透明度误插入字体历史、空闲新键盘未刷新
以及字号字段 Return 未提交文字，各自保留修改前失败日志：
`target/text-format-history-before-r2.log`、`target/text-format-field-before.log`、
`target/text-format-opacity-before.log`、`target/text-format-seat-before.log` 和
`target/text-format-field-submit-before.log`。

同一构建的私有 Sway 完整流程位于 `target/wayland-text-format-history-sway-audit-r4/`，
字号、粗体、斜体和字体家族撤销重做逐像素恢复；原生菜单历史、连续输入组、独立透明度、
空草稿、默认值与 1600×900 导出通过。字号字段 Return 提交独立四十点文字并保留面板，
两次全局撤销分别移除新成稿和原成稿，重做严格恢复，没有重复提交历史。
空闲 RSS 为 66296 KiB、单线程，一秒 CPU 时钟增量为零。Sway 最小指针回归也通过，
证据为 `target/wayland-text-popup-pointer-sway-audit-r4/`。

回归对字号先按原图比例取整的行为保持一致：三十点和三十一点在该输出上得到相同字形，
字段样本改用四十点跨越实际栅格，而不是放宽像素比较。90% 透明度按钮不再符合不透明
红色定位条件，因此复用本次调整前从画面取得的按钮位置，再严格确认弹层。此前辅助
场景失败保留在 `target/wayland-text-format-history-sway-audited/`、
`target/wayland-text-format-history-sway-audit-r2/` 和
`target/wayland-text-format-history-sway-audit-r3/`，不计为当前通过结果。
当前完整汇总为 `target/verification-text-format-history/result.json`。

### niri 首次移动边界

当前安装版本为 `niri 26.04 (v26.04-69-g3d2a6738)`。原生菜单经键盘关闭后，第一次
指针移动可能仍返回菜单锚点坐标，随后点击作用于旧文字。最小实际回归命令为
`scripts/wayland-text-popup-pointer.py`，它同时验证抓取销毁之后的协议坐标和新草稿位置。
本轮执行动作与直接取消两组都失败：目标为 `(341, 500)`，实际为 `(672.6289, 359.5352)`。
最新构建证据位于 `target/wayland-text-popup-pointer-niri-audited/` 和
`target/wayland-text-popup-pointer-niri-cancel-audited/`，各自保存启动时构建指纹。

同一个私有 niri 会话中，只读链接的原版 Qt 窗口也收到旧坐标
`(672.62890625, 359.53515625)`。原版实际协议、截图、事件记录及构建证据位于
`target/niri-popup-source/qt-pointer/`。独立诊断再发送一次移动后，坐标恢复为
`(341.3164, 500.75)`；该诊断属于实验构建，不作为最新产品通过结果。

当前光标构建再次复现同一坐标失败，退出码为一；证据为
`target/wayland-text-caret-optimized-pointer-niri/` 与
`target/text-caret-pointer-niri-optimized.log`。Sway 同一最小场景通过，证据为
`target/wayland-text-caret-optimized-pointer-sway/`。光标专项通过不消除该合成器边界。

显式窗口几何、向下取整锚点和销毁前解除映射均未修复问题，实验改动已经撤回。
上游 v26.04 所用 Smithay 的 `PopupPointerGrab::motion` 存在“结束抓取后直接返回”的
路径，与首次事件丢失现象一致；这项源码线索不等同于安装二进制的逐行源码确认。
源码保存于 `target/niri-popup-source/`，来源为
[Smithay 对应提交](https://github.com/Smithay/smithay/blob/ff5fa7df392cecfba049ffed55cdaa4e98a8e7ef/src/desktop/wayland/popup/grab.rs)。
产品不补发移动、不修改原版、不干预用户合成器；失败边界仍然保留。

无并行 GUI 负载时重新运行未缩减的完整场景，仍在透明度草稿位置失败，证据为
`target/wayland-text-format-history-niri-audit-r2/`。它与最小场景核对同一最新构建。
此前并行负载下出现一次粗体重做快照差异，协议同时记录输入延迟，保留在
`target/wayland-text-format-history-niri-audited/`。本轮未单独复现该差异，不把它标为
已经确认根因并修复的产品缺陷。

### 此前原生菜单构建回归

2026-11-03 构建的 SHA-256 为
`b0a46d9a46d0269c8173234302dce0ccdf4789d6f9d45ebeff892a7673db1528`，
工作区 Rust 测试 686 项通过、10 项忽略，Python 测试 75 项通过。
汇总位于 `target/verification-text-popup/result.json`，所有当前 GUI 与性能结果均核对该指纹。

普通窗口、层表面分别覆盖 1.2 倍和 0.75 倍输出。菜单逻辑尺寸为 225×229，缓冲分别为
270×275 和 169×172，合法小比例不回退到一倍。四组原生创建、抓取和销毁次数分别为
14、15、14、15；普通窗口失活收到真实 `popup_done`，返回后正文与字符选区保持不变。
实际输入验证菜单关闭后的首次左键不替换整词，并再次验证取消菜单不打断连续输入撤销组。

证据位于 `target/wayland-text-popup-layer-complete/`、
`target/wayland-text-popup-window-complete/`、`target/wayland-text-popup-layer075-complete/`
和 `target/wayland-text-popup-window075-complete/`。四组菜单空闲 RSS 为 53616–55636 KiB，
全部单线程，一秒 CPU 时钟增量为零。贴图菜单、文字事务、字体属性、独立透明度和
字号滚轮也用该构建重新验证；导出保持 1600×900，不自动创建 OCR 模型目录。

缺失原生抓取、锚点偏移、小比例拒绝和左键双击序列分别保留失败记录：
`target/text-popup-before.log`、`target/text-popup-anchor-wire-before.log`、
`target/text-popup-subunit-scale-before.log` 和 `target/text-popup-button-sequence-before.log`。
当前构建没有重新比较 Qt 菜单的逐像素差异，也未重测缓存模型推理或完整合成器矩阵。

### 此前画布菜单回归

`tests/fixtures/qt-text-editing.json` 保存九组真实提交和局部撤销状态，
`tests/fixtures/qt-text-menu.json` 保存九组实际菜单、可用性、几何和菜单动作结果。
探针分别位于 `target/qt-text-editing/`、`target/qt-text-menu-groups/`，最初菜单探针保留在 `target/qt-text-menu/`。
文字事务真实输入差分位于 `src/app/input/tests/text_session.rs`；
文字菜单真实输入和几何差分位于 `src/app/input/tests/text_context_menu.rs`。
菜单缓存测试比较一倍、1.2 倍与两倍比例下局部恢复和整帧绘制，逐字节一致。
此前画布菜单构建回归位于 `target/wayland-text-menu-r1/`，原有文字事务、字体属性、
两组透明度和字号滚轮也使用同一构建重新验证，汇总见 `target/verification-text-menu/result.json`。
菜单复制、剪切、删除、粘贴、全选及局部撤销重做通过；最终提交只进入一次标注历史，
导出保持 1600×900。菜单空闲 RSS 为 59540 KiB、单线程，一秒 CPU 时钟增量为零。
此前一倍缓冲比较中，原版与 Rust 菜单尺寸一致，各通道平均绝对像素差约 1.3–1.4，尚未逐像素一致；
比较图与原始统计位于 `target/text-menu-native/comparison.png` 和 `target/text-menu-native/comparison.json`。

亮色主题、系统菜单字体覆盖及完整合成器、输出与按钮组合矩阵仍需继续对照。
截图字段输入法后续验证见 `docs/text-field-input-method-compatibility.md`。
独立窗口输入法、完整焦点与字段行为、选区边缘换行与位置回移、完整角点缩放、Unicode 以及
逐像素矩阵仍未完成。文字事务和菜单专项通过不代表完整文字工作流或整个重写完成。
