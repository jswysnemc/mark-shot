# 配置参考

## 应用配置文件

Mark Shot 在 Linux 上从 `~/.config/mark-shot/config.json` 读取应用配置，在其他平台上使用 Qt 应用配置目录。贴图窗口同样使用该文件中的 OCR 与翻译配置。默认 OCR helper 会优先使用 `rapidocr`，也可以回退到 `tesseract`。翻译 helper 使用 OpenAI 兼容的 `/chat/completions` 接口。

<details>
<summary>应用配置 JSON 示例与配置项详细说明 (点击展开)</summary>

```json
{
  "env": {
    "QT_FONT_DPI": 96
  },
  "ui": {
    "language": "system",
    "theme": "system"
  },
  "debug": {
    "enabled": false,
    "logPath": ""
  },
  "annotation": {
    "defaultTool": "move",
    "fullscreenDefaultTool": "laser",
    "defaultColor": "#FF4D4D"
  },
  "save": {
    "pathTemplate": "{pictures}/mark-shot/mark-shot-{datetime}.png"
  },
  "export": {
    "imageFrame": {
      "enabled": false,
      "padding": 112,
      "cornerRadius": 18,
      "shadowRadius": 72,
      "shadowOffsetY": 28,
      "shadowOpacity": 0.32
    }
  },
  "capture": {
    "wayland": {
      "kde": {
        "kwinScreenshot": {
          "enabled": true
        }
      }
    }
  },
  "shortcuts": {
    "tools": {
      "pen": "P",
      "rectangle": "R"
    },
    "actions": {
      "copy": "Ctrl+C",
      "save": "Ctrl+S",
      "pin": "Ctrl+P",
      "upload": "Ctrl+U"
    },
    "startup": {
      "colorPicker": "C",
      "ruler": "R",
      "codeScanner": "Q",
      "displayCapture": "D"
    }
  },
  "windows": {
    "tray": {
      "enabled": true
    },
    "hotkeys": {
      "capture": "Ctrl+Alt+S"
    }
  },
  "codeScan": {
    "command": "",
    "timeoutMs": 15000
  },
  "upload": {
    "command": "",
    "timeoutMs": 60000,
    "env": {
      "MARK_SHOT_UPLOAD_URL": "",
      "MARK_SHOT_UPLOAD_FIELD": "image",
      "MARK_SHOT_UPLOAD_API_KEY": "",
      "MARK_SHOT_UPLOAD_AUTH_SCHEME": "Bearer",
      "MARK_SHOT_UPLOAD_URL_PATH": "",
      "MARK_SHOT_UPLOAD_DELETE_URL_PATH": ""
    }
  },
  "pinnedWindow": {
    "autoOcr": false,
    "alwaysOnTop": true,
    "border": true,
    "borderColor": "#5EEAD4",
    "borderWidth": 2
  },
  "scrollCapture": {
    "frame": 5,
    "previewGap": 5,
    "hidePreviewDuringCapture": false
  },
  "windowDetection": {
    "enabled": true,
    "command": "mark-shot-window-detection-niri",
    "env": {
      "MARK_SHOT_NIRI_PANEL_EDGE": "top",
      "MARK_SHOT_NIRI_OFFSET_Y": 0
    },
    "timeoutMs": 1000
  },
  "ocr": {
    "enabled": true,
    "backend": "rapidocr",
    "command": "",
    "timeoutMs": 30000
  },
  "translation": {
    "autoAfterOcr": false,
    "targetLanguage": "Simplified Chinese",
    "apiBase": "https://api.openai.com/v1",
    "apiKeyEnv": "OPENAI_API_KEY",
    "apiKey": "",
    "model": "gpt-4o-mini",
    "temperature": 0.2,
    "timeoutMs": 60000,
    "timeoutSeconds": 60,
    "systemPrompt": "",
    "command": ""
  }
}
```

