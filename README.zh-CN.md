# Mark Shot

[English README](README.md)

<details>
<summary>演示视频</summary>
<p align="center">
  <video src="https://github.com/user-attachments/assets/c2298867-06b4-404d-87bc-62ab8d81088b" width="100%" controls></video>
</p>
</details>

`mark-shot` 是一款基于 Qt 6 开发的高性能截图标注工具。项目最初针对 `niri` 等 Wayland 窗口管理器设计，也支持在 X11/GNOME 桌面环境中完成常规截图与标注工作流。

它可以瞬间截取屏幕画面，并打开自适应全屏标注覆盖层，为用户提供区域裁切、标注、复制到剪贴板、保存以及桌面贴图等功能。

---

## 核心特色

### 标注工具箱
- **画笔与荧光笔**：支持平滑的自由线条绘制与半透明高亮叠色。
- **几何矢量工具**：高精度的直线、矩形与椭圆路径。
- **优化箭头**：采用六顶点经典箭头路径，边缘平滑且支持抗锯齿渲染。
- **双重联动文本**：
  - 支持超大字号的无级调节，可通过鼠标滚轮或属性滑块平滑缩放。
  - 引入物理宽度缓冲区设计，避免文本在极高缩放比例下由于渲染抖动产生意外折行。
  - **对角控制点**可实现字号与文本框的等比例联动缩放；**左右边控制线**则仅调节排版边界宽度。
- **激光演示笔**：适用于展示或教学，笔迹会随时间平滑融解消失。
- **自增步骤序号**：点击即可放置依次递增的数字步骤标记。
- **马赛克**：支持对敏感信息执行毛玻璃区域模糊虚化。

### 贴图悬浮固定（Pin）
- 支持将截图或标注区域作为一个独立、无边框且置顶的悬浮贴图窗口固定在屏幕上。
- 支持在贴图窗口中直接选择 OCR 识别出的文字，使用 `Ctrl + C` 或右键菜单复制图片文字。
- 支持通过 OpenAI 兼容接口调用 LLM 翻译 OCR 文本，并将译文按原图位置覆盖渲染到贴图上。
- **便捷交互**：
  - 鼠标左键拖动可自由平移贴图位置。
  - 滚动鼠标滚轮可等比例缩放贴图，双击 `Ctrl` 键复位大小。
  - 双击鼠标左键或按下 `Esc` 键即可关闭贴图。
  - 右键单击唤出菜单，支持多角度旋转、透明度微调（0.2 - 1.0）、复制图片文字、翻译、另存为、复制或关闭。

### 滚动截图
- 通过 PipeWire screencast、交互式滚动覆盖层和图像拼接器，捕获长页面或长区域截图。
- 该功能主要面向 `niri` 以及行为相近的 Wayland 环境；这些环境的输出几何、捕获时序和窗口位置更容易保持稳定。
- **兼容性说明**：KDE、GNOME、X11 以及其他非 `niri` 环境中的滚动截图仍是测试特性，尚不完善。这些桌面栈的 portal 后端策略、Shell 或窗口管理器行为、窗口几何反馈、帧时序和滚动事件处理存在差异。
- 如果滚动截图无法使用，请使用普通截图流程，或者通过 Mark Shot 拓展命令接入外部长截图工具。
- 如果需要提交滚动截图问题，请先运行 `DEBUG=1 mark-shot` 并复现问题，然后把 `/tmp/mark-shot-scroll.log` 附到 GitHub issue 中。需要自定义日志路径时，可设置 `MARK_SHOT_DEBUG_LOG=/path/to/log`。

### 跨显示服务器支持
- **Wayland**：使用 PipeWire portal screencast 支持实验性滚动截图，使用 `grim` 支持 wlroots 截屏，使用 `layer-shell-qt` 创建原生覆盖层，使用 `wl-copy` 持久化剪贴板。
- **X11**：使用 `QScreen::grabWindow` 截屏、全屏置顶窗口作为覆盖层、`xclip` 持久化剪贴板。
- 运行时通过 `$XDG_SESSION_TYPE` 自动检测，无需手动配置。

### 桌面集成
- **桌面快捷方式**：
  - `mark-shot.desktop`：配置为系统全局截图工具，支持系统快捷键直接调用。
  - `mark-shot-edit.desktop`：注册为独立的图像编辑器，可集成到文件管理器（如 Dolphin、Nautilus）的右键"打开方式"菜单中。
- 附带高分辨率的 `mark-shot.svg` 与 `mark-shot-edit.svg` 系统矢量图标。

