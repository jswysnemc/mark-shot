# Mark Shot 用户操作手册

本文档介绍 Mark Shot 的日常使用，重点覆盖**指定窗口/组件悬停框选截图**功能
（鼠标移动时自动跟踪并高亮光标下方的窗口，单击即可选中该窗口）、标注工作流、
headless 截图与配置。

> 本仓库的文档在社区版 fork 中编写，并镜像到上游与商业版仓库。商业版额外包含
> 针对其本地 MCP server 的专门章节。

---

## 1. 快速开始

### 1.1 启动

开始一次区域截图会话：

```bash
mark-shot
```

按下桌面快捷键（见 § 8）或在终端运行。焦点显示器上会打开一个冻结的全屏覆盖层。
移动鼠标拉出选择矩形，松开后进入标注编辑器。

### 1.2 便携版应用

如果使用便携版（`mark-shot-upstream` / `mark-shot-community` /
`mark-shot-enterprise`），请用自带的启动脚本启动，以便找到内置的 Qt 库、
插件与辅助脚本：

```bash
portable/mark-shot-community/bin/run-mark-shot.sh
```

启动脚本会把其 `bin/` 目录加入 `PATH`，这是窗口检测脚本
（`mark-shot-window-detection-*`）以及 OCR / 上传辅助脚本能够被找到的必要条件。

---

## 2. 窗口 / 组件悬停框选

Mark Shot 在选区开始前会检测当前桌面的窗口。覆盖层打开后，**移动鼠标即可在
光标所在窗口上高亮显示青色边框**；**直接单击（不拖动）即可选中整个窗口**作为
截图区域，随后可直接标注、复制、钉住或保存。

高亮的窗口来自按合成器区分的检测脚本，它们在覆盖层出现前运行：

| 桌面 | 检测来源 | 说明 |
| :--- | :--- | :--- |
| GNOME Wayland | 内置 `mark-shot-scroll-helper@snemc.org` Shell 扩展（D-Bus） | 需要启用扩展（见 § 2.1） |
| KDE Plasma Wayland | 通过 `qdbus6` / `qdbus` + journalctl 的一次性 KWin 脚本 | 需要 KWin 会话 |
| Hyprland | `hyprctl -j clients` | |
| niri | `niri msg -j windows` + 配置解析 | |
| X11 | 进程内 XCB 枚举 `_NET_CLIENT_LIST_STACKING` | 无需脚本 |
| Windows | 进程内 `EnumWindows` | 无需脚本 |

只跟踪**顶层窗口**。窗口内部的单个控件（“组件”）在 Wayland 合成器中不暴露，
因此所有平台上的悬停框选都针对整个窗口。

### 2.1 GNOME Wayland：启用辅助扩展

```bash
gnome-extensions enable mark-shot-scroll-helper@snemc.org
```

验证 D-Bus 辅助接口可用：

```bash
gdbus call --session \
  --dest org.gnome.Shell \
  --object-path /org/gnome/Shell/Extensions/MarkShotScrollHelper \
  --method org.gnome.Shell.Extensions.MarkShotScrollHelper.Version
# -> ('5',)
```

如果调用失败，请注销后重新登录（X11 上重启 GNOME Shell）再试。没有扩展时
GNOME 检测脚本会报错退出，悬停框选保持关闭（普通拖拽框选不受影响）。

### 2.2 使用方法

1. 触发截图（`mark-shot` 或桌面快捷键）。
2. 不按任何鼠标键，把光标移到某个窗口上，出现青色边框标示将被选中的窗口。
3. **单击一次**（按下并松开、位移不超过几像素）即可选中该窗口。窗口重叠时，
   优先选中光标下最顶层的窗口（按 z 序）。
4. 松开后进入标注编辑器，窗口刚好被精确框选。
5. 如果只想做**手动**框选，正常拖动矩形即可——一旦拖动超过单击阈值，悬停框
   即被忽略。

当取色器（`C`）或标尺（`R`）启动工具激活时，悬停高亮被禁用；二维码扫描（`Q`）、
显示器快照（`D`）以及 GIF / 视频录制的启动模式仍可使用。

### 2.3 多显示器下正确选择窗口

窗口检测按每个捕获目标分别执行。在多显示器环境，每个冻结窗口只接收与其自身
几何相交的窗口，因此悬停框与你在该显示器上看到的内容一致。

### 2.4 启用 / 禁用

该功能默认开启（`windowDetection.enabled = true`）。可在
**设置 → 高级 → 窗口检测已启用** 中切换，或编辑 `~/.config/mark-shot/config.json`：

```json
{
  "windowDetection": {
    "enabled": true,
    "command": "mark-shot-window-detection-gnome",
    "timeoutMs": 1000,
    "env": {}
  }
}
```

- `command`：检测脚本。GNOME / KDE / Hyprland / niri Wayland 下会自动选择与
  当前会话匹配的内置 `mark-shot-window-detection-*` 脚本；X11 与 Windows 在
  进程内枚举平台，`command` 可留空。**用户自定义命令（例如绝对路径）始终会被
  尊重，不会被覆盖。**
