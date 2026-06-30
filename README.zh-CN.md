<div align="center">
  <img src="data/icons/hicolor/scalable/apps/mark-shot.svg" alt="Mark Shot Logo" width="128" />
  <h1>Mark Shot</h1>
  <p>
    <a href="https://github.com/jswysnemc/mark-shot/releases">
      <img src="https://img.shields.io/github/v/release/jswysnemc/mark-shot?color=6da0f2&labelColor=4a5054&label=release&style=flat-square&logo=github" alt="Release" />
    </a>
    <a href="https://gitter.im/mark-shot/community">
      <img src="https://img.shields.io/badge/gitter-join%20chat-46bc99?labelColor=4a5054&style=flat-square&logo=gitter" alt="Gitter" />
    </a>
    <img src="https://img.shields.io/badge/language-C%2B%2B-dfb56c?labelColor=4a5054&style=flat-square&logo=c%2B%2B" alt="Language C++" />
    <img src="https://img.shields.io/badge/framework-Qt%206-92d076?labelColor=4a5054&style=flat-square&logo=qt" alt="Framework Qt 6" />
    <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows-28c0e7?labelColor=4a5054&style=flat-square" alt="Platform Linux | Windows" />
    <img src="https://img.shields.io/badge/display-Wayland%20%7C%20X11-9979d9?labelColor=4a5054&style=flat-square" alt="Display Wayland | X11" />
    <img src="https://img.shields.io/badge/features-Screenshot%20%7C%20OCR%20%7C%20Pin%20%7C%20Scroll-ff8f59?labelColor=4a5054&style=flat-square" alt="Features Screenshot | OCR | Pin | Scroll" />
  </p>
</div>

[English README](README.md)

**标签**：`C++` / `Qt 6` / `屏幕截图` / `图像标注` / `桌面贴图` / `OCR 识别` / `滚动长截图` / `Wayland` / `Windows`


<details>
<summary>演示视频</summary>
<p align="center">
  <video src="https://github.com/user-attachments/assets/4f86fcee-fef9-409e-98ba-1491ecee06c7" width="100%" controls></video>
</p>
</details>

`mark-shot` 是一款基于 Qt 6 开发的高性能截图标注工具。项目最初针对 `niri` 等 Wayland 窗口管理器设计，目前支持在 Linux（X11、GNOME、wlroots/Wayland 桌面）以及 Windows 环境中完成常规截图与标注工作流。

它可以瞬间截取屏幕画面，并打开自适应全屏标注覆盖层，为用户提供区域裁切、标注、复制到剪贴板、保存以及桌面贴图等功能。

---

## 核心特色

### 标注工具箱
- **画笔与荧光笔**：支持平滑的自由线条绘制与半透明高亮叠色。
- **几何矢量工具**：高精度的直线、矩形与椭圆路径。其中矩形支持三种风格切换：
  - `描边`：原有的描边或填充矩形，可选圆角。
  - `高亮`：以 `CompositionMode_Multiply` 与半透明填充实现的荧光笔式覆盖效果。
  - `反色`：对矩形覆盖区域内的像素做 RGB 反相，同时保留外轮廓作为视觉提示。
- **优化箭头**：采用六顶点经典箭头路径，边缘平滑且支持抗锯齿渲染。
- **双重联动文本**：
  - 支持超大字号的无级调节，可通过鼠标滚轮或属性滑块平滑缩放。
  - 引入物理宽度缓冲区设计，避免文本在极高缩放比例下由于渲染抖动产生意外折行。
  - **对角控制点**可实现字号与文本框的等比例联动缩放；**左右边控制线**则仅调节排版边界宽度。
- **激光演示笔**：适用于展示或教学，笔迹会随时间平滑融解消失。
- **自增步骤序号**：点击即可放置依次递增的数字步骤标记。
- **马赛克**：支持对敏感信息执行毛玻璃区域模糊虚化。
- **双框独立调节的放大镜**：放大镜的内层取景框与外层透镜各自带有 resize 把手，矩形透镜每框 8 个角/边把手，圆形透镜每框 4 个上下左右把手。调整任一框时按放大倍率联动另一框，倍率始终保持不变；平移单框时另一框保持原位。
- **启动阶段扫码**：选区前按 `Q` 进入扫码模式，框选二维码或条形码区域后，会打开可复制的识别结果窗口。
- **快速截取显示器**：选区前按 `D` 会立刻截取全部输出屏幕，再按显示器裁切成缩略图；悬浮到缩略图上可复制、编辑或保存该显示器截图。
- **图床上传**：选区后按 `Ctrl+U` 或点击工具栏上传按钮，将当前截图上传到自定义图床（如 ImgURL、sm.ms、imgbb、litterbox 等），上传成功后 URL 自动复制到剪贴板。支持通过 `upload.env` 配置图床参数，或通过 `upload.command` 接入任意自定义上传脚本。