---

## 命令行接口 (CLI)

### 常用使用示例

```bash
# 捕获屏幕并进入区域裁剪与标注模式
mark-shot

# 在多显示器环境下捕获所有输出屏幕
mark-shot --all-outputs

# 跳过选区步骤，直接对捕获的完整屏幕截图进行标注
mark-shot --fullscreen

# 选区完成后默认使用移动工具，全屏标注默认使用激光笔，并设置红色默认颜色
mark-shot --default-tool move --fullscreen-default-tool laser --default-color '#FF4D4D'

# 打开一个已有的本地图片文件并直接进入标注模式
mark-shot path/to/image.png

# 强制使用标准的 XDG 全屏普通窗口运行（而非 Wayland layer-shell）
mark-shot --xdg-window
```

### CLI 参数说明

| 参数选项 | 功能描述 |
| :--- | :--- |
| `[file]` | **位置参数**：打开一个已有的本地图片文件进入标注模式，而不是捕获当前屏幕。 |
| `-h`, `--help` | 显示帮助信息并退出。 |
| `-v`, `--version` | 显示当前版本信息并退出。 |
| `--all-outputs` | 捕获虚拟显示桌面的所有输出屏幕，而不是仅捕获当前的活动屏幕。 |
| `--xdg-window` | 强制使用标准的 XDG 全屏普通窗口（xdg-shell）替代默认的 Wayland 覆盖层（layer-shell）。 |
| `--fullscreen` | 跳过选区步骤，直接对捕获的完整屏幕截图进行标注。 |
| `--default-tool <tool>` | 指定普通选区完成后的默认标注工具；未设置 `--fullscreen-default-tool` 时也作为全屏模式默认工具。 |
| `--fullscreen-default-tool <tool>` | 指定全屏标注模式的默认工具。 |
| `--default-color <color>` | 指定默认标注颜色。支持 `#RRGGBB` 与 `#RRGGBBAA`。 |

### 快捷键绑定

将 `mark-shot` 绑定为系统截图快捷键：

**niri**（修改 `~/.config/niri/config.kdl`）：
```kdl
binds {
    Mod+Shift+S { spawn "mark-shot"; }
}
```

**GNOME**：在系统设置 → 键盘 → 自定义快捷键中添加。

### 拓展命令

右侧动作工具栏提供 **Extensions** 按钮，程序会从 `~/.config/mark-shot/extensions.json` 读取用户自定义命令。配置文件可以是 JSON 数组，也可以是包含 `commands` 数组的 JSON 对象。

```json
{
  "commands": [
    {
      "name": "Long screenshot",
      "command": "./target/release/wayscrollshot {slurp}",
      "workingDirectory": "~/Desktop/projects/wayscrollshot",
      "closeOnStart": true
    },
    {
      "name": "OCR selection",
      "command": "ocr-tool {image}",
      "saveImage": true
    }
  ]
}
```

`command` 会通过 `$SHELL -c` 执行，因此支持 shell 表达式。使用 `{slurp}` 可把当前选区作为 `x,y widthxheight` 几何字符串传入命令。使用 `{image}` 或 `{imagePath}` 可把当前已渲染选区作为临时 PNG 路径传入命令，使用 `{imageUrl}` 可传入 `file://` URL。这些占位符会自动进行 shell 引用转义，配置中不要再额外加引号。若未使用图片占位符，可设置 `saveImage` 或 `needsImage` 为 `true`，程序会自动把临时 PNG 路径追加到命令末尾。`workingDirectory` 与 `cwd` 等价。`closeOnStart` 默认值为 `true`，命令启动前会先隐藏并关闭 Mark Shot。

### 应用配置文件

Mark Shot 会从 `~/.config/mark-shot/config.json` 读取应用配置。贴图窗口同样使用该文件中的 OCR 与翻译配置。默认 OCR helper 会优先使用 `rapidocr`，也可以回退到 `tesseract`。翻译 helper 使用 OpenAI 兼容的 `/chat/completions` 接口。