| 配置项键名 | 数据类型 | 默认值 | 功能描述 |
| :--- | :---: | :---: | :--- |
| `env` | 对象 | `{}` | 在创建 `QApplication` 之前应用到进程的环境变量（例如设置 `"QT_FONT_DPI": 96` 来规避高 DPI 缩放带来的截图边界偏移）。别名：`environment`。 |
| `ui.language` | 字符串 | `"system"` | 界面语言。支持 `system`（跟随系统语言）、`english`、`chinese`，也接受 `en`/`zh`/`zh_cn`/`cn` 等变体。取代旧版最外层 `language` 字段。可在设置通用页配置。 |
| `ui.theme` | 字符串 | `"system"` | 界面主题。支持 `system`（跟随 Qt 或桌面色彩方案）、`dark` 和 `light`。可在设置通用页配置，设置窗口会立即应用该主题。取代旧版最外层 `theme` 字段。 |
| `capture.freezeScope` | 字符串 | `"all-screens"` | 普通区域截图模式下的显示器冻结范围。当为多显示器环境且没有显式指定捕获全部输出时生效。支持的值包括：`all-screens`（冻结所有显示器）、`cursor-screen`（仅冻结鼠标指针当前所在的显示器）。别名：`freezeScope`、`freezeDisplayScope` 等。 |
| `capture.wayland.kde.kwinScreenshot.enabled` | 布尔值 | `true` | 是否在 KDE Wayland 环境下启用 KWin 的 `org.kde.KWin.ScreenShot2` 限制级别 DBus 接口截屏功能。如果关闭，将自动回退到常规 Portal 截屏。 |
| `debug.enabled` | 布尔值 | `false` | 在 Linux 和 Windows 上启用调试日志。命令行参数 `--debug` / `--no-debug` 会覆盖此项；除非设置 `--no-debug`，否则 `DEBUG=1` 仍会启用日志。 |
| `debug.logPath` | 字符串 | 系统临时目录 `mark-shot-scroll.log` | 调试日志输出路径。命令行参数 `--debug-log` 会覆盖此项；未设置配置或命令行路径时，`MARK_SHOT_DEBUG_LOG` 仍然有效。 |
| `annotation.defaultTool` | 字符串 | `"move"` | 选区完成后默认激活的标注工具。支持的值包括：`move`、`select`、`pen`、`line`、`highlighter`、`rectangle`、`ellipse`、`arrow`、`text`、`number`、`mosaic`、`magnifier`、`laser`。命令行参数 `--default-tool` 会覆盖此项。 |
| `annotation.fullscreenDefaultTool` | 字符串 | `"laser"` | 全屏标注模式下默认激活的工具。命令行参数 `--fullscreen-default-tool` 会覆盖此项。若在全屏模式下配置为 `move`，系统会自动降级为使用 `select`。 |
| `annotation.defaultColor` | 字符串 | `"#FF4D4D"` | 初始标注颜色。支持不透明的十六进制格式 `#RRGGBB` 或包含透明度的 `#RRGGBBAA`。命令行参数 `--default-color` 会覆盖此项。 |
| `save.pathTemplate` | 字符串 | `"{pictures}/mark-shot/mark-shot-{datetime}.png"` | 保存截图文件的路径模板（包括保存动作和另存为的初始文件名）。父级目录在保存时若不存在会自动创建。别名包括：`save.path`、`save.location`、最外层的 `savePathTemplate` 以及 `save.directory`（目录模板）。 |
| `save.directoryTemplate` | 字符串 | `""` | 仅指定保存目录模板。指定后，文件名会自动采用默认的 `mark-shot-{datetime}.png`。别名包括：`save.directory`、`save.dir`、`save.folder`。 |
| `recording.storage.videoDirectory` | 字符串 | `"{pictures}/mark-shot/videos"` | MP4 录制文件默认输出目录。别名包括 `recording.storage.videos`、`recording.storage.videoDir` 和 `recording.output.videoDirectory`。 |
| `recording.storage.gifDirectory` | 字符串 | `"{pictures}/mark-shot/gifs"` | GIF 录制文件默认输出目录。别名包括 `recording.storage.gifs`、`recording.storage.gifDir` 和 `recording.output.gifDirectory`。 |
| `export.imageFrame` | 布尔值/对象 | `false` | 用户分享类导出的可选 Mac 风格外框。对象形式支持 `enabled`、`padding` (`0`-`256`，默认 `112`)、`cornerRadius` (`0`-`128`，默认 `18`)、`shadowRadius` (`0`-`128`，默认 `72`)、`shadowOffsetY` (`0`-`128`，默认 `28`) 和 `shadowOpacity` (`0.0`-`1.0`，默认 `0.32`)。作用于保存、另存为、复制、上传、打开方式和扩展命令图片；OCR、扫码、贴图、快速显示器截图和滚动截图保持原始图片。设置 `enabled` 为 `true` 后启用外框导出。 |
| `shortcuts` | 对象 | - | 自定义快捷键配置。别名：`hotkeys`（或在 `annotation.shortcuts` / `annotation.hotkeys` 下）。详细子节点见折叠说明。 |
| `windows.tray.enabled` | 布尔值 | Windows 为 `true`，其他平台为 `false` | 自动启动托盘模式。键名出于兼容性保留。可以用 `mark-shot --tray` 在不修改配置时启动托盘模式，也可以用 `mark-shot --capture` 在自动启动托盘时强制执行单次截图。 |
| `windows.hotkeys.capture` | 字符串 | `"Ctrl+Alt+S"` | 托盘模式运行时触发区域截图的全局快捷键。Windows 使用 RegisterHotKey，支持的 Linux 桌面使用 desktop portal。别名包括 `hotkey`、`captureHotkey` 和 `screenshot` |
| `windows.hotkeys.fullscreen` | 字符串 | `""` | 可选的全屏标注截图全局快捷键。别名：`fullscreenHotkey`。默认生成配置只写入区域截图快捷键。 |
| `colorPicker.history` | 数组 | `[]` | 启动取色器最近拾取的颜色记录。以 `#RRGGBBAA` 字符串存储，最多保留 7 条。在颜色面板确认颜色时会自动更新。 |
| `codeScan.command` | 字符串 | `""` | 自定义二维码/条形码扫码命令。支持 `{image}`、`{imagePath}` 和 `{imageUrl}` 占位符；如果没有占位符，Mark Shot 会把临时 PNG 路径追加到命令末尾。命令必须输出与 `mark-shot-code-scan` 相同结构的 JSON。别名：`codeScanner.command`、`barcodeScanner.command`、`barcode.command`。 |
| `codeScan.timeoutMs` | 数值 | `15000` | 扫码命令超时时间。环境变量 `MARK_SHOT_CODE_SCAN_TIMEOUT_MS` 可以覆盖该值。 |
| `upload.command` | 字符串 | `""` | 自定义图床上传命令。支持 `{image}`、`{imagePath}` 和 `{imageUrl}` 占位符；如果没有占位符，Mark Shot 会把临时 PNG 路径追加到命令末尾。命令必须输出 JSON `{"url":"...","deleteUrl":"...","errors":[]}` 或纯文本 URL（以 `http://`/`https://` 开头）。留空时使用内置 `mark-shot-upload` 脚本，通过 `upload.env` 配置图床参数。别名：`imageUpload.command`、`uploader.command`、`imageHost.command`。 |
| `upload.timeoutMs` | 数值 | `60000` | 上传命令超时时间。环境变量 `MARK_SHOT_UPLOAD_TIMEOUT_MS` 可以覆盖该值。 |
| `upload.env` | 对象 | `{}` | 传递给上传命令的环境变量。会合并到系统环境变量之上。用于配置内置 `mark-shot-upload` 脚本的图床参数（端点、字段、API Key、认证方案、URL 提取路径等）。别名：`environment`、`envVars`、`variables`。 |
| `pinnedWindow.autoOcr` | 布尔值 | `false` | 控制贴图窗口创建后是否立即在后台自动启动 OCR 文本识别。如果禁用，则仅在右键菜单中触发复制文字或翻译时按需识别。别名：`pinned`、`pin`。 |
| `pinnedWindow.border` | 布尔值/对象 | `true` | 贴图窗口外边框的配置。可以为布尔值，或者包含 `enabled` (布尔值)、`color` (十六进制/名称/RGBA对象) 和 `width` (浮点数，`1.0` - `12.0`) 的配置对象。也支持 `borderEnabled`、`borderColor`、`borderWidth` 平铺配置。 |
| `scrollCapture.frame` | 布尔值/数值/对象 | `5` | 滚动截图外框偏移。数值表示实际捕获区域和外框之间的像素间距；`false` 关闭外框。对象形式支持 `enabled` 和 `gap`。别名：`captureFrame`、`border`、`outline`，也支持平铺的 `frameEnabled` / `frameGap`。 |
| `scrollCapture.previewGap` | 数值/对象 | `5` | 外框和滚动预览面板之间的像素间距。预览面板会在外框周围选择第一个可用的不重叠位置。别名：`previewDistance`、`previewOffset`、`panelGap`；对象形式支持 `gap`。 |
| `scrollCapture.hidePreviewDuringCapture` | 布尔值 | `false` | 控制滚动截图运行时是否强制隐藏预览面板。开启后，即使小选区周围有空间，也会隐藏预览面板并显示悬浮拖拽手柄；暂停时仍会显示预览面板。别名：`hidePreviewWhileCapturing`、`hidePanelDuringCapture`、`hideUiDuringCapture`；也支持 `scrollCapture.preview.hideWhileCapturing`。 |
| `ocr.enabled` | 布尔值 | `true` | 控制 OCR 功能是否全局可用。本身不控制贴图后台 OCR 的自动触发。 |
| `ocr.resultPanel` | 布尔值/对象 | `true` | 控制主截图 OCR 流程是否打开可编辑结果窗口。对象形式支持 `enabled`、`show`、`visible` 或 `use`。别名包括 `resultWindow`、`ocrResultPanel`、`ocrResultWindow`。环境变量 `MARK_SHOT_OCR_RESULT_PANEL` 和 `MARK_SHOT_OCR_RESULT_WINDOW` 会覆盖配置文件。 |
| `translation.autoAfterOcr` | 布尔值 | `false` | 控制贴图窗口 OCR 成功后是否自动启动翻译并缓存翻译结果。开启后，用户在右键菜单选择翻译时会瞬间渲染已缓存的翻译，无需临时发起网络请求。别名：`translation.auto` / `autoAfterOCR` 等。 |
| `windowDetection.enabled` | 布尔值 | `true` | 控制窗口边界识别。设置为 `false` 后会同时关闭内置 X11 窗口识别和已配置的外挂检测脚本。 |
| `windowDetection.env` | 对象 | `{}` | 传给窗口边界检测脚本的环境变量。别名：`environment`。<br>• **Niri 适配脚本**：自动读取 DMS bar、dock、frame 和 frame-exclusion 设置，也支持 `MARK_SHOT_NIRI_PANEL_EDGE`（`top`/`bottom`/`left`/`right`/`none`）以及像素偏移 `MARK_SHOT_NIRI_OFFSET_X/Y/WIDTH/HEIGHT`。<br>• **Hyprland 适配脚本**：支持 `MARK_SHOT_HYPRLAND_INCLUDE_INACTIVE`（`1`/`0`）以及像素偏移 `MARK_SHOT_HYPRLAND_OFFSET_X/Y/WIDTH/HEIGHT`。 |

