# Mark Shot

[English README](README.md)

<video src="https://github.com/user-attachments/assets/c2298867-06b4-404d-87bc-62ab8d81088b" width="100%" controls></video>

`mark-shot` 是一款基于 Qt 6 开发的高性能截图标注工具。项目最初针对 `niri` 等 Wayland 窗口管理器设计，也支持在 X11/GNOME 桌面环境中完成常规截图与标注工作流。

它可以瞬间截取屏幕画面，并打开自适应全屏标注覆盖层，为用户提供区域裁切、标注、复制到剪贴板、保存以及桌面贴图等功能。

---

## 核心特色

### 标注工具箱
- **画笔与荧光笔**：支持平滑的自由线条绘制与半透明高亮叠色。
- **几何矢量工具**：高精度的直线、矩形与椭圆路径。
- **优化箭头**：采用六顶点经典箭头路径，边缘平滑且支持抗锯齿渲染。
- **双重联动文本**：
  - 支持高达 `1000px` 的字号，可通过滚轮或控制滑块调节。
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

# 选区完成后默认使用移动工具，全屏标注默认使用激光笔，并设置默认颜色
mark-shot --default-tool move --fullscreen-default-tool laser --default-color '#2DD4BF'

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
    "defaultColor": "#2DD4BF"
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

`annotation.defaultTool` 用于指定普通选区完成后默认选中的工具。`annotation.fullscreenDefaultTool` 用于指定全屏标注模式的默认工具，包括 `--fullscreen` 与图片文件标注模式。支持值为 `move`、`select`、`pen`、`line`、`highlighter`、`rectangle`、`ellipse`、`arrow`、`text`、`number`、`mosaic`、`laser`。运行参数 `--default-tool <tool>` 会覆盖普通模式默认工具，并出于兼容性在未设置 `--fullscreen-default-tool <tool>` 时同时作为全屏默认工具。

全屏标注模式没有独立截图选区可移动。如果把全屏默认工具配置为 `move`，Mark Shot 会改用 `select` 进入全屏模式。

`annotation.defaultColor` 用于指定初始标注颜色。使用 `#RRGGBB` 可设置不透明颜色，使用 `#RRGGBBAA` 可包含透明度。运行参数 `--default-color <color>` 会覆盖配置文件中的值。

顶层 `env`（别名：`environment`）会在创建 `QApplication` 前应用到 Mark Shot 自身进程。它适合设置 Qt 启动期环境变量，例如 `"QT_FONT_DPI": 96`，用于避免字体 DPI 覆盖影响截图与选区几何。

`windowDetection.env`（别名：`environment`）会作为环境变量传给检测脚本。
- **Niri 适配脚本** (`mark-shot-window-detection-niri`)：支持 `MARK_SHOT_NIRI_PANEL_EDGE`（`top`、`bottom`、`left`、`right` 或 `none`），也支持通过 `MARK_SHOT_NIRI_OFFSET_X`、`MARK_SHOT_NIRI_OFFSET_Y`、`MARK_SHOT_NIRI_OFFSET_WIDTH`、`MARK_SHOT_NIRI_OFFSET_HEIGHT` 做像素级调整。
- **Hyprland 适配脚本** (`mark-shot-window-detection-hyprland`)：支持 `MARK_SHOT_HYPRLAND_INCLUDE_INACTIVE`（值为 `1` 时检测非活动工作区的窗口，默认 `0` 仅检测当前工作区），也支持通过 `MARK_SHOT_HYPRLAND_OFFSET_X`、`MARK_SHOT_HYPRLAND_OFFSET_Y`、`MARK_SHOT_HYPRLAND_OFFSET_WIDTH`、`MARK_SHOT_HYPRLAND_OFFSET_HEIGHT` 进行像素微调以精准贴合窗口边框。

### 截图前窗口检测与脚本贡献指南

为了使 Mark Shot 在各种 Wayland 合成器中都能实现精准的窗口边界识别，项目采用了一种灵活的外部脚本调用机制：用户可通过 `windowDetection.command` 配置检测脚本，由脚本负责调用合成器的特定命令提取窗口位置，最终将数据转换为统一格式回传给 Mark Shot 消费。

项目已内置了针对以下窗口管理器的边界检测脚本：
- **Niri**：`mark-shot-window-detection-niri`
- **Hyprland**：`mark-shot-window-detection-hyprland`

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

---

## 反馈与交流

扫描下方二维码可加入使用反馈群：

![反馈群](.doc/feedback-group.png)

---

## 许可证说明

本项目基于 **MIT 许可证** 开源，详情请参阅 [LICENSE](LICENSE) 文件。

## 致谢

感谢 [serendipitywgy](https://github.com/serendipitywgy) 通过 `serendipitywgy/mark-shot` 贡献跨桌面兼容性改进、OCR 复制工具栏动作和智能矩形框预选功能。