```json
{
  "env": {
    "QT_FONT_DPI": 96
  },
  "annotation": {
    "defaultTool": "move",
    "fullscreenDefaultTool": "laser",
    "defaultColor": "#FF4D4D"
  },
  "shortcuts": {
    "tools": {
      "pen": "P",
      "rectangle": "R"
    },
    "actions": {
      "copy": "Ctrl+C",
      "save": "Ctrl+S",
      "pin": "Ctrl+P"
    },
    "startup": {
      "colorPicker": "C",
      "ruler": "R"
    }
  },
  "pinnedWindow": {
    "border": false,
    "borderColor": "#2DD4BF",
    "borderWidth": 2
  },
  "scrollCapture": {
    "frame": 5,
    "previewGap": 5
  },
  "windowDetection": {
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
| `annotation.defaultTool` | 字符串 | `"move"` | 选区完成后默认激活的标注工具。支持的值包括：`move`、`select`、`pen`、`line`、`highlighter`、`rectangle`、`ellipse`、`arrow`、`text`、`number`、`mosaic`、`magnifier`、`laser`。命令行参数 `--default-tool` 会覆盖此项。 |
| `annotation.fullscreenDefaultTool` | 字符串 | `"laser"` | 全屏标注模式下默认激活的工具。命令行参数 `--fullscreen-default-tool` 会覆盖此项。若在全屏模式下配置为 `move`，系统会自动降级为使用 `select`。 |
| `annotation.defaultColor` | 字符串 | `"#FF4D4D"` | 初始标注颜色。支持不透明的十六进制格式 `#RRGGBB` 或包含透明度的 `#RRGGBBAA`。命令行参数 `--default-color` 会覆盖此项。 |
| `shortcuts` | 对象 | - | 自定义快捷键配置。别名：`hotkeys`（或在 `annotation.shortcuts` / `annotation.hotkeys` 下）。详细子节点见折叠说明。 |
| `pinnedWindow.autoOcr` | 布尔值 | `false` | 控制贴图窗口创建后是否立即在后台自动启动 OCR 文本识别。如果禁用，则仅在右键菜单中触发复制文字或翻译时按需识别。别名：`pinned`、`pin`。 |
| `pinnedWindow.border` | 布尔值/对象 | `false` | 贴图窗口外边框的配置。可以为布尔值，或者包含 `enabled` (布尔值)、`color` (十六进制/名称/RGBA对象) 和 `width` (浮点数，`1.0` - `12.0`) 的配置对象。也支持 `borderEnabled`、`borderColor`、`borderWidth` 平铺配置。 |
| `scrollCapture.frame` | 布尔值/数值/对象 | `5` | 滚动截图外框偏移。数值表示实际捕获区域和外框之间的像素间距；`false` 关闭外框。对象形式支持 `enabled` 和 `gap`。别名：`captureFrame`、`border`、`outline`，也支持平铺的 `frameEnabled` / `frameGap`。 |
| `scrollCapture.previewGap` | 数值/对象 | `5` | 外框和滚动预览面板之间的像素间距。预览面板会在外框周围选择第一个可用的不重叠位置。别名：`previewDistance`、`previewOffset`、`panelGap`；对象形式支持 `gap`。 |
| `ocr.enabled` | 布尔值 | `true` | 控制 OCR 功能是否全局可用。本身不控制贴图后台 OCR 的自动触发。 |
| `ocr.resultPanel` | 布尔值/对象 | `true` | 控制主截图 OCR 流程是否打开可编辑结果窗口。对象形式支持 `enabled`、`show`、`visible` 或 `use`。别名包括 `resultWindow`、`ocrResultPanel`、`ocrResultWindow`。环境变量 `MARK_SHOT_OCR_RESULT_PANEL` 和 `MARK_SHOT_OCR_RESULT_WINDOW` 会覆盖配置文件。 |
| `translation.autoAfterOcr` | 布尔值 | `false` | 控制贴图窗口 OCR 成功后是否自动启动翻译并缓存翻译结果。开启后，用户在右键菜单选择翻译时会瞬间渲染已缓存的翻译，无需临时发起网络请求。别名：`translation.auto` / `autoAfterOCR` 等。 |
| `windowDetection.env` | 对象 | `{}` | 传给窗口边界检测脚本的环境变量。别名：`environment`。<br>• **Niri 适配脚本**：支持 `MARK_SHOT_NIRI_PANEL_EDGE`（`top`/`bottom`/`left`/`right`/`none`）以及像素偏移 `MARK_SHOT_NIRI_OFFSET_X/Y/WIDTH/HEIGHT`。<br>• **Hyprland 适配脚本**：支持 `MARK_SHOT_HYPRLAND_INCLUDE_INACTIVE`（`1`/`0`）以及像素偏移 `MARK_SHOT_HYPRLAND_OFFSET_X/Y/WIDTH/HEIGHT`。 |

### OCR 结果面板开关

主截图窗口里的 OCR 流程默认打开可编辑 OCR 结果窗口。

