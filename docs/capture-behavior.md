# 截图行为与工具栏设置

“设置 → 截图与标注”提供独立的双击动作、中键单击动作、显示器冻结范围、隐藏自身窗口和 KDE KWin 截图开关。

- 双击：在有效选区空白处执行，默认复制并关闭；文字编辑、尺子和标注控制点优先
- 中键单击：默认不执行完成动作；可改为复制、保存、另存为、贴图或取消。超过拖动阈值后仍然平移，即使指针回到起点也不会触发单击
- 显示器冻结：默认冻结全部显示器，可改为光标所在显示器。`--monitor`、`--display` 和 `--all-outputs` 优先于默认配置；普通区域截图设置不缩减全屏标注范围
- 隐藏自身窗口：捕获前协调贴图、设置、OCR 结果与编辑窗口临时隐藏，结束后恢复。存在自身窗口时会等待隐藏确认及约 350 毫秒的桌面收起动画；窗口未能确认时取消捕获，避免误拍自身界面。捕获进程异常退出也会释放隐藏租约
- KDE KWin：关闭后普通 KDE 截图使用桌面门户，授权窗口由桌面管理

“设置 → 工具栏”提供跟随选区、顶部、底部、左侧和右侧停靠，以及图标尺寸（12–48 像素）、属性字号（8–24 像素）。主栏和侧栏的按钮内容仍通过“主栏、侧栏与工具池”编辑，Esc 返回工具栏设置。

工具栏位置下方提供实时位置示意，选中不同位置后立即更新选区、主栏、侧栏与属性栏布局。预览随窗口宽度缩放，并沿用当前明暗主题。

固定停靠后不随选区移动；空间不足时副栏移到主栏靠屏幕内部的一侧，属性栏避让两条工具栏。图标、按钮、点击区域共同调整尺寸，属性控件在窄屏下分组换行。全屏标注沿用停靠边缘；排列切换在侧边和横向边缘之间切换，左侧配置会保留左侧方向，仅影响本次会话。图片编辑窗口保留原有位置策略。

```toml
[capture]
freeze_scope = "all-screens" # all-screens | cursor-screen
hide_own_windows = true
kde_kwin_screenshot = true
double_click_action = "copy" # none | copy | save | save-as | pin | cancel
middle_click_action = "none"
toolbar_position = "selection" # selection | top | bottom | left | right

[toolbar]
icon_size = 22
font_size = 11
```

已有 Qt JSON 配置沿用 `capture.freezeScope`、`capture.hideOwnWindows`、`capture.doubleClickAction`、`capture.middleClickAction`、`capture.toolbarPosition`、`capture.wayland.kde.kwinScreenshot.enabled`、`toolbar.iconSize` 和 `toolbar.fontSize`。设置保存更新原文档格式，并保留其他配置字段。
