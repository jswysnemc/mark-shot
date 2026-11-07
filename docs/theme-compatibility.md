# 明暗主题与验证范围

2026-11-07 最新主题候选补齐字体预览、字体滚动条、字号与样式控件、尺子悬停和
画布滚动条。Linux 跨进程主题通知已接入截图、OCR、历史、贴图与存活菜单。默认
release 同构建的七组私有流程和 1,363 项工作区测试通过，候选、证据和验证边界见
[字体与主题通知](font-ruler-theme-verification.md)。Windows 后端继续实施。

后续用户反馈浅色配置下仍有黑色区域，消息未附带截图，具体组件尚未定位，不能视为主题验收通过。
本次代码审计确认图片编辑初始预览 `src/app/image_setup.rs`、导航背景
`src/renderer/canvas_navigation.rs` 与翻译后预览 `src/features/selection_translation/pixels.rs`
曾使用固定 `(28,31,34)` 画布底色。后续修复通过 `src/theme/canvas.rs` 统一显示底色，
`src/app/file_canvas_theme.rs` 重建显示预览并同步原生背景，同时使导航缓存失效。
翻译取消也按当前主题恢复预览，不把旧底色片段写回画面；原件及导出不应用主题底色。

私有 Sway 的旧候选实测在浅色画布返回 `(28,31,34)`，新候选
`target/canvas-theme-candidate-r1/mark-shot`（SHA-256
`eba06f324b482e39b0150d93c77a5c13048499a2341646f1e4ae6920ded496aa`）
通过正常及缩放视图明暗往返，完整透明 PNG 导出与原件逐字节一致。
正常视图底色为浅色 `(238,240,242)`、深色 `(28,31,34)`；导航选区外保留原有遮罩，
对应 `(107,108,109)` 与 `(13,14,16)`，不把遮罩误认为原始底色。
证据为 `target/canvas-theme-green-r2/result.json`，同一文件的字体滚动、搜索、外部配置刷新、
正文及原图导出回归见 `target/canvas-theme-font-r2/result.json`。
三项新增测试覆盖透明原件、已有标注、导航缓存和翻译退出；工作区 1,366 项通过，19 项忽略。
这项修复针对已复现的画布残留，不等于确认用户未附图的区域或全部主题验收完成。

界面默认使用浅色，`Ctrl+Shift+L` 切换并保存浅色与深色。设置窗口仍按本次要求暂不
完成；主题不依赖该窗口。配置使用 `[ui] theme = "light"` 或 `"dark"`，快捷键通过
`keys.toggle_theme` 自定义。

`src/theme/toolbar.rs` 保存原版主副工具栏、属性控件和弹层的明暗角色配色。
`src/config/interface_theme.rs` 继承角色颜色、识别旧模板内置颜色并独立原子保存主题，
保留显式自定义颜色、几何尺寸、注释及其他配置。切换主题不改变标注颜色或导出 PNG。
`src/app/theme_switch.rs` 使当前编辑器缓存和损坏范围失效；原生普通窗口通过
`src/backend/wayland/window/theme_switch.rs` 通知所属内容刷新。

原版角色样本为 `tests/fixtures/qt-theme.json`，配置、继承与旧模板迁移回归位于
`src/config/tests/interface_theme.rs`。这些数值与状态对照不能证明整个界面逐像素一致。

2026-11-05 的实际主窗口验证使用本地二进制
`bb53a056a8e0d57795eb0e5b82fbb4be441eaf1fb4c7bb2789ba8c24e951d356`。
私有 Sway 与隔离 niri 都执行默认浅色、切换深色、恢复浅色，并独立读取保存后的
TOML。工具栏同一采样点分别为 RGB `(236,238,240)` 和 `(15,16,22)`；恢复后配置
仍为浅色，上传端点与其他配置保持完整，最终原图 PNG 与原版外框样本一致。
记录分别位于 `target/wayland-upload-sway-r5/result.json` 和
`target/wayland-upload-niri-r5/result.json`。一次快捷键后等待配置与实际精确像素同时
完成，观察上限分别约为 205 ms 和 300 ms，包含输入、截图等待及采集开销，不能
当作实际呈现时间；没有重复发送快捷键或把未完成画面当作成功。

本次实际验证覆盖主编辑窗口；全部独立窗口、贴图弹层、不同输出比例、跨进程主题
通知和原版全界面逐像素对照仍需完成，整体重写目标保持未完成。

2026-11-06 的优先界面候选新增一倍及一点五倍私有 Sway 验证，实际执行主编辑器和
活动置顶图菜单的浅色、深色与浅色恢复。每次检查保存的配置，主菜单恢复时要求同一区域
完整像素一致。中文菜单字形已修正，稳定菜单保持单线程、CPU和缓冲提交计数不变。
最新结果分别位于 `target/priority-ui-sway-r2/checks/result.json` 和
`target/priority-ui-sway-scale1-r2/checks/result.json`，候选为
`target/priority-ui-candidate-r2/mark-shot`；此前记录仍保持各自构建范围。
跨进程通知、全部独立窗口和完整光栅对照尚未由这些流程证明。

2026-11-07 修正旧配置导致的首帧深色残留。此前 `[ui] theme="light"` 与旧模板
内置深色值同时存在时，启动仍保留深色共享背景；组件中的旧 `#1E2124FA` 等内置值
也会阻止运行时继承。启动现在立即按明确主题更新内置共享颜色，未设置主题时使用浅色。
`src/config/component_theme.rs` 识别旧模板组件角色，自定义颜色及绘图工具颜色保持原样。
失败回归位于 `target/light-theme-startup-before-r1.log` 和
`target/theme-components-before-r1.log`。

`scripts/wayland-theme-components.py` 用旧配置实际启动编辑器、显示文字属性与首次模型
引导，再切换深色和恢复浅色。显式浅色与未设置主题两种入口均通过；原图为固定灰色，
不会把原图亮度误计为组件浅色。最新记录为 `target/theme-components-final-r2/checks/result.json`，
构建与 `target/light-scroll-candidate-r2/mark-shot` 相同。模型面板浅色区域为 145,926 个
像素，深色后为 792 个；恢复时完整画面像素一致。模型标题、推荐文字与选中状态改用
不透明强调色，避免误用半透明背景色导致文字过淡；可见强调像素为 1,086 个，旧构建
失败证据为 `target/model-emphasis-before-r1.log`。这些入口与配置矩阵共同证明此次
修正，全部其他独立窗口及跨进程通知仍须分别验证。