如果要关闭结果窗口并直接复制识别文字，可写到 `~/.config/mark-shot/config.json`：

```json
{
  "ocr": {
    "resultPanel": false
  }
}
```

也可以在启动前用 shell 环境变量覆盖配置文件：

```bash
MARK_SHOT_OCR_RESULT_PANEL=0 mark-shot
```

可识别的开启值有 `1`、`true`、`yes`、`on`；关闭值有 `0`、`false`、`no`、`off`。环境变量优先级高于配置文件。

<details>
<summary>快捷键配置项子节点说明</summary>

`shortcuts` 节点支持以下子项配置：
- **`tools`**（别名：`tool`、`toolShortcuts`）：工具切换快捷键，对应 `defaultTool` 支持的各种工具。
- **`actions`**（别名：`action`、`actionShortcuts`）：全局动作快捷键，支持 `copy`、`save`、`pin`、`undo`、`redo`、`cancel`、`openWith`、`extensions`、`scrollCapture`、`ocrCopy`、`clear`、`toggleCaptureScope`、`toggleToolbarLayout`。
- **`startup`**（别名：`startupTools`、`selection`）：选区前辅助工具的快捷键，支持 `colorPicker`（取色器）与 `ruler`（测量尺）。

*快捷键格式采用 Qt 按键序列文本，例如 `Ctrl+C`、`Ctrl+Shift+Z` 或 `Alt+R`。也可以直接定义在 `shortcuts` 顶层。*
</details>

### 截图前窗口检测与脚本贡献指南

为了使 Mark Shot 在各种 Wayland 合成器中都能实现精准的窗口边界识别，项目采用了一种灵活的外部脚本调用机制：用户可通过 `windowDetection.command` 配置检测脚本，由脚本负责调用合成器的特定命令提取窗口位置，最终将数据转换为统一格式回传给 Mark Shot 消费。

项目已内置了针对以下窗口管理器的边界检测脚本：
- **Niri**：`mark-shot-window-detection-niri`
- **Hyprland**：`mark-shot-window-detection-hyprland`

##### 如何使用与配置：
1. 将项目仓库 `scripts/` 目录下的相应脚本复制到系统的 `$PATH` 路径目录下（例如 `~/.local/bin/` 或 `/usr/local/bin/`）。
2. 为该脚本赋予可执行权限：
   ```bash
   chmod +x ~/.local/bin/mark-shot-window-detection-niri
   # 或
   chmod +x ~/.local/bin/mark-shot-window-detection-hyprland
   ```
3. 在您的 `~/.config/mark-shot/config.json` 配置文件中，在 `windowDetection.command` 字段中指定该脚本的名称（如果在 `$PATH` 中）或其绝对路径：
   ```json
   "windowDetection": {
     "command": "mark-shot-window-detection-niri"
   }
   ```

我们强烈欢迎并鼓励社区用户为其他桌面环境与 Wayland 合成器贡献适配脚本，以扩充软件的兼容性。

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

#### 3. 如何贡献适配脚本

目前，仓库仅内置了适用于 niri 窗口管理器的适配器：`mark-shot-window-detection-niri`。

如果您在 Hyprland、Sway、KDE (KWin Wayland) 或 GNOME (Mutter Wayland) 等环境中使用，欢迎编写适配脚本并向本项目提交 Pull Request。以下是不同环境的实现思路提示：
- **Hyprland**：可以通过调用 `hyprctl clients -j` 解析生成的 JSON 信息。
- **Sway**：可以使用 `swaymsg -t get_tree` 获取完整的树形窗口布局。
- **KDE / KWin**：可以通过调用 KWin Script 获取窗口对象，或者查询相应的 D-Bus 接口获取。
- **GNOME**：由于 GNOME Wayland 没有内置导出窗口位置的 CLI 命令，通常需借助 GNOME Shell 扩展读取窗口逻辑边界，再通过 D-Bus 对外暴露接口。

若脚本执行失败或超时（默认为 `1000ms`），Mark Shot 将继续截图流程，并在 X11 模式下自动回退至内置的窗口检测器。

手动安装时，必须同时安装 `mark-shot`、`mark-shot-ocr` 和 `mark-shot-translate`。否则贴图窗口可以打开，但复制图片文字与翻译功能无法调用后端脚本。

---

## 编译与安装

### 正式版产物

每次正式发布会提供 Linux 二进制压缩包、Debian 安装包与 Fedora RPM 安装包：

- `linux-x86_64.tar.gz` 与 `linux-arm64.tar.gz`
- `amd64.deb` 与 `arm64.deb`
- `fedora_x86_64.rpm` 与 `fedora_aarch64.rpm`