### 贴图悬浮固定（Pin）
- 支持将截图或标注区域作为一个独立、无边框且置顶的悬浮贴图窗口固定在屏幕上。
- 支持在贴图窗口中直接选择 OCR 识别出的文字，使用 `Ctrl + C` 或右键菜单复制图片文字。
- 支持通过 OpenAI 兼容接口调用 LLM 翻译 OCR 文本，并将译文按原图位置覆盖渲染到贴图上。
- **便捷交互**：
  - 鼠标左键拖动可自由平移贴图位置。
  - 滚动鼠标滚轮可等比例缩放贴图。
  - 双击鼠标左键或按下 `Esc` 键即可关闭贴图。
  - 右键单击唤出菜单，支持多角度旋转、复制图片文字、翻译、另存为、复制或关闭。

### 滚动截图
- 通过 PipeWire screencast、交互式滚动覆盖层和图像拼接器，捕获长页面或长区域截图。
- 该功能主要面向 `niri` 以及行为相近的 Wayland 环境；这些环境的输出几何、捕获时序和窗口位置更容易保持稳定。
- **大选区悬浮手柄**：在选择的截图区域过大，以至于屏幕剩余空间不足以展示滚动预览面板时，预览面板会自动隐藏，并在选区边缘显示一个**悬浮拖拽手柄**（带方向箭头的悬浮按钮）。
  - **拖动调整选区**：可按住并拖拽该悬浮手柄，将截图选区沿滚动轴方向平移，以捕获超出初始屏幕范围的内容；
  - **点击切换轴向**：在未开始捕获前，点击悬浮手柄可直接切换滚动方向（垂直/水平）。
- **兼容性说明**：KDE、GNOME、X11 以及其他非 `niri` 环境中的滚动截图仍是测试特性，尚不完善。这些桌面栈的 portal 后端策略、Shell 或窗口管理器行为、窗口几何反馈、帧时序和滚动事件处理存在差异。
- 如果滚动截图无法使用，请使用普通截图流程，或者通过 Mark Shot 拓展命令接入外部长截图工具。
- 如果需要提交滚动截图问题，请先运行 `mark-shot --debug --debug-log /path/to/mark-shot.log` 并复现问题，然后把日志附到 GitHub issue 中。也可以在 `config.json` 中通过 `debug.enabled` 和 `debug.logPath` 开启；`DEBUG=1` 与 `MARK_SHOT_DEBUG_LOG=/path/to/log` 仍然可用。

### 跨显示服务器支持
- **Wayland**：使用 PipeWire portal screencast 支持实验性滚动截图，使用 `grim` 支持 wlroots 截屏，使用 `layer-shell-qt` 创建原生覆盖层，使用 `wl-copy` 持久化剪贴板。
- **X11**：使用 `QScreen::grabWindow` 截屏、全屏置顶窗口作为覆盖层、`xclip` 持久化剪贴板。
- **Windows**：使用 Qt 原生截屏与剪贴板 API 支持基础截图、标注、复制、保存和贴图流程。PipeWire、xdg-desktop-portal、`grim`、XCB 窗口检测、LayerShellQt、GNOME Shell helper 等 Linux 专用后端会在编译期关闭。
- Linux 显示服务器后端会在运行时通过 `$XDG_SESSION_TYPE` 自动检测；Windows 使用 Qt 原生平台后端。

### 桌面集成
- **桌面快捷方式**：
  - `mark-shot.desktop`：配置为系统全局截图工具，支持系统快捷键直接调用。
  - `mark-shot-edit.desktop`：注册为独立的图像编辑器，可集成到文件管理器（如 Dolphin、Nautilus）的右键"打开方式"菜单中。
- 附带高分辨率的 `mark-shot.svg` 与 `mark-shot-edit.svg` 系统矢量图标。

### KDE KWin ScreenShot2 授权

在 KDE Wayland 中，Mark Shot 可以使用 KWin 的 `org.kde.KWin.ScreenShot2` 接口执行精确区域截图。KWin 将该接口视为受限 D-Bus 接口，因此应用对应的桌面文件必须声明授权字段。

<details>
<summary>KDE KWin ScreenShot2 授权与桌面文件配置说明 (点击展开)</summary>

声明授权字段：
```ini
X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2
```

发行版安装包和 `cmake --install` 会自动安装所需桌面文件。如果直接运行本地构建产物而没有安装项目，请创建或更新 `~/.local/share/applications/mark-shot.desktop`：

```ini
[Desktop Entry]
Type=Application
Name=Mark Shot
Comment=Wayland screenshot selection and annotation tool
Exec=/absolute/path/to/mark-shot
Icon=mark-shot
Terminal=false
Categories=Graphics;Utility;
X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2
```

