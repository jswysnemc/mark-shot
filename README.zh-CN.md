# Mark Shot

[English README](README.md)

`mark-shot` 是一个基于 Qt 6 的 Wayland 截图标注工具，适合 niri 等窗口管理器使用。它通过 `grim` 获取冻结的屏幕画面，打开全屏标注界面，然后支持选区、标注、保存、复制，或使用其他桌面应用打开处理后的图片。

## 功能

- 默认捕获当前输出，可通过 `--all-outputs` 捕获整个合成器画面。
- 可通过 `mark-shot path/to/image.png` 打开已有图片进行标注。
- 支持区域选择、全屏标注，以及选区移动和缩放。
- 提供画笔、直线、荧光笔、矩形、椭圆、箭头、文本、编号和马赛克工具。
- 提供激光笔工具，可用于教学场景下的临时高亮，标注会自动淡出。
- 支持撤销、重做、保存、复制到 Wayland 剪贴板，以及使用其他应用打开。
- 默认使用 layer-shell 覆盖层，也可通过 `--xdg-window` 使用普通全屏 xdg 窗口。
- 面向支持 `wlr-screencopy` 的 Wayland 合成器，尤其适合 niri。

## 编译

Arch Linux 依赖：

```bash
sudo pacman -S --needed base-devel cmake ninja qt6-base qt6-wayland layer-shell-qt grim wl-clipboard
```

编译并运行：

```bash
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
./build/mark-shot
```

安装到本地：

```bash
cmake --install build --prefix "$HOME/.local"
```

## 使用

捕获当前输出：

```bash
mark-shot
```

捕获所有输出：

```bash
mark-shot --all-outputs
```

跳过选区步骤，直接标注完整截图：

```bash
mark-shot --fullscreen
```

打开已有图片并直接标注：

```bash
mark-shot path/to/image.png
```

使用普通全屏 xdg 窗口而不是 layer-shell：

```bash
mark-shot --xdg-window
```

niri 快捷键示例：

```kdl
Mod+Shift+S { spawn "mark-shot"; }
```

## 控制

- 拖动鼠标创建选区。
- `V`、`S`、`P`、`L`、`H`、`R`、`E`、`A`、`T`、`N`、`M`、`G` 分别切换到移动、选择、画笔、直线、荧光笔、矩形、椭圆、箭头、文本、编号、马赛克和激光笔。
- `F` 在当前选区和完整截图之间切换标注范围。
- 在完整截图标注模式下，布局按钮可在横向和纵向工具栏之间切换。
- 移动工具可拖动选区，也可从边缘和角落调整选区尺寸。
- 选择工具可选中已有标注，拖动对象或控制点可移动或缩放标注。
- 鼠标中键切换到选择工具。
- 切换绘制工具或选中标注后会显示附着在工具栏上的小属性面板，可用滑条调整粗细和透明度，使用支持透明度的 Qt 颜色选择器，并按对象类型调整文本背景色、填充、矩形圆角和文字字体。
- 点击选中框上的红色 `x` 可只删除当前标注。
- 绘制矩形或椭圆时按住 `Ctrl`，可约束为正方形或圆形。
- 右键打开径向调色板。
- 鼠标滚轮调整当前工具或选中标注的线宽、编号尺寸、文字尺寸或马赛克块大小。
- `Ctrl+Z` 撤销，`Ctrl+Shift+Z` 或 `Ctrl+Y` 重做。
- `Ctrl+C` 复制编辑后的选区。
- `Ctrl+S` 或 `Enter` 保存编辑后的选区。
- `Esc` 退出。

## 许可证

MIT License。
