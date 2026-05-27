# Mark Shot

[English README](README.md)

<video src="https://github.com/user-attachments/assets/c2298867-06b4-404d-87bc-62ab8d81088b" width="100%" controls></video>

`mark-shot` 是一款基于 Qt 6 开发的高性能 Wayland 截图标注工具，针对 `niri` 等现代 Wayland 窗口管理器进行了原生级深度优化。

它通过 `grim` 瞬间截取屏幕画面，并打开自适应全屏标注覆盖层，为用户提供区域裁切、标注、复制到剪贴板、保存以及桌面贴图等功能。

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
- 支持将截图或标注区域作为一个独立、无边框且置顶的悬浮贴图窗口（`PinnedImageWindow`）固定在屏幕上。
- 支持在贴图窗口中直接选择 OCR 识别出的文字，使用 `Ctrl + C` 或右键菜单复制图片文字。
- 支持通过 OpenAI 兼容接口调用 LLM 翻译 OCR 文本，并将译文按原图位置覆盖渲染到贴图上。
- **便捷交互**：
  - 鼠标左键拖动可自由平移贴图位置。
  - 滚动鼠标滚轮可等比例缩放贴图，双击 `Ctrl` 键复位大小。
  - 双击鼠标左键或按下 `Esc` 键即可关闭贴图。
  - 右键单击唤出菜单，支持多角度旋转、透明度微调（0.2 - 1.0）、复制图片文字、翻译、另存为、复制或关闭。
- **右键误触拦截**：由于 Qt 6 原生上下文菜单的弹出点击机制，右键可能在弹出时误触发菜单项。我们为此设计了局部的 `LeftClickMenuFilter` 事件过滤器，拦截非左键在菜单范围内的动作，从而杜绝误触。

### Wayland 与系统深度集成
- **原生 Wayland 覆盖层**：默认借助 `layer-shell-qt` 创建高速覆盖层；亦支持通过 `--xdg-window` 降级至普通的 XDG 全屏窗口。
- **桌面快捷方式与专属 SVG 图标**：
  - `mark-shot.desktop`：配置为系统全局截图工具，支持系统快捷键直接调用。
  - `mark-shot-edit.desktop`：注册为独立的“Mark Shot 图像编辑器”，可集成到文件管理器（如 Dolphin、Nautilus）的右键“打开方式”菜单中，用于直接打开并标注已有本地图像。
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

### 窗口管理器配置绑定

若要将 `mark-shot` 绑定为系统快捷键，可以修改窗口管理器配置。

以 `niri` 配置为例（修改 `~/.config/niri/config.kdl`）：
```kdl
binds {
    Mod+Shift+S { spawn "mark-shot"; }
}
```

### 拓展命令

右侧动作工具栏提供 **Extensions** 按钮，程序会从 `~/.config/mark-shot/extensions.json` 读取用户自定义命令。配置文件可以是 JSON 数组，也可以是包含 `commands` 数组的 JSON 对象。

```json
{
  "commands": [
    {
      "name": "Long screenshot",
      "command": "./target/release/wayscrollshot {slurp}",
      "workingDirectory": "~/Desktop/projecies/wayscrollshot",
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

如果手动安装到 `/usr/bin`，必须同时安装 `mark-shot`、`mark-shot-ocr` 和 `mark-shot-translate`。否则贴图窗口可以打开，但复制图片文字与翻译功能无法调用后端脚本。

---

## 编译与安装

### 系统依赖 (以 Arch Linux 为例)

在构建前，请先安装以下必要依赖包：

```bash
sudo pacman -S --needed base-devel cmake ninja qt6-base qt6-wayland layer-shell-qt grim wl-clipboard
```

若要使用 RapidOCR 后端，建议创建用户级 Python 虚拟环境：

```bash
python -m venv ~/.local/share/mark-shot/ocr-venv
~/.local/share/mark-shot/ocr-venv/bin/python -m pip install -U pip rapidocr onnxruntime
```

### 构建与编译

使用 CMake 和 Ninja 进行工程构建：

```bash
# 配置项目并输出到 build 目录
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# 执行编译
cmake --build build
```

编译成功后，可执行文件将生成在 `./build/mark-shot` 路径下。

### 安装与集成

要将软件安装到本地用户目录下并自动关联快捷方式和系统右键菜单，请运行：

```bash
cmake --install build --prefix "$HOME/.local"
```

---

## 交互快捷键与手势指南

### 工具切换快捷键

| 快捷键 | 切换的目标工具 | 对应功能说明 |
| :---: | :--- | :--- |
| **V** | 移动 / 导航 (Move / Pan) | 在已有图像模式下，用于平移和拖动图像画布。 |
| **S** | 选择 (Select) | 选中并移动、缩放或删除已绘制的矢量标注，或右键点击选区进行贴图。 |
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
| **Ctrl + C** | 确认所有文字编辑，并将当前截图/已标注选区复制到 Wayland 系统剪贴板。 |
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
| **鼠标右键单击** | 弹出功能菜单（包括旋转、调整透明度、保存、复制、关闭等）。 |
| **Esc 键** | 关闭当前获得焦点的贴图窗口。 |

---

## 许可证说明

本项目基于 **MIT 许可证** 开源，详情请参阅 [LICENSE](LICENSE) file。
