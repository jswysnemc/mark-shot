# OCR 结果窗口独立置顶

本文件记录本地 Rust 重构的独立置顶实现。标题栏置顶与原图菜单的贴图不是同一项
操作：原图贴图创建独立图像进程；标题栏置顶只改变当前 OCR 结果窗口的协议角色。
开发和验证只使用私有合成器及本地固定文档，不修改用户配置，不提交、不推送、不发版。

## 原版依据

`tests/fixtures/qt-ocr-stacking-reference/main.cpp` 只读连接完整原版 `OcrResultWindow`，
使用实际按钮及配置保存接口记录状态。原版资源 `:/icons/ui/pin.svg` 直接导出为
`assets/icons/mark-shot/pin.svg`，不使用其他项目的贴图图标代替。

原版离屏采集保存在 `target/qt-ocr-stacking-r3/`，包含五个真实状态：默认、开启、关闭、
保存失败和反馈到期。已确认：

- 默认关闭；即使贴图的 `pinnedWindow.alwaysOnTop` 为 true，OCR 窗口仍关闭置顶。
- 独立保存到 `ocrResultWindow.alwaysOnTop`，不改变贴图开关。
- 760 像素宽窗口的按钮为 `[694,6,26,26]`，图标使用 16 像素逻辑尺寸。
- 常态和选中态沿用原版 quiet 按钮，选中图标使用强调色。
- 保存失败回退按钮状态，页脚显示 `Failed to save settings`，2500 毫秒后清除。

原版 `ocr_result_window_placement.cpp` 使用 layer-shell 实现支持环境中的真正置顶，
改变角色时保留当前逻辑几何及内部控件。离屏采集不证明实际 Wayland 角色或像素一致；
协议效果由原生运行时场景分别验证。

## 配置边界

原生配置使用：

```toml
[ocr_result_window]
always_on_top = false
```

独立读取和保存也支持原版 JSON 的 `ocrResultWindow.alwaysOnTop`。JSON 保存仍写入 JSON，
不把旧文件替换为 TOML。更新使用已有共享进程锁和原子替换，保留未知字段、厂商参数、
贴图开关和目标语言。TOML 保留既有兼容键及行尾注释。

非法结构、链接、硬链接、非普通文件和超大文件均不能写入。保存成功后才改变按钮状态
和原生角色请求；保存失败不能先切换角色再伪装为成功。

## 文件职责

| 文件 | 职责 |
| --- | --- |
| `src/config/ocr_result_window.rs` | 独立开关、两种格式读取及原子保存 |
| `src/config/tests/ocr_result_window.rs` | 默认、兼容键、字段保持和失败边界 |
| `src/backend/wayland/window/role.rs` | 普通 xdg 与 layer-shell 角色接口 |
| `src/backend/wayland/window/stacking.rs` | 主角色切换、输出选择和浮层事务 |
| `src/backend/wayland/window/layer_geometry.rs` | 输出本地边距、居中及连续指针几何 |
| `src/features/ocr_result/stacking.rs` | 按钮交付、保存回退及单次反馈期限 |
| `scripts/wayland-ocr-stacking.py` | 实际输入、角色往返、首次启动和资源回收 |
| `scripts/wayland_ocr_stacking_checks.py` | 创建、配置、绘制、焦点及层级证据 |

现有表面、标题栏、输入、弹层和对话框模块只接入这些接口。普通窗口继续使用原有
xdg-toplevel；不把全部窗口无条件改成浮层。

## 原生角色行为

支持 layer-shell 时，置顶窗口使用 `Layer::Top`、按需键盘输入及非独占区域。
角色切换重建协议表面、视区、分数缩放和有界缓冲，但不析构业务对象，因此正文、
撤销历史、语言、翻译句柄和唤醒通道继续使用同一实例。

旧表面、旧输入焦点及按钮抓取先失效；新角色重新取得输入上下文。过期输入法批次
不能修改新角色中的正文。逻辑首帧通知只发送一次，不因角色重建重复发送 ready。
分数缩放事件还需要匹配当前协议对象，旧角色事件不能只凭主窗口编号修改新表面。

浮层通过输出本地边距定位，使用原版包含右下边界的整数居中规则。标题拖动和页脚
尺寸调整使用固定全局按下起点，连续移动不能重复累加同一位移。尺寸调整保留最小
尺寸，指针形状分别使用抓取和右下尺寸调整。

浮层菜单使用无 xdg 父面的 popup，并通过当前 layer-surface 挂接；子菜单继续使用
存活 xdg 弹层父链。它们不混合不同角色的父面。

