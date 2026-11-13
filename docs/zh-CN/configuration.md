# 配置

[English](../configuration.md) | **简体中文**

大多数选项可以在设置窗口中修改。全部设置保存在一个 TOML 文件中，也可以手动编辑。

## 设置窗口

打开方式：运行 `mark-shot --settings`，在编辑器中按 `Ctrl+,`，或点击工具栏的设置按钮。

| 页面 | 内容 |
| --- | --- |
| 外观与语言 | 主题、语言、托盘图标 |
| 截图与标注 | 完成动作、双击与中键动作、冻结的显示器范围、初始工具、窗口与区域识别 |
| 屏幕录制 | 格式、质量、编码器、帧率、音频、保存目录 |
| 工具栏 | 工具栏位置和尺寸，主工具栏与动作工具栏的按钮 |
| 快捷键 | 编辑器的全部按键，以及全局截图快捷键 |
| 文字识别 | 识别引擎、运行方式、已安装的模型 |
| 翻译服务 | 目标语言、翻译服务及其凭据 |
| 存储与历史 | 保存目录、文件名格式、截图历史 |
| 贴图与外框 | 贴图边框、带实时预览的导出外框 |
| 上传与集成 | 上传 |
| 高级 | 日志、关于 |

`Ctrl+F` 搜索全部设置项。按 `Enter` 或离开字段时保存，`Esc` 放弃本次编辑。
底栏显示修改的生效时机和配置文件路径。文件中的注释和未知字段会保留。

## 配置文件

| 平台 | 默认路径 |
| --- | --- |
| Linux | `~/.config/mark-shot/config.toml`（遵循 `XDG_CONFIG_HOME`） |
| Windows | `%APPDATA%\mark-shot\config.toml` |

`--config 文件` 仅对本次运行使用另一个文件。

首次启动只写入最小配置。缺少的字段使用内置默认值，因此删除某个字段即可恢复默认。
下面的命令输出带注释的完整配置参考：

```sh
mark-shot --print-default-config
```

如果目录中没有 `config.toml`，但存在 Qt 版本留下的 `config.json`，Mark Shot 会读取该文件，
并继续以原格式写入。

## 常用选项

### 主题与语言

```toml
[ui]
theme = "light"      # light | dark | system
language = ""        # 留空或 "system"、"english"、"chinese"
```

在编辑器中按 `Ctrl+Shift+L` 可以切换浅色和深色主题，并保存选择。

### 完成动作与覆盖层

```toml
[general]
to = "cs"            # c 复制，s 保存，p 贴图，o 标准输出，详见使用说明
overlay = "auto"     # auto | window | layer
```

`window` 以全屏窗口显示编辑器，`layer` 把编辑器绘制在所有内容上方，
`auto` 在 niri 和 COSMIC 上使用 `layer`，在其他桌面使用 `window`。

### 截图行为

```toml
[capture]
freeze_scope = "all-screens"     # all-screens | cursor-screen
hide_own_windows = true          # 截图时隐藏贴图等 Mark Shot 自身的窗口
kde_kwin_screenshot = true       # 设为 false 时，KDE 上改用桌面门户
double_click_action = "copy"     # none | copy | save | save-as | pin | cancel
middle_click_action = "none"
toolbar_position = "selection"   # selection | top | bottom | left | right

[startup]
include_cursor = false

[region_detection]
enabled = true

[window_detection]
enabled = true
```

### 保存位置

```toml
[save]
directory = "screenshots"            # 位于系统图片目录内
month_format = "%Y-%m"               # 按月份划分的子目录
filename_format = "%Y-%m-%d_%H-%M"
```

### 贴图

```toml
[pinned]
always_on_top = true
border_enabled = true
border_color = "#5EEAD4"
border_width = 2.0
```

### 截图历史

```toml
[history]
enabled = true
limit = 50               # 1 到 200
max_size_mib = 256
index_annotations = false
directory = ""           # 留空使用默认数据目录
```

`index_annotations` 把标注文字写入历史记录的 PNG 元数据。这些文字可能包含最终图片中已被裁掉或遮盖的内容，
因此默认关闭。

### 上传

上传按钮以 multipart POST 请求发送 PNG。程序没有默认图床。