- `timeoutMs`：等待脚本的最长时间（100–30000 ms，默认 1000）。
- `env`：传给脚本的额外环境变量。各合成器的微调（偏移量等）见脚本头注释。

### 2.5 故障排查

| 现象 | 检查项 |
| :--- | :--- |
| GNOME Wayland 下没有青色框 | 扩展是否启用？上面 `gdbus` 调用必须返回版本号 |
| X11 / Windows 下没有青色框 | 平台枚举是内置的，无需操作；确认没有启用取色器 / 标尺启动工具 |
| 悬停框选到了错误的（下层）窗口 | 自定义检测脚本缺少 z 序数据；没有 `zOrder` 的窗口按最底层处理 |
| 截图启动变慢 | 检测脚本在覆盖层之前运行；只有桌面响应慢才需要调大 `timeoutMs`，或设 `enabled:false` 跳过 |
| 查看诊断 | 运行 `mark-shot --debug --debug-log /tmp/mark-shot.log`，查找 `window-detection` 日志行 |

---

## 3. 区域选择与启动工具

提交区域之前可使用以下启动工具：

| 快捷键 | 工具 | 行为 |
| :---: | :--- | :--- |
| `C` | 取色器 | 采样像素；滚轮缩放放大镜；左键打开颜色面板（HEX / RGB / HSL / HSV / Qt 格式）；右键或 `Esc` 退出 |
| `R` | 标尺 | 悬停读取像素坐标；左键拖动测量矩形（宽、高、对角线、面积）；右键或 `Esc` 退出 |
| `Q` | 二维码扫描 | 圈选二维码 / 条形码区域；解码结果在可复制窗口中打开 |
| `D` | 显示器快照 | 捕获全部输出、按显示器裁剪并显示可悬停缩略图（复制 / 编辑 / 保存） |
| `S` | 停止录制 | 停止覆盖层中显示的 GIF / 视频录制 |

`Esc` 取消会话；右键（无启动工具时）同样取消。

---

## 4. 标注工具

选中区域（或打开本地图片）后进入编辑器，显示标注工具栏。工具可用数字键或
工具栏切换：

| 快捷键 | 工具 | 说明 |
| :---: | :--- | :--- |
| `V` | 移动 / 平移 | 移动整个选区；本地图片模式下平移画布 |
| `S` | 选择 | 选择、移动、缩放、旋转、删除已有标注 |
| `P` | 画笔 | 平滑手绘笔迹 |
| `L` | 直线 | 直线 |
| `H` | 荧光笔 | 半透明标记；支持手绘或直线样式 |
| `R` | 矩形 | 支持 `描边` / `高亮` / `反色` 三种样式与圆角 |
| `E` | 椭圆 | 椭圆 / 正圆 |
| `A` | 箭头 | 经典箭头（实心、KDE、双向） |
| `T` | 文字 | 富文本；滚轮或滑块调节大小；对角手柄等比缩放、边侧手柄调节换行宽度；字体面板支持精确字号、字体族、粗体 / 斜体 |
| `N` | 序号 | 自动递增的编号标记（阿拉伯、字母、罗马、中文等） |
| `M` | 马赛克 | 亚克力磨砂模糊，用于遮挡敏感信息 |
| `G` | 激光 | 自动消散的临时笔迹 |

绘制技巧：

- 绘制矩形 / 椭圆时按住 `Ctrl` 约束为正圆 / 正方形。
- 工具激活时滚动滚轮可动态调节描边宽度、文字大小、编号缩放或马赛克块大小
  （实时预览）。
- `选择` 工具下滚动滚轮缩放画布，按住中键平移；双击 `Ctrl` 重置。

### 4.1 编辑已有标注

切换到**选择**（`S`）。点击标注显示控制手柄：

- 内部拖动：移动；
- 拖动角 / 边手柄：缩放；
- 拖动上边缘外侧的圆形手柄：旋转；
- 按 `Delete` / `Backspace`：删除；
- 双击文字：就地编辑。

右侧属性面板编辑选中标注：颜色、宽度、样式、文字字体 / 字号 / 粗体 / 斜体。
`选择` 工具下拖出选框可多选，多选后可整体移动、缩放、旋转、删除。

### 4.2 动作

| 快捷键 | 动作 |
| :--- | :--- |
| `Ctrl+C` | 复制到剪贴板 |
| `Ctrl+S` / `Enter` | 保存（路径模板来自设置） |
| `Ctrl+P` | 钉住为悬浮贴纸窗口 |
| `Ctrl+U` | 上传到配置的图床；返回的 URL 自动复制 |
| `Ctrl+Z` / `Ctrl+Y` | 撤销 / 重做 |
| `F` | 切换捕获范围（选区 ↔ 全屏） |

### 4.3 导出相框

开启 **设置 → 导出 → 苹果风格相框** 后，保存 / 复制 / 上传的图片会带上透明
内边距、圆角与柔和阴影。