如果通过 KDE 的命令快捷键服务绑定 Mark Shot，还需要创建 `~/.local/share/applications/net.local.mark-shot.desktop`：

```ini
[Desktop Entry]
Type=Application
Name=Mark Shot Shortcut Service
Exec=/absolute/path/to/mark-shot
Icon=mark-shot
Terminal=false
NoDisplay=true
StartupNotify=false
Categories=Utility;
X-KDE-GlobalAccel-CommandShortcut=true
X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2
```

修改桌面文件后，建议注销并重新登录，让 KDE 重新读取桌面文件缓存。如果当前 KDE 会话仍返回 `NoAuthorized`，请重启 KWin 或重启系统一次。
</details>

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

# 直接将本地图片作为贴图窗口打开
mark-shot --pin-image path/to/image.png

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
| `--tray` | 将 Mark Shot 保持运行在系统托盘中，并在平台支持时注册全局截图快捷键。 |
| `--capture` | 当配置中启用托盘自动启动时，强制触发单次截图。 |
| `--pin-image <path>` | 直接将本地图片作为贴图窗口打开，跳过截图与选区流程。 |
| `--debug` | 为本次运行启用调试日志。 |
| `--no-debug` | 为本次运行禁用调试日志，并覆盖配置文件和环境变量。 |
| `--debug-log <path>` | 将调试日志写入指定路径；除非同时设置 `--no-debug`，否则会启用调试日志。 |

### 快捷键绑定

将 `mark-shot` 绑定为系统截图快捷键：

**niri**（修改 `~/.config/niri/config.kdl`）：
```kdl
binds {
    Mod+Shift+S { spawn "mark-shot"; }
}
```

**Hyprland**（修改 `~/.config/hypr/hyprland.conf`）：
```ini
# 绑定 Super+Shift+S 启动 mark-shot 选区截图
bind = SUPER SHIFT, S, exec, mark-shot
# 绑定 Print 按键启动 mark-shot 选区截图
bind = , Print, exec, mark-shot
```

**Sway / i3**（修改 `~/.config/sway/config` 或 `~/.config/i3/config`）：
```ini
# 绑定 Super+Shift+S 启动 mark-shot 选区截图
bindsym Mod4+Shift+S exec mark-shot
# 绑定 Print 按键启动 mark-shot 选区截图
bindsym Print exec mark-shot
```

**GNOME**：在系统设置 → 键盘 → 键盘快捷键 → 自定义快捷键中添加。

**托盘模式**：
```powershell
mark-shot --tray
```

托盘模式默认注册以下全局快捷键：
- `Ctrl+Alt+S`：启动区域截图。

托盘菜单还提供截图、全屏截图和退出等操作。


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

`command` 在类 Unix 系统上通过 `$SHELL -c` 执行，在 Windows 上通过 `%COMSPEC% /C` 执行，因此支持 shell 表达式。使用 `{slurp}` 可把当前选区作为 `x,y widthxheight` 几何字符串传入命令。使用 `{image}` 或 `{imagePath}` 可把当前已渲染选区作为临时 PNG 路径传入命令，使用 `{imageUrl}` 可传入 `file://` URL。这些占位符会自动进行 shell 引用转义，配置中不要再额外加引号。若未使用图片占位符，可设置 `saveImage` 或 `needsImage` 为 `true`，程序会自动把临时 PNG 路径追加到命令末尾。`workingDirectory` 与 `cwd` 等价。`closeOnStart` 默认值为 `true`，命令启动前会先隐藏并关闭 Mark Shot。

### 应用配置文件

Mark Shot 在 Linux 上从 `~/.config/mark-shot/config.json` 读取应用配置，在其他平台上使用 Qt 应用配置目录。贴图窗口同样使用该文件中的 OCR 与翻译配置。默认 OCR helper 会优先使用 `rapidocr`，也可以回退到 `tesseract`。翻译 helper 使用 OpenAI 兼容的 `/chat/completions` 接口。

<details>
<summary>应用配置 JSON 示例与配置项详细说明 (点击展开)</summary>