##### 保存路径占位符说明
在 `save.pathTemplate` 或 `save.directoryTemplate` 中，支持使用以下动态占位符（这些占位符会被自动替换为当前截图会话的对应数值或路径）：
- **物理路径**：`{home}`（用户主目录）、`{pictures}`（图片目录）、`{desktop}`（桌面目录）、`{downloads}`（下载目录）、`{config}`（配置目录）、`{data}`（数据目录）。
- **时间与日期**：`{timestamp}`（秒级时间戳）、`{timestamp.ms}`（毫秒级时间戳）、`{yyyy}`（4位年份）、`{yy}`（2位年份）、`{MM}`（月份，补零）、`{M}`（月份）、`{dd}`（日期，补零）、`{d}`（日期）、`{HH}`（24小时制小时）、`{hh}`（12小时制小时）、`{mm}`（分钟）、`{ss}`（秒）、`{zzz}`（毫秒）、`{date}`（`yyyyMMdd` 简写）、`{time}`（`HHmmss` 简写）、`{datetime}`（`yyyyMMdd-HHmmss` 简写）。
- **自定义日期时间格式**：使用 `{datetime:FORMAT}`，例如 `{datetime:yyyy-MM-dd_HH-mm-ss}`，格式符合 Qt 的 `QDateTime::toString()` 规范。
- **选区与几何**：`{selection.x}`, `{selection.y}`, `{selection.width}`, `{selection.height}`, `{selection.right}`, `{selection.bottom}`, `{selection.geometry}`（选区位置与大小）；以及对应的 `{source.*}` 字段（表示捕获源的完整屏幕几何）。
- **图片与输出信息**：`{image.width}`, `{image.height}`（最终渲染图片物理宽高）、`{name}`（输出名，默认 `capture`）、`{ext}`（文件后缀名，如 `png`）。

