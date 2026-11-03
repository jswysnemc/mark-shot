# 界面主题检查

## 本次完成

| 界面 | 调整 |
| --- | --- |
| 滚动截图控制栏 | 从截图窗口传递 `ToolbarPalette`，按钮及图标使用同一配色 |
| 滚动截图预览 | 背景、文字、拼接长度、图片细边框和拖动提示适配深浅主题 |
| GNOME 滚动预览 | 新增可选的 `SetScrollPreviewTheme` 接口，旧预览接口保持兼容 |
| 截图操作提示 | 使用工具栏面板、边框与文字颜色 |
| 文本编辑右键菜单 | 不再固定为深色，跟随截图会话主题 |
| 截图历史 | 保留原有主题同步，改为单行操作栏、紧凑缩略图和底部设置 |

GNOME 扩展需要更新后才会使用新增主题接口。Qt 渲染与扩展样式函数已有测试，真实 GNOME Shell 显示效果尚未验证。

## 第二轮补齐

截图窗口内的控件统一使用会话的 `ToolbarPalette`；截图窗口之外的浮动面板通过 `configuredToolbarPalette()` 按配置读取。暗色主题保持原有像素配色，只为亮色主题生成新颜色。

| 界面 | 相关文件 | 调整 |
| --- | --- | --- |
| 选区尺寸和比例面板 | `qt/src/selection_aspect/selection_hud_colors.cpp` | HUD 底板、文字、行内输入框和比例弹层改用 `HudColors` |
| 显示器选择器 | `qt/src/display_capture/display_capture_picker_style.cpp` | 主面板、卡片、悬停操作栏与按钮使用同一明暗配色 |
| 启动取色面板 | `qt/src/shot_window_startup_color_dialog.cpp` | 面板、格式行、按钮与色块描边跟随主题 |
| 扫码结果窗口 | `qt/src/shot_window_code_scan_result_window.cpp` | 外框、文本框与按钮使用截图会话配色，不再混用系统调色板 |
| 录制控制栏和倒计时 | `qt/src/recording/ui/recording_control_bar.cpp`、`recording_countdown.cpp` | 背景、文字与按钮悬停跟随主题，停止图标保留红色警示 |
| 截图辅助提示 | `qt/src/startup_shortcut_hint.cpp`、`selection_loupe.cpp`、`shot_window_recording_status_overlay.cpp` | 快捷键提示、放大镜说明和录制状态面板跟随主题 |
| 截图覆盖层标签 | `qt/src/ui/overlay_chip_colors.h` | 选区尺寸、缩放比例、粗细预览、取色值与尺子悬停说明共用标签配色 |
| 工具提示和普通对话框 | `qt/src/settings/application_theme.cpp`、`qt/src/ui/theme.cpp` | 启动时即按配置主题设置全局调色板与工具提示，浅色工具提示改用深色细边 |

尺子测量标签会写入导出图片，保持原有配色。

截图内容、导出图像中的标注、捕获边框、状态警示色及透明棋盘格不应简单地随界面主题反转。
