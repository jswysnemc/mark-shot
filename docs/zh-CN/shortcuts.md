# 快捷键

[English](../shortcuts.md) | **简体中文**

本页列出编辑器内的默认按键。启动截图的全局快捷键是另一项功能，见[桌面集成](desktop-integration.md)。

## 完成动作与通用操作

| 动作 | 默认按键 | 配置项 |
| --- | --- | --- |
| 确认并执行默认完成动作 | `Return` | `accept` |
| 复制 | `Ctrl+C` | `copy` |
| 保存 | `Ctrl+S` | `save` |
| 另存为 | `Ctrl+Alt+S` | `save_as` |
| 贴图 | `Ctrl+P` | `pin` |
| 取消 | `Escape` | `cancel` |
| 撤销 | `Ctrl+Z` | `undo` |
| 重做 | `Ctrl+Shift+Z`、`Ctrl+Y` | `redo` |
| 全选标注 | `Ctrl+A` | `select_all` |
| 删除 | `Delete`、`Backspace` | `delete` |
| 滚动长截图 | `Ctrl+Shift+S` | `scroll_capture` |
| 截图历史 | `Ctrl+H` | `history` |
| 设置 | `Ctrl+,` | `settings` |
| 切换浅色与深色主题 | `Ctrl+Shift+L` | `toggle_theme` |
| 隐藏或显示工具栏 | `Space` | `toggle_ui` |
| 开关指针放大镜 | `Shift+M` | `toggle_magnifier` |
| 增大粗细 | `]` | `size_up` |
| 减小粗细 | `[` | `size_down` |

## 录制

| 动作 | 默认按键 | 配置项 |
| --- | --- | --- |
| 录制视频 | `Ctrl+Shift+R` | `record_video` |
| 录制 GIF | `Ctrl+Shift+G` | `record_gif` |
| 暂停或继续 | `Ctrl+Shift+P` | `record_pause` |
| 停止 | `Ctrl+Shift+E` | `record_stop` |

## 工具

| 工具 | 默认按键 | 配置项 |
| --- | --- | --- |
| 选区 | `S` | `tool_selection` |
| 选择 | `V` | `tool_pick` |
| 画笔 | `P` | `tool_pen` |
| 荧光笔 | `H` | `tool_highlighter` |
| 直线 | `D` | `tool_line` |
| 箭头 | `A` | `tool_arrow` |
| 矩形 | `R` | `tool_rectangle` |
| 椭圆 | `C` | `tool_circle` |
| 形状标记 | `K` | `tool_marker` |
| 文字 | `T` | `tool_text` |
| 序号 | `N` | `tool_number` |
| 编号箭头 | `Shift+N` | `tool_numerated_arrow` |
| 滤镜 | `M`、`X` | `tool_redact` |
| 放大镜 | `Z` | `tool_magnifier` |
| 尺子 | `U` | `tool_ruler` |
| 激光笔 | `G` | `tool_laser` |
| 取色器 | `I` | `tool_eyedropper` |
| 文字识别 | `O` | `tool_ocr` |

## 移动和调整选区

| 动作 | 默认按键 | 配置项 |
| --- | --- | --- |
| 移动 1 像素 | 方向键 | `move_left`、`move_right`、`move_up`、`move_down` |
| 移动 10 像素 | `Shift` 加方向键 | `move_*_fast` |
| 调整 1 像素 | `Alt` 加方向键 | `resize_left`、`resize_right`、`resize_up`、`resize_down` |
| 调整 10 像素 | `Alt+Shift` 加方向键 | `resize_*_fast` |

步长由 `[selection]` 中的 `nudge` 和 `nudge_fast` 决定。

## 没有默认按键的动作

下列动作可以通过工具栏使用，也可以为其指定按键：
`open_with`、`extensions`、`upload`、`translate`、`scan_code`、`clear_annotations`、
`toggle_scope`、`toggle_toolbar_layout`。

## 绘制时的修饰键

| 修饰键 | 作用 |
| --- | --- |
| `Shift` | 矩形和椭圆绘制为正方形或正圆；直线、箭头和旋转吸附到 45 度的倍数 |
| `Ctrl` | 缩放标注时保持长宽比 |
| 连按两次 `Ctrl` | 复位画布的缩放和位置 |

## 修改按键

打开**设置 → 快捷键**。每个动作都可以重新录入、追加按键、清除或恢复默认，按键冲突时界面会给出提示。

也可以编辑配置文件的 `[keys]` 表。取值为单个按键或按键列表，空列表表示取消绑定：

```toml
[keys]
copy = "Ctrl+C"
redo = ["Ctrl+Shift+Z", "Ctrl+Y"]
upload = "F8"
toggle_ui = []
```

从工具栏移除某个工具不会禁用它的按键。