```json
{
  "env": {
    "QT_FONT_DPI": 96
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
| `capture.freezeScope` | 字符串 | `"all-screens"` | 普通区域截图模式下的显示器冻结范围。当为多显示器环境且没有显式指定捕获全部输出时生效。支持的值包括：`all-screens`（冻结所有显示器）、`cursor-screen`（仅冻结鼠标指针当前所在的显示器）。别名：`freezeScope`、`freezeDisplayScope` 等。 |
| `capture.wayland.kde.kwinScreenshot.enabled` | 布尔值 | `true` | 是否在 KDE Wayland 环境下启用 KWin 的 `org.kde.KWin.ScreenShot2` 限制级别 DBus 接口截屏功能。如果关闭，将自动回退到常规 Portal 截屏。 |
| `debug.enabled` | 布尔值 | `false` | 在 Linux 和 Windows 上启用调试日志。命令行参数 `--debug` / `--no-debug` 会覆盖此项；除非设置 `--no-debug`，否则 `DEBUG=1` 仍会启用日志。 |
| `debug.logPath` | 字符串 | 系统临时目录 `mark-shot-scroll.log` | 调试日志输出路径。命令行参数 `--debug-log` 会覆盖此项；未设置配置或命令行路径时，`MARK_SHOT_DEBUG_LOG` 仍然有效。 |
| `annotation.defaultTool` | 字符串 | `"move"` | 选区完成后默认激活的标注工具。支持的值包括：`move`、`select`、`pen`、`line`、`highlighter`、`rectangle`、`ellipse`、`arrow`、`text`、`number`、`mosaic`、`magnifier`、`laser`。命令行参数 `--default-tool` 会覆盖此项。 |
| `annotation.fullscreenDefaultTool` | 字符串 | `"laser"` | 全屏标注模式下默认激活的工具。命令行参数 `--fullscreen-default-tool` 会覆盖此项。若在全屏模式下配置为 `move`，系统会自动降级为使用 `select`。 |
| `annotation.defaultColor` | 字符串 | `"#FF4D4D"` | 初始标注颜色。支持不透明的十六进制格式 `#RRGGBB` 或包含透明度的 `#RRGGBBAA`。命令行参数 `--default-color` 会覆盖此项。 |
| `save.pathTemplate` | 字符串 | `"{pictures}/mark-shot/mark-shot-{datetime}.png"` | 保存截图文件的路径模板（包括保存动作和另存为的初始文件名）。父级目录在保存时若不存在会自动创建。别名包括：`save.path`、`save.location`、最外层的 `savePathTemplate` 以及 `save.directory`（目录模板）。 |
| `save.directoryTemplate` | 字符串 | `""` | 仅指定保存目录模板。指定后，文件名会自动采用默认的 `mark-shot-{datetime}.png`。别名包括：`save.directory`、`save.dir`、`save.folder`。 |
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
| `windowDetection.env` | 对象 | `{}` | 传给窗口边界检测脚本的环境变量。别名：`environment`。<br>• **Niri 适配脚本**：支持 `MARK_SHOT_NIRI_PANEL_EDGE`（`top`/`bottom`/`left`/`right`/`none`）以及像素偏移 `MARK_SHOT_NIRI_OFFSET_X/Y/WIDTH/HEIGHT`。<br>• **Hyprland 适配脚本**：支持 `MARK_SHOT_HYPRLAND_INCLUDE_INACTIVE`（`1`/`0`）以及像素偏移 `MARK_SHOT_HYPRLAND_OFFSET_X/Y/WIDTH/HEIGHT`。 |

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

## 编译与安装

### 安装指南

##### Arch Linux (AUR)
Arch Linux 用户可以直接通过 AUR 助手进行安装：
```bash
# 从源码编译安装
paru -S mark-shot
# 或
yay -S mark-shot

# 安装预编译二进制包
paru -S mark-shot-bin
# 或
yay -S mark-shot-bin
```

`mark-shot` 从源码编译；`mark-shot-bin` 从 GitHub Releases 下载预编译 pacman 包安装。

##### NixOS
NixOS 用户可以通过添加 Flake input 来进行安装
```nix
# flake.nix
mark-shot = {
  url = "github:jswysnemc/mark-shot";
  inputs.nixpkgs.follows = "nixpkgs";
};

# home-manager
home.packages = with pkgs; [
  # 其他用户应用
  inputs.mark-shot.packages.${pkgs.stdenv.hostPlatform.system}.default
]
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

Mark Shot 的文字识别功能依赖内置的 `mark-shot-ocr` Python 脚本。该脚本支持 **RapidOCR**（首选，基于 PaddleOCR PP-OCR 模型）和 **Tesseract**（回退）。Linux 上会自动安装该脚本；Windows 上需要手动配置。

<details>
<summary><b>Linux</b></summary>

```bash
python3 -m venv ~/.local/share/mark-shot/ocr-venv
~/.local/share/mark-shot/ocr-venv/bin/pip install -U pip rapidocr onnxruntime
```

安装完成后 `mark-shot-ocr` 会被自动发现，无需额外配置。

**环境变量**（可选）：

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `MARK_SHOT_OCR_VERSION` | PaddleOCR 版本（`PP-OCRv5`、`PP-OCRv4` 等） | `PP-OCRv5` |
| `MARK_SHOT_OCR_MODEL_TYPE` | 模型大小：`mobile` 或 `server` | `mobile` |
| `MARK_SHOT_OCR_MODEL_DIR` | 自定义模型存储目录 | `~/.local/share/mark-shot/models` |
| `MARK_SHOT_OCR_NO_VENV` | 设为 `1` 禁用自动切换虚拟环境 | — |
| `MARK_SHOT_OCR_PYTHON` | 指定用于 re-exec 的 Python 解释器路径 | `~/.local/share/mark-shot/ocr-venv/bin/python` |

</details>

<details>
<summary><b>Windows</b></summary>

内置的辅助脚本不会在 Windows 上自动安装，需要手动完成以下步骤：

**1. 安装 Python 3**

从 [python.org](https://www.python.org/downloads/) 下载安装 Python 3.10 或更高版本。安装时请勾选 **Add python.exe to PATH**。

**2. 复制 OCR 辅助脚本**

将 [Mark Shot 仓库](https://github.com/jswysnemc/mark-shot) 中的 `scripts/mark-shot-ocr` 复制到本地目录，例如 `%LOCALAPPDATA%\mark-shot\mark-shot-ocr.py`。

```powershell
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\mark-shot"
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/jswysnemc/mark-shot/main/scripts/mark-shot-ocr" `
  -OutFile "$env:LOCALAPPDATA\mark-shot\mark-shot-ocr.py"
