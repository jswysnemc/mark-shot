# Mark Shot

面向 Linux Wayland 的 Rust 截图与标注应用。界面直接使用 Wayland 共享内存表面、
tiny-skia 和 cosmic-text 绘制，不依赖 Qt、GTK、浏览器或 WebView。

本次重写参考 `/home/snemc/workspace/LumineCapture` 的原生架构，界面与操作以 Mark Shot Qt
实现为依据。截图、编辑、贴图、OCR 和滚动截图基础工作流已实现，完整界面一致性仍在补齐。
二维码扫描和录像不在本次范围内。
完整功能对照与当前验证状态见 `docs/rewrite-status.md`。
OCR 原文和已开启编辑的表格单元格支持原生 Wayland 输入法，候选与复制、历史及表格
数据隔离。Qt 状态对照、已验证范围与剩余差异见 `docs/window-input-method-compatibility.md`。

## 构建

需要 Rust 1.95 或更新版本，以及 Wayland、xkbcommon、PipeWire、Fontconfig 开发库。文字系统匹配使用
Fontconfig 提供的 `fc-match`；缺少该工具时使用已加载字体回退，不能保证与原版字体一致。OCR 依赖的
最低编译器版本为 1.95，当前使用 Rust 1.98 验证。

```sh
cargo build --release --locked
cargo test --workspace --locked
./target/release/mark-shot --help
```

Arch Linux 构建依赖：

```sh
sudo pacman -S --needed rust pkgconf wayland libxkbcommon pipewire fontconfig
```

Debian / Ubuntu 构建依赖：

```sh
sudo apt install cargo rustc pkg-config libwayland-dev libxkbcommon-dev libpipewire-0.3-dev libssl-dev libfontconfig-dev fontconfig
```

发行版自带 Rust 版本不足时，应使用 rustup 提供的稳定工具链。

## 工具栏与贴图

选区完成后显示原版分组主栏与纵向动作副栏，全图编辑将副栏动作合并到主栏。
工具栏使用 Mark Shot SVG、握柄、细分隔线及原版暗色配色，支持左键拖动握柄与窄屏换行。
右键不执行工具栏按钮；选择阶段右键取消，编辑阶段右键取消草稿并切回标注选择，
双击右键打开颜色面板。

贴图右键提供原生菜单，可旋转、缩放、重置大小、切换置顶、编辑、复制、识别文字、
另存为和关闭。左键拖动，双击左键关闭，滚轮只改变显示大小；复制和保存保留原图像素。
八方向边缘缩放保持比例与对边；贴住屏幕的边改为移动，便于恢复图片。默认置顶模式支持
同一次手势跨输出拖动，超出屏幕只裁切可见表面，不压缩原图。滚轮缩放保持指针锚点，
普通窗口使用合成器的系统移动和缩放。
普通贴图不初始化 OCR；首次文字识别显示模型下载引导，下载需要用户主动点击。

默认贴图使用两像素青色边框，相关选项位于 `[pinned]`。翻译、上传、扩展、打开方式和完整
设置入口尚未实现；全部差异及交互细节见 `docs/interface-compatibility.md`。

## 截图历史

`mark-shot --history` 打开独立原生历史窗口。截图界面中按 `Ctrl+H` 也可打开；未完成的
绘图或拖动手势会阻止交接，窗口首帧就绪后才关闭截图界面。取消截图不写入历史。

默认记录完成图片，保留最近 50 项，数量范围 1–200，总量默认 256 MiB；超过数量或
总量时删除最旧项。历史窗口提供缩略图、原图预览、日期与尺寸搜索、复制、贴图、另存为、
重新编辑、删除和带确认的清空。复制、贴图及另存为不重复记录，重新编辑完成后新增一项。

```toml
[history]
enabled = true
limit = 50
max_size_mib = 256
index_annotations = false
```

默认目录为 `~/.local/share/mark-shot/capture-history`，可通过 `directory` 指定其他路径。
首次禁用记录时，捕获和打开空历史窗口都不会创建历史目录。窗口可以修改记录开关及保留
数量；修改数量立即清理，配置更新保留其他字段、未知字段和注释，并遵循 `--config`。

附加标注文字索引默认关闭。显式启用 `index_annotations` 后，PNG 元数据可能包含最终
画面中已经裁掉或遮盖的原始标注文字；关闭索引不会删除旧记录中的文字。日期、尺寸及
文件名搜索不依赖此开关。存储和隐私边界见 `docs/history-compatibility.md`。

## 标记与测量

编辑器中按 `K` 使用二十四种形状标记，按 `U` 使用长度或角度尺，按 `Z` 使用持久双框
放大镜。标记支持形状、填充与独立笔宽；尺子显示原图宽、高、对角线、面积或夹角，
支持定点、移动、调整和旋转。再次按 `U` 切换下一次测量模式，已有测量保持不变。

