# 使用说明

[English](../usage.md) | **简体中文**

## 截图流程

运行 `mark-shot` 后，屏幕画面冻结，并显示选区覆盖层。

1. **框选。** 拖动鼠标绘制区域。指针悬停时，程序会高亮其下方的窗口或界面区域；单击即可采用该区域，
   滚动滚轮可以在外层区域和内层区域之间切换。指针旁的放大镜显示像素颜色和坐标。
2. **调整。** 使用选区工具（`S`）拖动选区边缘或内部。方向键每次移动一个像素，`Alt` 加方向键调整大小，
   同时按住 `Shift` 则每次十个像素。
3. **标注。** 从工具栏选择工具，见[标注工具](annotation-tools.md)。
4. **完成。** 执行下文列出的任一动作。

框选阶段按 `Esc` 或单击右键会取消截图。

### 完成截图

| 动作 | 按键 | 结果 |
| --- | --- | --- |
| 确认 | `Enter` | 执行默认完成动作 |
| 复制 | `Ctrl+C`，或在选区内双击 | 把图片复制到剪贴板 |
| 保存 | `Ctrl+S` | 保存到截图目录 |
| 另存为 | `Ctrl+Alt+S` | 打开文件对话框 |
| 贴图 | `Ctrl+P` | 把图片置顶显示在其他窗口上方 |
| 取消 | `Esc` | 放弃本次截图 |

**默认完成动作**由 `[general]` 中的 `to` 决定，取值是下列字母的组合：

| 字母 | 动作 |
| --- | --- |
| `c` | 复制到剪贴板 |
| `s` | 保存到截图目录 |
| `p` | 贴到屏幕上 |
| `o` | 把 PNG 写入标准输出 |

默认值为 `cs`，即复制并保存。在编辑器中选择其他动作时，这些动作也会一并执行，例如设置
`to = "s"` 后，复制的同时会保存文件。`-t 字母` 仅对本次运行覆盖该设置。

文件保存到 `~/Pictures/screenshots/<年-月>/<日期_时间>.png`。目录和文件名格式可以在设置窗口或
`[save]` 中修改；`--output 目录` 仅对本次运行指定目录。

### 截图方式

| 命令 | 行为 |
| --- | --- |
| `mark-shot` | 框选区域后进入编辑 |
| `mark-shot -m` | 只冻结指针所在的显示器 |
| `mark-shot --all-outputs` | 冻结整个桌面 |
| `mark-shot --display 名称` | 使用指定的显示器，重复该选项可以指定多个 |
| `mark-shot --fullscreen` | 打开编辑器并选中整个画面 |
| `mark-shot -f` | 不打开编辑器，立即截图 |
| `mark-shot -r` | 拖动选区，松开鼠标后立即完成 |
| `mark-shot -w` | 截取活动窗口 |
| `mark-shot -d 3` | 等待三秒后截图，取值 0 到 60，可以使用小数 |
| `mark-shot picture.png` | 在编辑器中打开图片文件 |

未指定范围选项时，程序冻结全部显示器。在 `[capture]` 中设置 `freeze_scope = "cursor-screen"`
后，默认只冻结指针所在的显示器。

活动窗口截图支持 X11、KDE Wayland、niri 和 Windows。在 niri 上，完成动作必须包含复制，例如
`mark-shot -w -t c`。`mark-shot --list-displays` 输出各显示器的名称。

在 Linux 上，已有截图界面时再次启动 Mark Shot，新请求会转交给正在运行的实例，不会打开第二个覆盖层。

### 滚动长截图

运行 `mark-shot --scroll`，或在框选区域后按 `Ctrl+Shift+S`。以平稳的速度滚动选区内的内容，
Mark Shot 持续获取画面并拼接为一张长图。固定的页头和页尾只保留一次。相邻两帧重叠不足时，
界面会提示缓慢回退，以便补充采集。

## 贴图

贴图是一个保持在其他窗口上方的小窗口。

| 来源 | 方法 |
| --- | --- |
| 编辑器 | 按 `Ctrl+P` 或点击贴图按钮 |
| 图片文件 | `mark-shot --pin-image 文件` |
| 剪贴板 | `mark-shot --pin-clipboard`，支持图片和纯文本 |

| 操作 | 结果 |
| --- | --- |
| 左键拖动 | 移动 |
| 拖动边缘或角 | 等比缩放 |
| 滚轮 | 以指针位置为中心缩放 |
| 双击 | 关闭 |
| 右键 | 打开菜单：旋转、缩放、重置大小、置顶、编辑、复制、识别文字、翻译、扫码、另存为、关闭 |
| `Ctrl+V` | 把剪贴板内容贴为另一个窗口 |

无论显示大小如何，复制和保存始终使用原始像素。识别文字后，在文字上拖动可以选择文字，
`Ctrl+C` 复制所选内容。边框和默认置顶选项位于 `[pinned]`。

## 截图历史