```

**3. 创建虚拟环境并安装依赖**

```powershell
python -m venv "$env:LOCALAPPDATA\mark-shot\ocr-venv"
& "$env:LOCALAPPDATA\mark-shot\ocr-venv\Scripts\pip.exe" install -U pip rapidocr onnxruntime
```

> `onnxruntime` 提供 CPU 推理。如果有兼容的 GPU，可以安装 `onnxruntime-directml` 或 `onnxruntime-gpu` 以加速识别。

**4. 在 `config.json` 中配置 `ocr.command`**

打开 `%LOCALAPPDATA%\mark-shot\config.json`（不存在则新建），设置 `ocr.command`：

```json
{
  "ocr": {
    "enabled": true,
    "backend": "rapidocr",
    "command": "\"%LOCALAPPDATA%\\mark-shot\\ocr-venv\\Scripts\\python.exe\" \"%LOCALAPPDATA%\\mark-shot\\mark-shot-ocr.py\" --format json --backend rapidocr {image}",
    "timeoutMs": 30000
  }
}
```

将 `%LOCALAPPDATA%` 替换为实际展开后的路径（如 `C:\Users\你的用户名\AppData\Local`）。`{image}` 占位符在运行时会被替换为临时截图路径；如果省略，Mark Shot 会自动追加。

> **提示**：设置环境变量 `MARK_SHOT_OCR_NO_VENV=1` 可以跳过脚本内置的虚拟环境自动检测，因为已经直接使用了虚拟环境中的 Python。

</details>

#### 扫码后端（可选）

```bash
python3 -m venv ~/.local/share/mark-shot/code-scan-venv
~/.local/share/mark-shot/code-scan-venv/bin/pip install -U pip zxing-cpp pillow
```

扫码 helper 优先使用 `zxing-cpp`，支持 QR Code、Data Matrix、Aztec、PDF417、EAN、UPC、Code 39、Code 93、Code 128 等常见格式。如果安装了 `pyzbar` 或 OpenCV，也会作为回退后端使用。

#### 图床上传后端（可选）

图床上传功能默认使用内置的 `mark-shot-upload` Python 脚本，无需额外安装依赖（仅使用 Python 3 标准库）。该脚本通过环境变量配置图床参数，支持任意兼容 multipart/form-data 上传协议的图床服务。

<details>
<summary>内置 helper 支持的环境变量</summary>

| 环境变量 | 说明 | 默认值 |
|---------|------|--------|
| `MARK_SHOT_UPLOAD_URL` | **必填**，图床上传接口 endpoint | — |
| `MARK_SHOT_UPLOAD_FIELD` | 文件字段名 | `image` |
| `MARK_SHOT_UPLOAD_API_KEY` | API Key / Token | — |
| `MARK_SHOT_UPLOAD_AUTH_HEADER` | 认证头名称 | `Authorization` |
| `MARK_SHOT_UPLOAD_AUTH_SCHEME` | 认证方案（如 `Bearer`），留空则直接用 API Key | `Bearer` |
| `MARK_SHOT_UPLOAD_URL_PATH` | URL 在 JSON 响应中的点分路径（如 `data.url`） | 自动探测 |
| `MARK_SHOT_UPLOAD_DELETE_URL_PATH` | 删除 URL 路径 | 自动探测 |
| `MARK_SHOT_UPLOAD_HEADER_xxx` | 自定义请求头（如 `MARK_SHOT_UPLOAD_HEADER_X-Custom=foo`） | — |
| `MARK_SHOT_UPLOAD_FIELD_xxx` | 额外表单字段（如 `MARK_SHOT_UPLOAD_FIELD_album=123`） | — |

</details>

<details>
<summary>配置示例：ImgURL V3</summary>

```json
"upload": {
  "env": {
    "MARK_SHOT_UPLOAD_URL": "https://www.imgurl.org/api/v3/upload",
    "MARK_SHOT_UPLOAD_FIELD": "file",
    "MARK_SHOT_UPLOAD_API_KEY": "sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "MARK_SHOT_UPLOAD_URL_PATH": "data.url"
  }
}
```

ImgURL V3 使用 `Authorization: Bearer <token>` 认证（`AUTH_SCHEME` 默认 `Bearer`，无需修改）。

</details>

<details>
<summary>配置示例：sm.ms</summary>

```json
"upload": {
  "env": {
    "MARK_SHOT_UPLOAD_URL": "https://sm.ms/api/v2/upload",
    "MARK_SHOT_UPLOAD_FIELD": "smfile",
    "MARK_SHOT_UPLOAD_API_KEY": "你的Token",
    "MARK_SHOT_UPLOAD_AUTH_SCHEME": "",
    "MARK_SHOT_UPLOAD_URL_PATH": "data.url"
  }
}
```

sm.ms 直接用 Token 作为 Authorization 值，因此 `AUTH_SCHEME` 设为空字符串。

</details>

<details>
<summary>配置示例：imgbb</summary>

```json
"upload": {
  "env": {
    "MARK_SHOT_UPLOAD_URL": "https://api.imgbb.com/1/upload?key=你的API_KEY",
    "MARK_SHOT_UPLOAD_FIELD": "image",
    "MARK_SHOT_UPLOAD_URL_PATH": "data.url"
  }
}
```

imgbb 通过 URL 查询参数传递 API Key，无需设置 `API_KEY`。

</details>

<details>
<summary>配置示例：litterbox（临时图床，无需 API Key）</summary>

```json
"upload": {
  "command": "curl -sf --max-time 30 -A 'Mozilla/5.0' -F reqtype=fileupload -F time=72h -F fileToUpload=@{image} https://litterbox.catbox.moe/resources/internals/api.php",
  "timeoutMs": 35000
}
```

litterbox 响应为纯文本 URL（非 JSON），Mark Shot 会自动识别 `http://`/`https://` 开头的输出作为上传结果。