双框放大镜支持圆形与矩形，默认 2.75 倍，范围 1.25–6。源框和镜框各自移动，任一框
缩放后按固定倍率联动。放大镜只取样冻结原图，不复制旧标注；保存和复制保留原图分辨率
与透明度。Ctrl 保持矩形长宽比，Shift 将尺子边或旋转吸附到四十五度倍数。

全屏放大镜入口：

```sh
./target/release/mark-shot --fullscreen --fullscreen-default-tool magnifier
```

`Shift+M` 控制光标取色放大镜，`Z` 创建持久标注，两者相互独立。其他原版快捷键和高级编辑
功能仍在补齐，完整差异见 `docs/annotation-compatibility.md`。

## 文字标注

文字使用原版二十点默认字号、半粗体字重与透明背景。字号属性范围为 8–300 点，
Linux 标准 96 DPI 下二十点对应 27 像素；预览比例与原始点字号分别保存。
正文和背景透明度互不影响，透明背景也保留左右六、上下四个原图像素的布局内边距。
文字字体按系统匹配结果定向加载，缺字片段使用系统回退，不替换其他界面的内置字体。

修改选中文字字号会同步工具默认字号，并按比例缩放已经换行的框宽；家族、粗体、斜体和背景不覆盖相应默认值。
撤销只恢复标注，不回退默认字号。滑条每次实际变化分别记录撤销，画布连续滚轮用 350 毫秒防抖合并。
文字属性栏采用原版 438×44 布局；选择工具下编辑已有文字时显示编辑按钮，宽度为 476。
字体弹层支持搜索、即时字号、粗体、斜体和字体列表，首次打开才枚举选择器目录，可见行预览使用独立有界缓存。
文字编辑使用独立预览；Esc、Ctrl+Enter 或外部点击提交，Enter 换行。提交前不修改持久正文；
空新文字不占标注编号或历史，清空已有文字后提交删除，重新编辑保留标注身份和当前工具。
Ctrl+Z、Ctrl+Y 与 Ctrl+Shift+Z 使用编辑器局部历史。字符拖选不移动标注；
文字区右键提供撤销、重做、剪切、复制、粘贴、删除与全选，删除只针对字符选区。
菜单内 Esc 只关闭菜单，外部首次点击只关闭菜单；菜单可见时快捷键与滚轮不穿透到画布。
菜单使用真实 Wayland 弹出表面和输入抓取，兼容普通窗口与层表面；失去激活只关闭菜单，
保留文字编辑事务与字符选区。菜单操作先关闭原生角色，再修改正文。
保存、另存为、贴图，以及无字符选区的 Ctrl+C 先提交文字；有字符选区的 Ctrl+C 复制文字。
此前 Rust 版本保存的像素字号会转换为点字号，新状态明确记录 `font_size_unit = "pt"`。
截图正文已接入 Wayland 原生输入法，候选与正文分离，逐次确认独立撤销，保存不包含未确认候选。
截图编辑器的字体搜索、点字号、十六进制颜色与属性步进字段已接入原生输入法；
原版状态与真实投递见 `docs/text-field-input-method-compatibility.md`。
实际 Qt 对照仍存在字形、抗锯齿、命名实例预览、最小字号宽度和部分换行差异；字段完整交互、
独立窗口输入法及完整角点缩放仍需补齐，不能把当前文字界面视为原版完整复刻。
详细几何、目录规则和剩余差异见 `docs/text-property-compatibility.md`。
事务与字符菜单规则见 `docs/text-editing-compatibility.md`，同构建验证记录见
`target/verification-text-menu/result.json`。
原生输入法范围与当前构建验证见 `docs/text-input-method-compatibility.md` 和
`target/verification-text-input-native/result.json`。

## 原图滤镜

编辑器中按 `M` 或 `X` 创建矩形滤镜区域，提供像素化、模糊、灰度、反色和提亮五种模式。
默认像素化，强度为 14，范围 4–48；像素化网格与模糊半径按原图像素计算，不随预览或
导出比例变化。灰度与反色不受强度影响，提亮按强度比例调整。

滤镜只读取冻结原图，不处理先前标注。覆盖层按标注透明度叠加，半透明区域可能变得更
不透明；未修改区域保持原始颜色和透明度。像素化直接平均预乘通道，不再次乘透明度。

按 `V` 选择已有区域后，可以移动、使用八个控制点缩放或旋转。Ctrl 保持已有区域比例，
Shift 将旋转吸附到四十五度倍数，创建时不强制正方形。模式、强度、透明度及几何操作
支持撤销与重做；修改已有模式不覆盖后续默认模式，修改已有强度同步工具默认强度。