完成的截图会自动记录。运行 `mark-shot --history` 或在编辑器中按 `Ctrl+H` 打开历史窗口。
窗口显示缩略图和预览，并提供复制、贴图、另存为、重新编辑、删除和清空。

默认保留最近 50 张截图，总量不超过 256 MiB，超出时先删除最旧的记录。已取消的截图不会记录。
相关设置位于 `[history]`，见[配置](configuration.md#截图历史)。

## 二维码与条形码

```sh
mark-shot --scan               # 框选屏幕区域
mark-shot --scan image.png     # 识别图片文件
mark-shot --scan-json image.png
```

识别结果显示在独立窗口中，文字可以选择和复制。`--scan-json` 不打开窗口，直接输出由
`{ "format", "text" }` 对象组成的 JSON 数组；未识别到内容时输出 `[]`。编辑器工具栏和贴图菜单中也有扫码按钮。
识别在本地完成，不需要下载模型。

## 上传与外部程序

- **上传**把标注后的 PNG 发送到图床，并复制返回的地址。程序没有默认图床，使用前需要先配置，
  见[配置](configuration.md#上传)。
- **打开方式**列出已安装的桌面应用，并把图片交给所选应用。

两个按钮都位于动作工具栏。

## 脚本化截图

`--capture-to` 不显示任何界面，直接截图并输出一份紧凑的 JSON 报告：

```sh
mark-shot --capture-to ~/shot.png
mark-shot --capture-to ~/shots --output-name build-42
mark-shot --capture-to ~/shot.png --region 100,100,800,600
mark-shot --capture-to ~/shot.png --display DP-1 --include-cursor
```

`--region` 使用全局逻辑坐标，格式为 `X,Y,宽,高`。

## 命令行参考

```text
mark-shot [选项] [图片]
```

**截图与编辑**

| 选项 | 含义 |
| --- | --- |
| `--all-outputs` | 冻结整个桌面 |
| `-m`、`--monitor` | 只冻结指针所在的显示器 |
| `--fullscreen` | 打开编辑器并选中整个画面 |
| `-f`、`--full` | 不打开编辑器，立即截图 |
| `-w`、`--window` | 截取活动窗口 |
| `-r`、`--select-region` | 拖动选区并立即完成 |
| `--ocr` | 框选区域并识别文字 |
| `-S`、`--scroll` | 框选区域并拼接滚动长截图 |
| `-d`、`--delay 秒数` | 延时截图 |
| `--display 名称` | 指定显示器，可重复 |
| `--include-cursor` | 在后端支持时把鼠标指针截入图片 |
| `--default-tool 工具` | 框选完成后的初始工具 |
| `--fullscreen-default-tool 工具` | `--fullscreen` 的初始工具 |
| `--file-default-tool 工具` | 打开图片文件时的初始工具 |
| `--default-color 颜色` | `#RRGGBB` 或 `#RRGGBBAA` |
| `--xdg-window` | 使用普通窗口，不使用覆盖层 |
| `-t`、`--to 字母` | 完成动作：`c`、`s`、`p`、`o` |
| `--output 目录` | 保存到该目录 |

工具名称：`selection`、`pick`、`pen`、`highlighter`、`laser`、`marker`、`ruler`、`magnifier`、
`line`、`arrow`、`rectangle`、`ellipse`、`text`、`number`、`numerated-arrow`、`redact`、`ocr`、
`eyedropper`。

**脚本化截图**

| 选项 | 含义 |
| --- | --- |
| `--capture-to 文件或目录` | 写入 PNG 并输出 JSON 报告 |
| `--region X,Y,宽,高` | 全局逻辑区域，需要与 `--capture-to` 一起使用 |
| `--output-name 名称` | 写入目录时使用的文件名 |
| `--list-displays` | 以 JSON 输出显示器信息 |

**其他窗口**

| 选项 | 含义 |
| --- | --- |
| `--pin-image 文件` | 把图片文件贴到屏幕上 |
| `--pin-clipboard` | 把剪贴板中的图片或纯文本贴到屏幕上 |
| `--scan`、`--scan 图片`、`--scan-json 图片` | 识别二维码与条形码 |
| `--history` | 打开截图历史 |
| `--settings` | 打开设置 |
| `--record`、`--record-select` | 打开录制控制窗口，或直接选择录制区域 |
| `--recording-status`、`--pause-recording`、`--resume-recording`、`--stop-recording` | 控制正在进行的录制 |

**配置与诊断**

| 选项 | 含义 |
| --- | --- |
| `--background` | 运行托盘和全局快捷键服务，直到退出 |
| `--config 文件` | 使用另一个配置文件 |
| `--print-default-config` | 输出完整的默认配置 |
| `--debug`、`--no-debug` | 开启或关闭诊断日志 |
| `--debug-log 文件` | 把诊断信息写入指定文件 |
| `-s`、`--speed` | 把各启动阶段的耗时输出到标准错误 |
| `-n`、`--nonotify` | 不发送桌面通知 |
| `-h`、`--help`、`-v`、`--version` | 显示帮助或版本 |