</details>

<details>
<summary>自定义上传命令</summary>

若内置 helper 无法满足需求，可通过 `upload.command` 接入任意自定义上传脚本。命令需满足：

1. **退出码**：成功时退出码为 0，非零视为失败
2. **输出格式**（二选一）：
   - **JSON**：`{"url":"https://...","deleteUrl":"https://...","errors":[]}`（`url` 必填，其他可选）
   - **纯文本 URL**：stdout 第一行非空内容以 `http://` 或 `https://` 开头
3. **占位符**：支持 `{image}`、`{imagePath}`、`{imageUrl}`；若命令中未包含占位符，Mark Shot 会自动在命令末尾追加临时图片路径

```json
"upload": {
  "command": "/path/to/your-uploader.sh --file {image} --json",
  "timeoutMs": 30000,
  "env": {
    "UPLOADER_API_KEY": "xxx"
  }
}
```

`upload.env` 中的环境变量会同时传递给自定义命令，便于复用配置。

</details>

#### Windows

安装与当前编译器匹配的 Qt 6、CMake、Ninja，以及支持 C++17 的编译器，例如 MSVC 或 MinGW。Windows 构建不需要 Qt DBus、PipeWire、X11/XCB、LayerShellQt、`grim`、`wl-copy` 或 `xclip`。

```powershell
cmake -S . -B build-windows -G Ninja -DCMAKE_BUILD_TYPE=Release `
  -DCMAKE_PREFIX_PATH=C:\Qt\6.7.3\msvc2019_64