```sh
./target/release/mark-shot --fullscreen --fullscreen-default-tool redact
```

配置使用 `[tools.redact]` 的 `mode` 和 `strength`。模式值为 `pixelate`、`blur`、
`grayscale`、`invert`、`brighten`；旧像素化开关仍可读取，新模式优先。按原图区域
按需处理，单项像素缓存最多 4 MiB，总量最多 16 MiB，不提前模糊整个工作区。

## 箭头与曲线

编辑器中按 `A` 创建箭头，按 `D` 创建普通直线。箭头提供实心、KDE 开口及对应双向头部，
箭身和普通直线分别提供实线、虚线、点线。头部始终连续，三类样式偏好分别保存。
默认笔宽为 3，范围 1–72，不与画笔或旧编号箭头共用默认值。

按 `V` 选择标注后，双击路径插入控制点，拖动控制点形成曲线。单控制点使用二次曲线，
多控制点形成连续平滑路径。端点、正文外控制点、外框和旋转把手分别命中；Ctrl 保持
外框缩放比例，Shift 将创建方向或旋转吸附到四十五度倍数。

Delete 优先删除当前控制点，没有控制点选择时删除整条标注。几何、样式和笔宽操作均
支持撤销；撤销清除临时定点选择。修改已有标注的头部或线型不覆盖后续默认样式，弹层
Enter 只确认，不触发截图导出。

```sh
./target/release/mark-shot --fullscreen --fullscreen-default-tool arrow
```

## 独立序号

编辑器中按 `N` 使用独立序号，单击创建圆框，拖动创建指向尾部。支持阿拉伯数字、大小写
字母、大小写罗马数字、天干和中文，共七种样式。字母在 Z 后继续 AA；天干每十项循环。
中文与天干使用随应用提供的小型字体子集，不等待系统字体扫描。

序号默认笔宽为 3，范围 1–72，与其他工具分别保存。删除旧序号不填补空缺，取消草稿
不消耗编号；重置保留已有标注，撤销和重做同时恢复标注与下一编号。新会话从一开始，
样式和笔宽保持上次偏好。圆心、尖端、缩放与旋转可独立调整，Ctrl 保持比例，Shift
吸附四十五度旋转；保存和复制保留原图分辨率及透明度。

```sh
./target/release/mark-shot --fullscreen --fullscreen-default-tool number
```

原有编号箭头保留 `Shift+N` 和 `numerated-arrow` 入口，不与独立序号共用计数。

## 激光笔

全屏激光入口：

```sh
./target/release/mark-shot --fullscreen --fullscreen-default-tool laser
```

编辑器中按 `G` 选择激光，按 `I` 选择取色器。激光支持颜色和独立笔宽，松开鼠标后在
1.8 秒内消散；切换工具取消未提交草稿，完成笔迹继续消散。临时笔迹不计入撤销历史，
也不进入保存或复制图像。全部笔迹消失后停止定时重绘。

独立运行器在不可见的 Xvfb 中创建私有 niri 会话，不操作当前桌面：

```sh
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-laser.py
```

## OCR

OCR 内置 PP-OCRv5 推理，不执行 Python 或外部 OCR 程序。默认按需初始化，普通截图
不会加载识别模型。首次选择 OCR 时显示模型下载引导；下载需要用户主动点击，显示
体积与进度，支持取消、断点续传和 SHA-256 校验。下载完成后可以离线使用。

`mark-shot --ocr IMAGE` 默认打开独立的可编辑结果窗口。文字与原图共用内容区域，切换
页面保留编辑、撤销与滚动位置。识别框形成多行多列时显示表格页，支持单元格编辑和
TSV、CSV、Markdown、HTML 复制。`Ctrl+C` 复制选中文字，`Ctrl+Shift+C` 复制全文；
表格页使用后一个快捷键复制整张表格的 TSV。

结果窗口不加载模型。截图进程收到结果窗口首帧确认后退出，关闭结果窗口不会丢失已复制
内容。将 `[ocr]` 中的 `result_panel` 设置为 `false`，可以保留截图内的逐字选择流程；
识别完成后的文字按钮仍可打开结果窗口。环境变量 `MARK_SHOT_OCR_RESULT_PANEL` 和
`MARK_SHOT_OCR_RESULT_WINDOW` 可以覆盖此开关。

OCR 可以只显示交互结果，不依赖常规截图的保存或复制输出配置。

配置保存在 `~/.config/mark-shot/config.toml`，模型保存在
`~/.local/share/mark-shot/models`，应用状态保存在 `~/.local/state/mark-shot`。

隔离 Wayland 回归测试使用已有的本地模型，不重新下载：