发行版安装包会同时安装 `mark-shot`、辅助脚本、桌面快捷方式、图标和运行元数据。

#### 安装指南

##### Arch Linux (AUR)
Arch Linux 用户可以直接通过 AUR 助手进行安装：
```bash
paru -S mark-shot
# 或
yay -S mark-shot
```

##### 其他发行版 (预编译安装包)
对于其他发行版（如 Ubuntu, Debian, Fedora），请在 Releases 页面下载编译好的安装包并运行以下命令安装：
- **Debian / Ubuntu**:
  ```bash
  sudo apt install ./mark-shot_<version>_amd64.deb
  ```
- **Fedora**:
  ```bash
  sudo dnf install ./mark-shot-<version>-1.x86_64.rpm
  ```

### 系统依赖

#### Wayland (Arch Linux)

```bash
sudo pacman -S --needed base-devel cmake ninja pkgconf qt6-base qt6-wayland layer-shell-qt pipewire grim wl-clipboard
```

#### X11/GNOME (Ubuntu/Debian)

```bash
# 构建工具
sudo apt install build-essential cmake ninja-build pkg-config libpipewire-0.3-dev

# Portal 与剪贴板工具
sudo apt install xdg-desktop-portal pipewire xclip

# Qt 6（若系统仓库无 Qt 6，可通过 aqtinstall 安装到用户目录）
pip install aqtinstall
aqt install-qt linux desktop 6.7.3 gcc_64 --outputdir ~/Qt
```

> **说明**：在 Ubuntu 22.04 等系统自带 Qt 5 的环境下，将 Qt 6 安装到 `~/Qt` 不会影响系统。编译时传入 `-DCMAKE_PREFIX_PATH=$HOME/Qt/6.7.3/gcc_64` 即可。

#### fcitx5 中文输入支持（X11 环境下的 Qt 6）

Qt 6 未自带 fcitx5 输入法插件。若需在 X11 环境下使用 fcitx5 中文输入，需从源码编译该插件：

```bash
sudo apt install libfcitx5utils-dev libfcitx5config-dev libfcitx5core-dev libfcitx5-qt-dev extra-cmake-modules

git clone --depth 1 --branch 5.0.10 https://github.com/fcitx/fcitx5-qt.git /tmp/fcitx5-qt
cmake -B /tmp/fcitx5-qt/build -S /tmp/fcitx5-qt \
  -DCMAKE_PREFIX_PATH=$HOME/Qt/6.7.3/gcc_64 \
  -DENABLE_QT4=OFF -DENABLE_QT5=OFF -DENABLE_QT6=ON
cmake --build /tmp/fcitx5-qt/build

cp /tmp/fcitx5-qt/build/qt6/platforminputcontext/libfcitx5platforminputcontextplugin.so \
   ~/Qt/6.7.3/gcc_64/plugins/platforminputcontexts/
cp /tmp/fcitx5-qt/build/qt6/dbusaddons/libFcitx5Qt6DBusAddons.so* \
   ~/Qt/6.7.3/gcc_64/lib/
```

#### OCR 后端（可选）

```bash
python3 -m venv ~/.local/share/mark-shot/ocr-venv
~/.local/share/mark-shot/ocr-venv/bin/pip install -U pip rapidocr onnxruntime
```

### 构建与编译

```bash
# 使用系统 Qt 6
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# 如果 Qt 6 安装在用户目录，额外指定 CMAKE_PREFIX_PATH
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_PREFIX_PATH=$HOME/Qt/6.7.3/gcc_64

# 执行编译
cmake --build build
```

LayerShellQt 会被自动检测。找到时启用完整 Wayland layer-shell 支持；未找到时编译照常成功，运行时自动降级为标准全屏窗口。

### 安装与集成

```bash
cmake --install build --prefix "$HOME/.local"
```

此命令会安装可执行文件、辅助脚本（`mark-shot-ocr`、`mark-shot-translate`）、桌面快捷方式和图标。

### GNOME Wayland 滚动截图扩展

GNOME Wayland 的滚动截图必须启用 **Mark Shot Scroll Helper** 扩展。没有该扩展时，Mark Shot 无法静默连续截取选定区域，也无法绘制 GNOME 原生滚动预览面板，因此会在 GNOME Wayland 上禁用滚动截图按钮。

扩展文件位于项目仓库的 `packaging/gnome-extension/mark-shot-scroll-helper@snemc.org` 路径。

