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

## 已发现但本次未修改

| 界面 | 相关文件 | 遗留问题 |
| --- | --- | --- |
| 选区尺寸和比例面板 | `qt/src/selection_aspect/selection_hud_painter.cpp`、`selection_size_field.cpp` | 自绘文字和编辑框仍使用固定深色配色 |
| 显示器选择器 | `qt/src/display_capture/display_capture_picker.cpp` | 主面板固定浅色，悬停操作栏固定深色 |
| 启动取色面板 | `qt/src/shot_window_startup_color_dialog.cpp` | 面板及操作按钮固定浅色 |
| 扫码结果窗口 | `qt/src/shot_window_code_scan_result_window.cpp` | 深色外框与系统调色板编辑器可能不一致 |
| 录制控制栏和倒计时 | `qt/src/recording/ui/recording_control_bar.cpp`、`recording_countdown.cpp` | 背景和文字固定深色主题 |
| 部分截图辅助提示 | `qt/src/startup_shortcut_hint.cpp`、`selection_loupe.cpp`、`shot_window_recording_status_overlay.cpp` | 提示背景及文字尚未接入会话配色 |
| 工具提示和部分对话框 | `qt/src/main.cpp`、`qt/src/ui/theme.cpp` | 启动时仍可能跟随系统而非配置主题 |

截图内容、导出图像中的标注、捕获边框、状态警示色及透明棋盘格不应简单地随界面主题反转。