cmake --build build-windows
```

当前 Windows 支持范围是普通截图与图片标注。滚动截图、合成器专用窗口检测和 Linux 桌面快捷方式在 Windows 上不可用。内置的 Python 辅助脚本（`mark-shot-ocr`、`mark-shot-code-scan`、`mark-shot-translate`）不会自动安装，请参考上方的 [OCR 后端](#ocr-后端可选)、[扫码后端](#扫码后端可选)和翻译章节进行手动配置。

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

或者使用 nix

```bash
nix build
```

LayerShellQt 会被自动检测。找到时启用完整 Wayland layer-shell 支持；未找到时编译照常成功，运行时自动降级为标准全屏窗口。

### 安装与集成

```bash
cmake --install build --prefix "$HOME/.local"
```

此命令会安装可执行文件、辅助脚本（`mark-shot-ocr`、`mark-shot-code-scan`、`mark-shot-translate`、`mark-shot-upload`）、桌面快捷方式和图标。

### GNOME Wayland 滚动截图扩展

GNOME Wayland 的滚动截图必须启用 **Mark Shot Scroll Helper** 扩展。没有该扩展时，Mark Shot 无法静默连续截取选定区域，也无法绘制 GNOME 原生滚动预览面板，因此会在 GNOME Wayland 上禁用滚动截图按钮。

扩展文件位于项目仓库的 `packaging/gnome-extension/mark-shot-scroll-helper@snemc.org` 路径。

<details>
<summary><b>展开/折叠 GNOME Wayland 滚动截图扩展安装与启用指南</b></summary>

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

验证 helper D-Bus 接口是否可用：

```bash
gdbus call --session \
  --dest org.gnome.Shell \
  --object-path /org/gnome/Shell/Extensions/MarkShotScrollHelper \
  --method org.gnome.Shell.Extensions.MarkShotScrollHelper.Version
```

预期结果为 `('4.2',)`。启用扩展后，请重新启动 `mark-shot`。

</details>

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
| **Q** | 扫码 (Code Scanner) | 进入二维码与条形码扫码模式。框选区域后会识别其中的码内容，并在可复制窗口中展示结果。右键或 Esc 返回普通选区。 |
| **D** | 显示器截取 (Display Capture) | 立即截取全部输出屏幕，按显示器裁切并显示缩略图；悬浮到缩略图上可复制、编辑或保存。 |

### 全局操作快捷键

| 快捷键 | 触发动作 |
| :---: | :--- |
| **Esc** | 立即退出并关闭标注窗口。 |
| **Ctrl + C** | 确认所有文字编辑，并将当前截图/已标注选区复制到系统剪贴板。 |
| **Ctrl + S** 或 **Enter / Return** | 确认所有文字编辑，并保存当前截图。 |
| **Ctrl + P** | 将当前选区固定为悬浮贴图窗口。 |
| **Ctrl + U** | 将当前截图上传到自定义图床，上传成功后 URL 自动复制到剪贴板。 |
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
| **鼠标右键单击** | 弹出功能菜单（包括旋转、复制图片文字、翻译、保存、复制、关闭等）。 |
| **Esc 键** | 关闭当前获得焦点的贴图窗口。 |

---

## 发版说明

### 0.1.32

- **启动快捷提示面板**：将原先居中的启动提示条改为 PixPin 风格的左侧垂直快捷键面板，默认显示在左下角，鼠标接近时自动避让到左上角。
- **输入设备提示**：在启动快捷提示面板中加入键盘、鼠标和滚轮图标，让每个操作对应的输入方式更直观。
- **窗口层级选择**：改进 GNOME、KDE Plasma、Hyprland、X11 和 Windows 下的窗口排序，选区时优先匹配视觉上最上层的窗口。
- **Wayland Fcitx5 候选窗支持**：调整 layer-shell 光标矩形处理，使 fcitx5 候选窗可以在 Wayland 截图覆盖层下正确显示。
- **设置齿轮图标更新**：重绘工具栏设置按钮和设置页 General 导航图标，使用更清晰的齿轮图形，避免看起来像太阳。
- **托盘模式兼容性**：修复直接以托盘模式启动时，在系统托盘暂不可用环境下的启动行为。
- **Wayland 文本编辑器宽度修复**：修复分数缩放 Wayland 环境下标注文本编辑器异常变窄的问题。

<details>
<summary>历史版本</summary>

### 0.1.31

- **命令行图片贴图**：新增 `--pin-image <path>` 参数，可直接将本地图片作为贴图窗口打开，跳过截图与选区流程。
- **取色器颜色历史**：启动取色器现可记录最近拾取的颜色，保存在 `config.json` 的 `colorPicker.history` 中（最多 7 条 `#RRGGBBAA`），并在颜色面板以色块展示。
- **界面语言设置**：新增可配置的 `ui.language` 选项（`system` / `english` / `chinese`），可在设置通用页选择；取代旧版最外层 `language` 字段。
- **按桌面环境自动选择窗口检测**：Mark Shot 运行时会自动探测当前桌面环境（GNOME、KDE Plasma、Hyprland、Niri）并选择匹配的检测脚本，其他 Wayland 会话回退到 niri 脚本，X11 会话使用原生 X11 检测。配置命令与环境不匹配时仅在内存中纠正，不修改 `config.json`。
- **GNOME 遮挡窗口过滤**：GNOME Shell 滚动助手扩展现会从检测结果中过滤完全被遮挡的窗口。
- **预编译 AUR 包**：新增 `mark-shot-bin` AUR 包，从 GitHub Releases 下载预编译 pacman 包安装，与源码版 `mark-shot` 并存。
- **GNOME Adwaita 配色修复**：在 `qApp` 层覆盖应用调色板，使深色配色在 GNOME Adwaita 下彻底替换 libqtk3 基础调色板。
- **AUR 可选依赖**：新增 `python-rapidocr`、`python-pillow`、`python-zxing-cpp` 作为 OCR/扫码首选可选依赖。