模态请求接口已接入临时降低浮层、隔离输入及关闭后恢复层级。OCR 当前未提供完整
设置对话框，本轮不以接口代码认定全部模态业务通过。

缺少 layer-shell 的环境禁用该入口，并显示实际普通窗口状态；保存偏好不因此遭改写。
GNOME 等环境的原版桌面扩展回退仍未移植，不能宣称所有 Wayland 桌面都支持置顶。

## 真实验证入口

```sh
uv run --no-project --with pillow --with numpy python scripts/wayland_text_sway_runtime.py \
  --root target/private-ocr-stacking -- \
  uv run --no-project --with pillow --with numpy python scripts/wayland-ocr-stacking.py \
  --binary target/release/mark-shot --root target/ocr-stacking-verification
```

使用 `scripts/wayland_test_runtime.py` 验证私有 niri；添加 `--startup-only` 验证保存为
置顶后的首次启动不临时创建普通窗口。每次必须使用新的运行与证据目录。

真实场景覆盖独立保存、右键不激活、鼠标释放及 Space 交付、浮层拖动和尺寸调整、
正文及撤销重做保持、候选菜单、过期提交拒绝、运行中翻译保持、保存失败与期限清理、
单次 ready 及关闭资源。它读取实际配置、协议对象、缓冲交付、剪贴板和请求记录，
不以按钮内部标志代替真实层级。

验收记录集中保存于 `target/ocr-stacking-local-r1/validation.json`，按构建 SHA-256
关联全部结果。失败尝试保留，不把旧构建或失败目录列入成功矩阵。

本轮优化二进制 SHA-256 为
`91432d667de2f9ad020656495abe7dc1a0a32daf6b53e6a6ef09eaf70fdf889d`。

| 验证范围 | 本轮结果 |
| --- | --- |
| 工作区 Rust 测试 | 909 项通过、10 项忽略，未额外排除测试 |
| Python 工具测试 | 197 项通过 |
| 格式、严格工作区 Clippy、优化构建 | 通过 |
| 置顶完整场景、保存置顶后直接启动 | 私有 Sway、niri 四组通过 |
| 输入法与翻译输入 | Sway 1 倍、1.2 倍及 niri 1.2 倍三组通过 |
| 既有窗口、菜单、反馈、历史回归 | 七组通过 |

置顶完整场景等待反馈到期和输入稳定后，进行 0.8 秒空闲采样。Sway 与 niri 均保持
单线程、CPU 计数不增长；常驻内存分别为 45272 KiB、35260 KiB。这只描述本次私有
场景，不代表完整性能矩阵。
运行结果、原版依据、源码指纹和私有运行器记录分别存放，避免验收记录过度集中。

## 标尺测试边界修正

此前排除的 `pick_tool_can_rotate_a_selected_ruler_using_its_outside_handle` 已定位为
测试坐标错误，不是选择工具遗漏旋转把手。测试按固定笔宽 3 计算把手，但编辑器实际
读取到笔宽 7，真实把手与测试坐标相差约 14 像素，超出命中范围。

`src/app/input/tests/ruler.rs` 现在显式覆盖笔宽 3、7、15，并按持久标注的实际笔宽计算
把手。断言分别检查几何命中、按下旋转事务、移动临时标注、释放后的选择与历史，以及
撤销恢复原标注。未修改生产命中逻辑，也未改变用户配置。

原始失败、坐标检查失败和修复通过分别保存在：

- `target/ocr-stacking-ruler-current-red-r1.log`
- `target/ocr-stacking-ruler-width-probe-r1.log`
- `target/ocr-stacking-ruler-green-r1.log`

`target/ocr-stacking-full-tests-r4.log` 使用 `cargo test --locked -j 2 --workspace`，
909 项通过、10 项忽略、0 项失败、0 项额外排除。HEAD 的旧失败记录继续保留为历史
依据，不再将它描述为当前构建仍然存在的失败。

## 尚未完成

- 原版 Qt 的完整 Wayland 置顶运行及跨渲染器逐像素对照。
- 跨输出拖动重绑定、桌面工作区边界、真实触摸设备及完整冷启动性能矩阵。
- GNOME 桌面扩展回退、全部模态入口、工具提示和完整键盘焦点遍历。
- 中文界面、亮色主题及项目其余重写项。

本轮不代表整体重写完成。工作区仍有 10 项主动忽略的测试，本轮未启用它们；未修改
HEAD 的历史失败证据保留于 `target/ocr-titlebar-ruler-head-baseline.log`。