```toml
[upload]
command = ""             # 留空使用内置的 HTTP 上传
timeout_ms = 60000

[upload.env]
MARK_SHOT_UPLOAD_URL = "https://image-host.example/upload"
MARK_SHOT_UPLOAD_FIELD = "image"
MARK_SHOT_UPLOAD_API_KEY = "token"
MARK_SHOT_UPLOAD_AUTH_HEADER = "Authorization"
MARK_SHOT_UPLOAD_AUTH_SCHEME = "Bearer"
MARK_SHOT_UPLOAD_URL_PATH = "data.url"
```

- `MARK_SHOT_UPLOAD_URL_PATH` 是图片地址在 JSON 响应中的路径，各级以点号分隔。
  未设置时依次尝试 `url`、`data.url`、`data.link`、`link` 和 `image.url`。
- 以 `MARK_SHOT_UPLOAD_HEADER_` 开头的字段用于添加请求头，以 `MARK_SHOT_UPLOAD_FIELD_`
  开头的字段用于添加表单字段。
- 设置 `command` 后，Mark Shot 改为运行该命令。`{image}` 会替换为临时 PNG 的路径；
  命令中没有占位符时，路径追加到命令末尾。命令需要在输出的第一行给出地址，或输出带 `url` 字段的 JSON 对象。

这些变量也可以设置在进程环境中，不写入文件。

### 通知与日志

```toml
[notifications]
enabled = true
preview = true           # 在通知中显示截图

[log]
level = "info"           # off | error | info
max_file_size_mb = 1
```

## 全部配置分组

| 分组 | 用途 |
| --- | --- |
| `[ui]` | 主题与语言 |
| `[general]` | 默认完成动作、覆盖层类型 |
| `[capture]`、`[startup]` | 截图行为、工具栏位置 |
| `[region_detection]`、`[window_detection]` | 自动识别的区域和窗口候选 |
| `[save]`、`[export.image_frame]` | 保存位置、导出外框 |
| `[pinned]`、`[history]`、`[notifications]` | 贴图、历史、通知 |
| `[desktop]` | 托盘图标与全局快捷键，见[桌面集成](desktop-integration.md) |
| `[recording]` | 见[屏幕录制](recording.md) |
| `[ocr]`、`[translation]` | 见[文字识别与翻译](ocr-and-translation.md) |
| `[keys]` | 见[快捷键](shortcuts.md) |
| `[toolbar_layout]`、`[toolbar]` | 工具栏按钮、图标与字号 |
| `[tools]` 与 `[tools.*]` | 各工具的初始颜色、笔宽和样式 |
| `[upload]`、`[log]` | 上传、日志 |
| `[theme]`、`[selection]`、`[annotations.*]`、`[input]`、`[animation]` 等 | 外观和交互的高级参数，通常不需要修改 |

`[tools]` 只决定工具的初始值。工具使用过之后，状态目录中记录的上次取值优先。

## 环境变量

| 变量 | 作用 |
| --- | --- |
| `MARK_SHOT_LANG` | 界面语言，优先于 `[ui].language` |
| `MARK_SHOT_BACKEND` | `auto`、`wayland` 或 `x11`，在 Linux 上强制指定显示协议 |
| `MARK_SHOT_CAPTURE` | `kde`、`image-copy`、`screencopy` 或 `portal`，强制指定 Wayland 截图方式 |
| `MARK_SHOT_LOG` | 日志级别：`error`、`warn`、`info`、`debug` 或 `trace` |
| `MARK_SHOT_CONFIG_DIR` | 指定一个目录，使用其中 Qt 格式的 `config.json` |
| `MARK_SHOT_MEDIA_LIBRARY` | 录制媒体库的绝对路径，用于开发构建 |
| `MARK_SHOT_UPLOAD_*` | 上传设置，见上文 |

强制指定的后端无法连接时，程序报告错误，不会自行改用另一种显示服务。

## 数据位置

| 数据 | Linux 路径 |
| --- | --- |
| 配置 | `~/.config/mark-shot/` |
| 工具的上次取值、日志文件 | `~/.local/state/mark-shot/` |
| 文字识别模型 | `~/.local/share/mark-shot/models/` |
| 文字识别推理库 | `~/.local/share/mark-shot/runtime/` |
| 截图历史 | `~/.local/share/mark-shot/capture-history/` |
| 截图 | `~/Pictures/screenshots/` |
| 录制文件 | 系统视频目录中的 `Mark Shot` 文件夹 |

Linux 路径遵循 XDG 基础目录变量。在 Windows 上，配置、文字识别模型和推理库保存在
`%APPDATA%\mark-shot\` 下。