### 0.1.30

- **设置配置对话框**：新增独立设置窗口，包含通用、截图、标注、贴图、滚动、快捷键、存储、集成、高级等多个分页。原先只能通过文件修改的配置项现在均可在一个界面内编辑，底层仍读写同一份 `config.json`，并统一了设计令牌与自定义导航侧边栏。
- **开机启动**：通用页新增 `开机启动` 开关。Linux 会写入 XDG `autostart` 桌面项，Windows 会写入当前用户的 `Run` 注册表项；在不支持的平台上开关会自动禁用。
- **Portal 全局快捷键支持**：新增基于 `xdg-desktop-portal` 的 `GlobalShortcuts` 后端，使全局截图热键在无 X11 的 Wayland 环境下也能生效。
- **贴图文本选择开关**：贴图窗口新增可配置的文本选择开关，并将 `pinned_window_config` 拆分为独立模块。
- **截图会话中的设置入口**：从工具栏或快捷键打开设置时，会先关闭冻结的截图会话，并在下一轮事件循环再弹出设置对话框，避免与 layer-shell 截图窗口冲突。
- **Wayland 贴图窗口定位**：抽出 layer-shell 几何计算并改进 resize 控制器，修复贴图窗口在多屏或屏幕外的放置问题。

### 0.1.29

- **放大镜双框独立调节**：放大镜的内层取景框与外层透镜各自带有 resize 把手，矩形透镜每框 8 个角/边把手，圆形透镜每框 4 个上下左右把手。调整任一框时按 `magnifierScale` 联动另一框，倍率始终保持不变；平移单框时另一框保持原位。
- **矩形高亮与反色风格**：矩形工具新增风格选择器，提供三种模式——`描边`（原有的描边或填充矩形，可选圆角）、`高亮`（以 `CompositionMode_Multiply` 与半透明填充实现的荧光笔式覆盖）、`反色`（对矩形覆盖区域的像素做 RGB 反相，并保留外轮廓作为视觉提示）。在 `高亮` 与 `反色` 模式下会自动隐藏填充开关与圆角滑块。
- **工具默认值跨会话持久化**：标注工具默认值（颜色、不透明度、各工具笔宽、矩形填充/圆角/风格、放大镜倍率与形状、箭头/荧光笔/数字步骤风格、文本字体、文字背景色）会通过独立的 `annotation-state.json` 跨会话保留。写盘通过 `QSaveFile` 原子提交，并在每个修改默认值的入口触发后立即写入。

### 0.1.28

- **可配置图片剪贴板策略**：新增 `clipboard.image.mode` 配置，支持 `image/png`、`url` 与 `threshold` 三种模式。默认使用直接 `image/png` 剪贴板数据，以改善办公套件和浏览器输入框的贴图兼容性，同时仍可通过 `thresholdM` 将大图切换为文件 URL 模式。
- **运行时默认配置创建**：程序启动时如果缺少 `config.json`，会自动创建默认配置文件，并写入新的剪贴板默认配置。
- **Shift 约束直线绘制**：绘制直线、箭头或直线模式荧光笔时，按住 `Shift` 会将线条吸附到水平、垂直或 45 度方向。

### 0.1.27

- **直线与箭头多骨架点编辑**：支持在直线和箭头标注上添加、拖拽、以及删除多个骨架点（控制点）。路径采用连续二次贝塞尔曲线拟合，并确保尖端精准落于终点。
- **快捷键与交互优化**：增强了键盘和滚轮的交互（例如使用 Backspace/Delete 键删除选中的骨架控制点），并将输入快捷键处理逻辑进行了模块化拆分。

### 0.1.26

- **自定义保存路径与占位符支持**：引入了截图保存路径模板和目录模板机制（`save.pathTemplate` 和 `save.directoryTemplate`），支持包括 `{pictures}`, `{datetime}`, `{selection.width}` 以及自定义日期格式 `{datetime:yyyy-MM-dd}` 等 30 多个动态占位符，实现灵活的文件命名和分类存储。
- **KDE KWin 截屏控制开关**：新增 `capture.wayland.kde.kwinScreenshot.enabled` 配置项，允许关闭或开启 KDE 限制级 API 截图方式，便于进行截图后端的回退调试。
- **文档优化与内容折叠**：精简重构了用户手册，包括对 KDE 繁杂的 KWin 授权文件进行了折叠处理，减少无关信息的视觉干扰。

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
