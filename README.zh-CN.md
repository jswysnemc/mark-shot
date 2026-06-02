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

### 贴图 OCR 与 LLM 翻译配置

贴图窗口会从 `~/.config/mark-shot/config.json` 读取 OCR 与翻译配置。默认 OCR helper 会优先使用 `rapidocr`，也可以回退到 `tesseract`。翻译 helper 使用 OpenAI 兼容的 `/chat/completions` 接口。

```json
{
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

手动安装时，必须同时安装 `mark-shot`、`mark-shot-ocr` 和 `mark-shot-translate`。否则贴图窗口可以打开，但复制图片文字与翻译功能无法调用后端脚本。

---

## 编译与安装

### 正式版产物

每次正式发布会提供 Linux 二进制压缩包与 Debian 安装包：

- `linux-x86_64.tar.gz` 与 `linux-arm64.tar.gz`
- `amd64.deb` 与 `arm64.deb`

Debian 安装包会同时安装 `mark-shot`、辅助脚本、桌面快捷方式、图标和运行元数据。

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

### 全局操作快捷键

| 快捷键 | 触发动作 |
| :---: | :--- |
| **Esc** | 立即退出并关闭标注窗口。 |
| **Ctrl + C** | 确认所有文字编辑，并将当前截图/已标注选区复制到系统剪贴板。 |
| **Ctrl + S** 或 **Enter / Return** | 确认所有文字编辑，并保存当前截图。 |
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

### 0.1.14

- 改进 portal screencast 协商、裁剪尺寸归一化和首帧等待时序，提高滚动截图稳定性。
- 更新滚动截图文档：KDE、GNOME、X11 以及其他非 `niri` 环境仍是测试目标，提交问题时请提供调试日志。
- 将应用版本号改为使用 CMake 项目版本，避免 `--version` 输出滞后。

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

## 许可证说明

本项目基于 **MIT 许可证** 开源，详情请参阅 [LICENSE](LICENSE) 文件。

## 致谢

感谢 [serendipitywgy](https://github.com/serendipitywgy) 通过 `serendipitywgy/mark-shot` 贡献跨桌面兼容性改进、OCR 复制工具栏动作和智能矩形框预选功能。