#### 启用扩展方式

##### 方式 A：通过发行版包安装
如果您是通过发行版包（如 `.deb` 或 `.rpm`）安装的 Mark Shot，该扩展已随系统默认安装。可运行以下命令为当前用户启用该扩展：
```bash
gnome-extensions enable mark-shot-scroll-helper@snemc.org
```
*如果提示找不到该扩展，请注销并重新登录系统后再次尝试。*

##### 方式 B：从仓库源码目录安装
如果您是从源码或本地手动构建的，需要先将该扩展复制到用户的 GNOME 扩展路径下：
```bash
# 定义扩展的 UUID
UUID=mark-shot-scroll-helper@snemc.org

# 创建用户级扩展目录
mkdir -p "$HOME/.local/share/gnome-shell/extensions"

# 从项目仓库中拷贝扩展文件
cp -r "packaging/gnome-extension/$UUID" "$HOME/.local/share/gnome-shell/extensions/"

# 启用该扩展（您可能需要重启 GNOME Shell 或注销并重新登录系统使该扩展生效）
gnome-extensions enable "$UUID"
```

---

## 交互快捷键与手势指南

### 工具切换快捷键

| 快捷键 | 切换的目标工具 | 对应功能说明 |
| :---: | :--- | :--- |
| **V** | 移动 / 导航 (Move / Pan) | 在已有图像模式下，用于平移和拖动图像画布。 |
| **S** | 选择 (Select) | 选中并移动、缩放或删除已绘制的矢量标注。 |
| **P** | 画笔 (Pen) | 自由曲线绘制。 |
| **L** | 直线 (Line) | 绘制笔直的矢量线条。 |
| **H** | 荧光笔 (Highlighter) | 半透明的高亮覆盖，适合标记重点。 |
| **R** | 矩形 (Rectangle) | 绘制矩形线框。 |
| **E** | 椭圆 (Ellipse) | 绘制椭圆形线框。 |
| **A** | 箭头 (Arrow) | 绘制经典的六顶点尖细长锐角箭头。 |
| **T** | 文本 (Text) | 输入并编排富文本，支持 1000px 字号及拖拽联动。 |
| **N** | 序号 (Number) | 自动递增步骤序号标贴。 |
| **M** | 马赛克 (Mosaic) | 进行毛玻璃敏感区域虚化。 |
| **G** | 激光笔 (Laser) | 教学或展示使用的临时痕迹，会自动平滑消融。 |

### 启动界面辅助工具

| 快捷键 | 工具 | 功能说明 |
| :---: | :--- | :--- |
| **C** | 取色器 (Color Picker) | 在选择截图区域之前采样截图像素。滚动鼠标滚轮可调整放大镜大小，左键单击会打开颜色面板，可复制 HEX、RGB、HSL、HSV 和 Qt 等格式。右键或 Esc 返回普通选区。 |
| **R** | 尺子 (Ruler) | 在选择截图区域之前测量坐标。悬停显示当前像素，左键拖拽绘制带像素刻度的测量矩形，并显示宽度、高度、对角线和面积。右键或 Esc 返回普通选区。 |

### 全局操作快捷键

| 快捷键 | 触发动作 |
| :---: | :--- |
| **Esc** | 立即退出并关闭标注窗口。 |
| **Ctrl + C** | 确认所有文字编辑，并将当前截图/已标注选区复制到系统剪贴板。 |
| **Ctrl + S** 或 **Enter / Return** | 确认所有文字编辑，并保存当前截图。 |
| **Ctrl + P** | 将当前选区固定为悬浮贴图窗口。 |
| **Ctrl + Z** | 撤销上一步标注操作。 |
| **Ctrl + Y** 或 **Ctrl + Shift + Z** | 重做已被撤销的标注操作。 |
| **Backspace** 或 **Delete** | 在 **选择 (Select)** 工具激活且选中了某标注时，删除被选中的标注。 |
| **F** | 切换当前截图覆盖范围（选区模式与全屏模式切换）。 |

### 高阶交互操作技巧

- **绘制图形约束**：在绘制 **矩形（Rectangle）** 或 **椭圆（Ellipse）** 时，按住 `Ctrl` 键可强制约束为正方形或正圆形。
- **快速切换至选择工具**：在标注过程中，在画布空白处单击鼠标右键可立即切换到 **选择（Select）** 工具。
- **双击右键快速切换颜色**：在画布空白处双击鼠标右键，可打开环形调色盘，快速切换当前标注工具的颜色。
- **滚轮无级调节**：在对应标注工具激活状态下，滚动鼠标滚轮可实时调整当前工具的线宽、字号大小、序号标贴尺寸或马赛克格网尺寸。
- **画布平移与缩放**：在 **选择（Select）** 工具模式下，或在编辑本地文件时，滚动鼠标滚轮可进行画布无缝缩放，按住鼠标中键拖拽可平移画布。双击 `Ctrl` 键复位缩放与平移。