*注：相对路径会被解析至默认的 `Pictures/mark-shot` 目录下，若路径模板未以 `.png` 结尾，程序会自动追加。*

##### 快捷键配置项子节点说明

`shortcuts` 节点支持以下子项配置：
- **`tools`**（别名：`tool`、`toolShortcuts`）：工具切换快捷键，对应 `defaultTool` 支持的各种工具。
- **`actions`**（别名：`action`、`actionShortcuts`）：全局动作快捷键，支持 `copy`、`save`、`pin`、`upload`、`undo`、`redo`、`cancel`、`openWith`、`extensions`、`scrollCapture`、`ocrCopy`、`clear`、`toggleCaptureScope`、`toggleToolbarLayout`。
- **`startup`**（别名：`startupTools`、`selection`）：选区前辅助工具的快捷键，支持 `colorPicker`（取色器）、`ruler`（测量尺）、`codeScanner`（扫码）与 `displayCapture`（显示器截取）。

*快捷键格式采用 Qt 按键序列文本，例如 `Ctrl+C`、`Ctrl+Shift+Z` 或 `Alt+R`。也可以直接定义在 `shortcuts` 顶层。*

</details>

### 工具默认值持久化

Mark Shot 会记住最近一次使用的标注工具默认值，并在下次启动时恢复，使得工具栏从首次绘制起就反映上次会话的样式。