```sh
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-ocr-result.py --models ~/.local/share/mark-shot/models
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-ocr-copy.py --models ~/.local/share/mark-shot/models --first-use --rapid
```

## 隔离验证

验证脚本需要 niri、Xvfb、niri-use、xdotool、wtype、grim 和 Pillow；滤镜独立参考还使用
NumPy。这些工具不属于应用运行时依赖。
独立运行器仅在私有 Xvfb 中使用固定键码，避免连续会话下临时虚拟键盘漏发映射；应用
仍只连接私有 Wayland，不提供 X11 后端。标记、尺子、双框放大镜及独立序号分别使用
`scripts/wayland-marker.py`、`scripts/wayland-ruler.py`、`scripts/wayland-magnifier.py`、
`scripts/wayland-number.py`；箭头、线型和控制点使用 `scripts/wayland-arrow.py`，原图滤镜
使用 `scripts/wayland-filter.py`。截图历史完整流程使用 `scripts/wayland-history.py`，首次
禁用、忙碌快捷键、长图重编辑及目录导航使用 `scripts/wayland-history-edge-cases.py`。这些脚本
均应通过 `scripts/wayland_test_runtime.py` 独立运行，不并发复用默认私有目录。

文字真实输入使用 `scripts/wayland-text.py`，属性栏和字体弹层使用 `scripts/wayland-text-properties.py`。
原生字符菜单使用 `scripts/wayland-text-menu.py --require-native`，`--overlay layer` 与
`--overlay window` 分别验证层表面和普通窗口；普通窗口场景还验证失活关闭。
运行器的 `--scale 0.75` 用于小比例回归。私有运行目录应保持简短，避免超过 Unix 套接字路径限制。
原版文字探针对照只用于开发验证，需要 Qt6 开发包，
不影响应用运行依赖：

```sh
cmake -S tests/fixtures/qt-text-reference -B target/qt-text-reference -G Ninja
cmake --build target/qt-text-reference
uv run --with pillow python scripts/text-annotation-reference.py --artifacts target/text-reference-verification
uv run --with pillow python scripts/wayland_test_runtime.py --root target/text-runtime -- \
  uv run --with pillow python scripts/wayland-text.py --artifacts target/text-input-verification
```

```sh
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py --root target/arrow-runtime -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-arrow.py
uv run --with pillow --with numpy python scripts/wayland_test_runtime.py --root target/filter-runtime -- \
  uv run --with pillow --with numpy python scripts/wayland-filter.py
uv run --with pillow --with numpy python scripts/wayland_test_runtime.py --root target/history-runtime -- \
  uv run --with pillow --with numpy python scripts/wayland-history.py --artifacts target/history-verification
uv run --with pillow --with numpy python scripts/wayland_test_runtime.py --root target/history-edges-runtime -- \
  uv run --with pillow --with numpy python scripts/wayland-history-edge-cases.py --artifacts target/history-edges-verification
```

历史脚本要求证据目录尚不存在，重新验证时使用新的目录，不覆盖旧构建证据。

双工具栏与贴图菜单使用原生虚拟指针、虚拟键盘，避免小贴图失去焦点：

```sh
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py --root target/toolbar-runtime -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-toolbar.py --artifacts target/toolbar-verification
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py --root target/pin-runtime -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-ui-parity.py --artifacts target/pin-verification --models ~/.local/share/mark-shot/models
```

这两项同样要求新证据目录；贴图回归将保存目录隔离到证据目录，已有模型仅复制到私有会话。

八方向贴图缩放、裁切和连续跨输出使用独立 headless Sway，另外需要 Sway 和 wlroots：

```sh
uv run --with pillow python scripts/wayland-pin-geometry.py --sway "$(command -v sway)" \
  --artifacts target/pin-geometry-verification
```

该脚本自行启动两个不同分数比例的输出，不通过 niri 运行器。运行目录采用短的私有路径，
避免 Unix 套接字路径上限；证据目录须尚不存在。虚拟键盘在发送按下前等待客户端绑定。
辅助测试可通过 `uv run --with pillow --with numpy python -m unittest discover -s scripts -p 'test_*.py'` 执行。

性能采样记录构建指纹、六次无界面捕获、首次缓冲提交和一秒空闲资源：

```sh
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-performance.py
```

首次缓冲提交不等同于合成器呈现时间；采样条件和当前数据见 `docs/rewrite-status.md`。

## 来源与许可

Wayland 原生界面、截图后端和部分编辑代码基于 LumineCapture，遵循其 MIT / Apache-2.0
双许可。相关许可与字体许可随源码保留。原版 Mark Shot 仓库作为功能参考，不修改其中
未提交的工作。