### 贴图窗口专属交互

| 交互手势 / 快捷键 | 动作效果 |
| :--- | :--- |
| **鼠标左键按住并拖动** | 自由平移和放置桌面贴图位置。 |
| **鼠标滚轮向上/向下** | 贴图窗口等比例无级放大/缩小。 |
| **双击鼠标左键** | 极速关闭该贴图窗口。 |
| **双击 Ctrl 键** | 一键复位贴图窗口的物理尺寸为原始比例。 |
| **鼠标右键单击** | 弹出功能菜单（包括旋转、调整透明度、复制图片文字、翻译、保存、复制、关闭等）。 |
| **Esc 键** | 关闭当前获得焦点的贴图窗口。 |

---

## 发版说明

### 0.1.20

- **滚动截图空闲暂停机制**：当预览面板因空间不足而自动隐藏时，新增 1000 毫秒的滚动空闲检测。若超时无新拼接画面，系统会自动暂停捕获并重新展现预览面板，同时将操作按钮变更为“继续捕获”，方便查看拼接进度。
- **滚动截图参数配置支持**：在设置窗口中集成了滚动截图相关参数的配置项，允许直接通过图形界面修改参数。
- **物理像素裁剪修复**：修复了裁剪选区算法中关于物理像素计算的退化问题，确保裁剪出的截图能准确保留物理像素，避免缩放失真。
- **滚动截图界面与体验优化**：重构滚动截图预览窗口及 GNOME Shell 辅助扩展，移除了手动隐藏预览的操作；精简了扩展端 D-Bus 事件处理；规范了控制按钮布局；修复了预览面板隐藏后仍可能绘制背景残留边框的问题。

<details>
<summary>历史发版说明 (点击展开)</summary>

### 0.1.19

- **GNOME Wayland 滚动截图支持**：引入自带的 `mark-shot-scroll-helper@snemc.org` GNOME Shell 扩展，在启用该扩展的情况下，支持在 GNOME Wayland 环境下完成滚动区域截图和交互预览。
- **贴图后台 OCR 与翻译配置**：新增 `pinnedWindow.autoOcr` 配置项（默认 `false`），贴图创建后默认不执行后台 OCR，仅在右键菜单中触发复制/翻译时按需识别。新增 `translation.autoAfterOcr` 配置项（默认 `false`），在后台 OCR 成功后自动执行翻译。
- **右键复制文本优化**：在贴图窗口右键选择“复制图片文字”时，若当前无识别文本，程序将在后台自动开始 OCR，并在识别完毕后复制到剪贴板。

### 0.1.18

- **可配置快捷键**：全面支持在配置文件中使用 `shortcuts` 或 `hotkeys`（包括别名配置如 `annotation.shortcuts` 等）来自定义工具栏工具、全局动作和启动辅助工具的快捷键。
- **贴图窗口边框**：支持为贴图窗口绘制外边框，可使用布尔值、嵌套对象或顶层属性配置边框的启用状态、颜色和线宽。
- **OCR 依赖诊断**：优化了 OCR 功能在缺失 Python 依赖（如 `rapidocr` 或 `tesseract` 模块）时的错误判定，能够通过捕获报错输出向用户抛出友好的缺失后端提示。
- **取色器交互优化**：微调了取色器点击复制后的关闭行为，在复制到剪贴板后添加了 180 毫秒的延时以确保画面交互顺畅过渡。

### 0.1.17

- **快捷键快速保存**：映射快捷键 `Ctrl+S` 为“快速保存”功能，跳过另存为对话框直接保存截图至系统默认图片目录（例如 `~/Pictures`），并通过系统 D-Bus 发送保存成功通知。同时将工具栏上的保存按钮调整为“另存为”。
- **应用进程启动环境配置**：支持在配置文件顶层配置 `env`（或别名 `environment`）块，在 `QApplication` 创建前应用到 `mark-shot` 自身进程。这适合指定 Qt 启动的环境变量（例如 `"QT_FONT_DPI": 96`），避免系统字体 DPI 覆盖影响到截图与选区的几何大小。
- **测量标尺布局与避让优化**：重构并精细化测量标尺的数据结构，废弃了原先的硬编码魔术数字。优化了状态信息悬浮框的避让算法，防止其与标尺刻度及指示线文字发生重叠。
- **滚动截图智能跟随恢复**：在滚动截图会话中，当拼接出新区域时，即使之前由于用户手动拖拽偏离了视口，系统也会自动重置为跟随状态，贴合最新截图画面。