状态文件独立保存在 `~/.config/mark-shot/annotation-state.json`（Linux 平台；其他平台使用 Qt 应用配置目录），与 `config.json` 完全分离。该文件只用于保存临时的工具默认值，可随时删除以重置编辑器到内置默认值。

被持久化的字段包括：

- 当前绘制颜色与不透明度，以及文本背景色。
- 各工具的笔宽：画笔、形状、数字步骤、马赛克块大小、激光笔。
- 矩形的填充开关、圆角半径以及风格（`Stroke` / `Highlight` / `Invert`）。
- 放大镜倍率以及透镜形状（`Circle` / `Rectangle`）。
- 箭头风格、荧光笔风格、数字步骤风格。
- 文本字体。

写盘通过 `QSaveFile` 原子提交完成，并在每个修改默认值的入口（滑块释放、取色器确认、风格切换、字体选择、调色板拾取等）触发后立即写入，因此进程崩溃也不会留下半截写入的文件。快捷键、保存路径模板、OCR、翻译、图床上传等应用级配置仍然保留在 `config.json` 中。

### 截图前窗口检测与脚本贡献指南

为了使 Mark Shot 在各种 Wayland 合成器中都能实现精准的窗口边界识别，项目采用了一种灵活的外部脚本调用机制：用户可通过 `windowDetection.command` 配置检测脚本，由脚本负责调用合成器的特定命令提取窗口位置，最终将数据转换为统一格式回传给 Mark Shot 消费。