---

## 5. 钉住的贴纸窗口

| 手势 / 快捷键 | 行为 |
| :--- | :--- |
| 左键拖动 | 移动贴纸 |
| 滚轮 | 等比缩放 |
| 双击左键 / `Esc` | 关闭 |
| 右键 | 上下文菜单（旋转、缩放、置顶、复制文字、翻译、保存、复制、关闭） |

贴纸窗口内的 OCR 文字可直接选择复制（`Ctrl+C` / 右键菜单）。翻译
（OpenAI 兼容接口）会把译文按原布局位置渲染回图片上。

---

## 6. 长截图（滚动截图）

1. 选择区域（超大区域会显示浮动拖拽手柄）。
2. 覆盖层滚动目标窗口，捕获的帧被拼接为长图。
3. GNOME Wayland 需要 Mark Shot Scroll Helper 扩展（§ 2.1）。

滚动截图在 niri 及类似的 wlroots/Wayland 合成器上已可稳定使用；在 KDE、X11
等环境属于测试特性。失败时可改用普通截图或自定义扩展命令。

---

## 7. Headless 截图（CLI）

非交互截图会写入 PNG 并输出 JSON：

```bash
# 主屏
mark-shot --capture-to /tmp/shot.png

# 目录（自动生成时间戳文件名）
mark-shot --capture-to /tmp/shots/

# 区域
mark-shot --capture-to /tmp/r.png --region 0,0,1280,720

# 指定显示器，包含鼠标
mark-shot --capture-to /tmp/w.png --display DP-1 --include-cursor

# 一次捕获多个显示器（每个一张 PNG）
mark-shot --capture-to /tmp/shots/ --display DP-1 --display DP-2

# 列出输出
mark-shot --list-displays
```

所有 headless 选项与位置参数（图片文件）互斥。完整参数表见 README。

---

## 8. 桌面快捷键与托盘

托盘模式（`mark-shot --tray`）默认注册 `Ctrl+Alt+S` 区域截图，并提供捕获 /
录制 / 设置 / 退出菜单。桌面快捷键配置：

- **GNOME**：设置 → 键盘 → 快捷键 → 自定义快捷键，绑定到 `mark-shot`。
- **KDE**：自定义快捷键绑定 `mark-shot`（精确 KDE 捕获还需 KWin ScreenShot2
  权限，见 README）。
- **Hyprland**：`bind = SUPER SHIFT, S, exec, mark-shot` 与
  `bind = , Print, exec, mark-shot`。
- **niri**：`binds { Mod+Shift+S { spawn "mark-shot"; } }`。
- **Sway / i3**：`bindsym Mod4+Shift+S exec mark-shot`。

---

## 9. 配置与后端

- 配置文件：`~/.config/mark-shot/config.json`（Linux），首次运行自动创建。
- 完整参考：[配置文档](configuration.zh-CN.md)。
- 后端：Wayland（PipeWire portal / grim / wlroots screencopy）、X11
  （`QScreen::grabWindow`）、Windows（原生 WGC）。录制优先使用 PipeWire
  portal，失败时自动回退。

可选辅助：

```bash
# OCR（RapidOCR / Tesseract）
python3 -m venv ~/.local/share/mark-shot/ocr-venv
~/.local/share/mark-shot/ocr-venv/bin/pip install -U pip rapidocr onnxruntime

# 二维码扫描（zxing-cpp）
python3 -m venv ~/.local/share/mark-shot/code-scan-venv
~/.local/share/mark-shot/code-scan-venv/bin/pip install -U pip zxing-cpp pillow
```

---

## 10. 功能自测清单

按以下步骤端到端验证一个构建：

1. **启动** — `run-mark-shot.sh` 打开冻结覆盖层。
2. **窗口悬停** — 鼠标移到窗口上：青色框跟随；单击选中窗口；重叠窗口选中
   最顶层。
3. **手动框选** — 拖动矩形；松开进入编辑器。
4. **标注** — 逐个试用工具（画笔、直线、矩形、椭圆、箭头、荧光笔、文字、序号、
   马赛克、放大镜、激光）；撤销 / 重做；选择工具移动 / 缩放 / 旋转 / 删除；
   双击文字编辑。
5. **复制 / 保存 / 钉住 / 上传** — `Ctrl+C`、`Ctrl+S`、`Ctrl+P`、`Ctrl+U`。
6. **启动工具** — `C` 取色器、`R` 标尺、`Q` 扫码、`D` 显示器快照。
7. **Headless** — `--capture-to`、`--region`、`--display`、`--list-displays`。
8. **托盘与快捷键** — `mark-shot --tray`，按 `Ctrl+Alt+S`。
9. **便携版细节** — 包内自带 Qt 库 / 插件 / 脚本可被找到。

---

## 11. 反馈

使用内置的[问题提交指南](../../.doc/submit-issue-via-gh.md)，通过 `gh issue
create` 提交问题，并附上 `mark-shot --debug --debug-log /tmp/mark-shot.log`
抓取的调试日志。