### 0.1.16

- **启动界面辅助工具**：在选区前新增了取色器（`C` 键激活，支持滚轮调节放大镜和复制多种颜色格式）与尺子（`R` 键激活，支持测量坐标、面积、对角线和长宽）。
- **多显示器截图会话**：重构并支持了多屏幕捕获与多窗口联动，使得多显示器环境下的截图会话更加连贯。
- **自定义默认标注工具与颜色**：支持在配置文件中设置初始标注工具 `defaultTool`、全屏默认工具 `fullscreenDefaultTool` 及默认颜色 `defaultColor`（也可以通过 `--default-tool` / `--default-color` 命令行参数覆盖）。
- **niri 窗口检测优化**：支持在配置中通过 `env`/`environment` 自定义环境变量，以便为 niri 检测脚本传递面板边界、像素偏移（`MARK_SHOT_NIRI_OFFSET_X/Y/WIDTH/HEIGHT`）等参数，修复了状态栏边缘对齐和无效超小窗口检测问题。

### 0.1.15

- **滚动区域灵活调节**：支持通过方向控制直接拖动边缘，动态调整滚动截图的捕获范围。
- **预览面板直观导航**：移除了预览面板底部的进度条，改为直接在总览缩略图的视口框上拖动，提升操作便利性。
- **滚动画面智能跟随**：在预览面板中增加鼠标滚轮滚动查看功能，且在滚回最下方时自动恢复对捕获进度的实时跟随。
- **窗口边界自动贴合**：支持调用外部脚本精确识别 Wayland 窗口边界，内置 `niri` 窗口管理器检测脚本。
- **多桌面环境兼容构建**：提供 Layer Shell 与常规 XDG 窗口的双版本编译支持，并为两个版本生成了 x86_64 AppImage 单文件格式包。
- **剪贴板持久化保存**：修复了程序退出后剪贴板截图丢失的问题，让截图稳定保留在系统剪贴板中供随时粘贴。

### 0.1.14

- **滚动截图体验优化**：优化了桌面 Portal 机制的连接与裁剪逻辑，显著提高图片拼接精度。
- **完善兼容性文档**：说明了 KDE、GNOME 与 X11 等桌面环境的测试版状态，引导用户通过日志排查问题。
- **精确的版本信息展示**：使命令行中的程序版本直接关联 CMake 构建配置，避免 `--version` 输出滞后。

### 0.1.13

- 修复 `niri` 输出存在缩放时滚动截图选区被放大的问题。

### 0.1.12

- 新增 Wayland 原生滚动截图能力，目前作为实验性功能发布。
- 重构标注属性面板，改为更紧凑的图标化布局。
- 新增箭头样式选择，包括类似 KDE/Spectacle 的开放式箭头。
- 修复 GNOME 等 portal 桌面环境中重复注册应用 ID 导致的启动警告。
- 新增 Linux ARM64 tar 包与 Ubuntu/Debian ARM64 `.deb` 安装包。
- 改进对旧版 PipeWire SPA 头文件的兼容性。

滚动截图不保证在 GNOME 或 KDE 中可用。该能力依赖 portal 捕获行为、窗口管理器时序、窗口几何反馈和滚动事件处理。GNOME Shell 与 KWin 的相关行为差异较大，稳定适配成本较高。
</details>

---

## 反馈与交流

### 提交 Issue
若您在运行中遇到问题或有新功能建议，我们推荐使用 GitHub CLI (`gh`) 命令行工具提交 Issue。我们提供了一键收集环境信息并自动生成的脚本，详情请参阅 [Issue 提交指南](.doc/submit-issue-via-gh.md)。

### 交流群
扫描下方二维码可加入使用反馈群：

<img src=".doc/feedback-group.png" width="220" alt="反馈群" />

---

## 许可证说明

本项目基于 **MIT 许可证** 开源，详情请参阅 [LICENSE](LICENSE) 文件。

## 致谢

感谢 [serendipitywgy](https://github.com/serendipitywgy) 通过 `serendipitywgy/mark-shot` 贡献跨桌面兼容性改进、OCR 复制工具栏动作和智能矩形框预选功能。