Mark Shot 还会在运行时探测当前桌面环境（`XDG_SESSION_TYPE`、`XDG_CURRENT_DESKTOP` 等），自动选择匹配的检测脚本。支持 GNOME、KDE Plasma、Hyprland、Niri；其他 Wayland 会话回退到 niri 脚本，X11 会话使用内置原生 X11 检测（空命令）。若已配置的 `windowDetection.command` 与当前环境不匹配，Mark Shot 会在内存中纠正而不修改 `config.json`，因此手动配置并非必需。

项目已内置了针对以下窗口管理器的边界检测脚本：
- **Niri**：`mark-shot-window-detection-niri`
- **Hyprland**：`mark-shot-window-detection-hyprland`
- **GNOME Shell**：`mark-shot-window-detection-gnome`
- **KDE Plasma / KWin**：`mark-shot-window-detection-kde`

<details>
<summary><b>展开/折叠 窗口检测脚本配置与贡献指南</b></summary>

#### 如何贡献适配脚本
目前，仓库已内置适用于 niri、Hyprland、GNOME (Mutter Wayland) 以及 KDE Plasma (KWin Wayland) 窗口管理器的适配脚本。

我们强烈欢迎并鼓励社区用户为其他桌面环境与 Wayland 合成器贡献适配脚本，以扩充软件的兼容性。如果您在 Sway 或其他 Wayland 合成器等环境中使用，欢迎编写适配脚本并向本项目提交 Pull Request。以下是不同环境的实现思路提示：
- **Hyprland**：可以通过调用 `hyprctl clients -j` 解析生成的 JSON 信息。
- **Sway**：可以使用 `swaymsg -t get_tree` 获取完整的树形窗口布局。
- **KDE / KWin**：可以通过调用 KWin Script 获取窗口对象，或者查询相应的 D-Bus 接口获取。
- **GNOME**：由于 GNOME Wayland 没有内置导出窗口位置的 CLI 命令，通常需借助 GNOME Shell 扩展读取窗口逻辑边界，再通过 D-Bus 对外暴露接口。

若脚本执行失败或超时（默认为 `1000ms`），Mark Shot 将继续截图流程，并在 X11 模式下自动回退至内置的窗口检测器。

#### 如何使用与配置：
1. 将项目仓库 `scripts/` 目录下的相应脚本复制到系统的 `$PATH` 路径目录下（例如 `~/.local/bin/` 或 `/usr/local/bin/`）。
2. 为该脚本赋予可执行权限：
   ```bash
   chmod +x ~/.local/bin/mark-shot-window-detection-niri
   # 或
   chmod +x ~/.local/bin/mark-shot-window-detection-hyprland
   # 或
   chmod +x ~/.local/bin/mark-shot-window-detection-kde
   ```
3. 在您的 `~/.config/mark-shot/config.json` 配置文件中，在 `windowDetection.command` 字段中指定该脚本的名称（如果在 `$PATH` 中）或其绝对路径：
   ```json
   "windowDetection": {
     "command": "mark-shot-window-detection-niri"
   }
   ```

   KDE Plasma / KWin Wayland 可使用以下配置：
   ```json
   "windowDetection": {
     "command": "mark-shot-window-detection-kde",
     "timeoutMs": 2000
   }
   ```

   KDE 脚本需要系统提供 `qdbus6` 命令，Arch Linux 可通过 `qt6-tools` 包安装。

#### 1. 脚本执行时的输入（环境变量）

脚本被调用时，Mark Shot 会传入以下环境变量以提供当前截图的上下文信息：

| 环境变量名称 | 数据类型 | 描述 |
| :--- | :--- | :--- |
| `MARK_SHOT_CONFIG` | 字符串 | 配置文件路径（通常为 `~/.config/mark-shot/config.json`） |
| `MARK_SHOT_CAPTURE_OUTPUT` | 字符串 | 当前要捕获的输出显示器名称（如 `eDP-1`、`DP-2`） |
| `MARK_SHOT_CAPTURE_ALL_OUTPUTS` | 整数 | `1` 表示捕获全部屏幕；`0` 表示仅捕获当前活动屏幕 |
| `MARK_SHOT_CAPTURE_X` | 整数 | 待捕获区域在全局逻辑合成器中的 X 坐标起始点 |
| `MARK_SHOT_CAPTURE_Y` | 整数 | 待捕获区域在全局逻辑合成器中的 Y 坐标起始点 |
| `MARK_SHOT_CAPTURE_WIDTH` | 整数 | 待捕获区域的逻辑像素宽度 |
| `MARK_SHOT_CAPTURE_HEIGHT` | 整数 | 待捕获区域的逻辑像素高度 |

> [!NOTE]
> 脚本返回的所有窗口坐标值必须使用**全局逻辑合成器坐标系**，该坐标系需要与 Qt 获取到的屏幕几何保持一致，从而避免在多屏缩放时发生位置偏移。

#### 2. 约定的输出 JSON 数据格式

脚本必须将检测到的窗口信息以 JSON 格式输出至标准输出（`stdout`）。Mark Shot 支持多种兼容的宽容解析格式：

##### 根节点格式
根节点可以是一个包含 `windows` 或 `windowGeometries` 数组的对象，也可以直接是包含窗口几何信息的数组。例如：
- 方式 A（对象包裹）：`{ "windows": [ ... ] }` 或 `{ "windowGeometries": [ ... ] }`
- 方式 B（对象本身即为窗口）：`{ "x": 100, "y": 100, "w": 400, "h": 300 }`
- 方式 C（纯数组）：`[ ... ]`

##### 窗口几何数据结构
数组内的每一个元素（或根对象）可以表示为以下四种形式之一：

- **键值对对象格式**：
  直接提供位置和大小的整型字段。
  - 横坐标键名支持：`x` 或 `left`
  - 纵坐标键名支持：`y` 或 `top`
  - 宽度键名支持：`width` 或 `w`
  - 高度键名支持：`height` 或 `h`
  *示例*：`{ "x": 100, "y": 200, "w": 800, "h": 600 }`

- **子嵌套对象格式**：
  使用 `at`（表示起始点 `[x, y]`）与 `size`（表示尺寸 `[width, height]`）字段。
  *示例*：`{ "at": [100, 200], "size": [800, 600] }`

- **数组及内嵌数组格式**：
  直接包含 4 个整数的数组，代表 `[x, y, width, height]`。
  也可以是对象中以 `rect` 命名的小数组。
  *示例*：`[100, 200, 800, 600]` 或 `{ "rect": [100, 200, 800, 600] }`

- **几何字符串格式**：
  使用字符串描述几何，格式需符合 `x,y widthxheight`（允许包含负数与空格）。
  也可以是对象中以 `geometry` 命名的字符串字段。
  *示例*：`"100,200 800x600"` 或 `{ "geometry": "100,200 800x600" }`

##### 完整 JSON 示例参考

```json
{
  "windows": [
    {
      "x": 100,
      "y": 150,
      "width": 800,
      "height": 600
    },
    {
      "left": 950,
      "top": 150,
      "w": 400,
      "h": 300
    },
    {
      "at": [100, 800],
      "size": [800, 450]
    },
    {
      "rect": [950, 800, 400, 300]
    },
    {
      "geometry": "100,1300 800x600"
    },
    [950, 1300, 400, 300]
  ]
}
```

</details>

手动安装时，必须同时安装 `mark-shot`、`mark-shot-ocr`、`mark-shot-code-scan`、`mark-shot-translate` 和 `mark-shot-upload`。否则 OCR、扫码、翻译或图床上传功能无法调用后端脚本。

---
