# 打开方式、扩展命令与光标放大区

截图工具栏的打开方式与扩展入口已接入实际列表和进程交付，不再固定禁用。
设置窗口继续按本次要求暂不完成，截图位置翻译仍属于整体重写的待完成项。
上传的后续实现和验证边界见 `docs/upload-compatibility.md`。

## 模块职责

- `src/features/external_actions/desktop_apps.rs`：图片桌面程序发现与条目过滤
- `src/features/external_actions/desktop_exec.rs`：原版双引号拆分、字段展开和可读文件 URL
- `src/features/external_actions/extensions.rs`：独立 `extensions.json` 的读取、默认值和别名
- `src/features/external_actions/launch.rs`：参数引用、私有 PNG 与独立进程启动
- `src/ui/external_panel/`：原版侧边面板、列表滚动、输入与有限图标缓存
- `src/app/external_actions.rs`：工具栏和快捷键入口、文字提交、外部进程交接与失败恢复
- `src/editor/cursor_magnifier.rs`：初始选择、调整与取色阶段判断
- `src/app/cursor_magnifier.rs`：显隐变化、旧圆环和取色说明的完整损坏范围

以上实现不增加依赖，不创建长期等待外部程序的线程。普通截图启动时不扫描应用目录，
面板关闭后释放动作和图标缓存。图标只栅格化到二十二像素，缓存最多六十四项。

## 原版规则与配置

打开方式读取用户和系统 `applications` 目录中的 Desktop Entry。仅列出
`Type=Application`、支持图片 MIME、有名称和命令、未启用 `Hidden` 或 `NoDisplay`
的条目。只读取 `[Desktop Entry]` 分组，动作分组不能覆盖主入口。
当前实现遵循 `XDG_DATA_HOME` 和 `XDG_DATA_DIRS`；原版用户数据目录固定为
`~/.local/share`，这项平台路径行为有明确差异。

桌面命令直接执行程序和参数，不经过 shell。`%f/%F`、`%u/%U`、`%c`、`%k`、`%%`
分别提供图片路径、文件 URL、应用名称、桌面文件路径和百分号。
原版不交付 `%i`，已经弃用的目录等字段移除；没有图片字段时追加 PNG 路径。
文件 URL 保留可读空格和 Unicode；百分号、井号、问号和控制字符保持编码。
替换结果不再次作为字段展开，避免文件名中的占位字符改变数据。

扩展配置位于 `$XDG_CONFIG_HOME/mark-shot/extensions.json`，不需要设置窗口。
兼容原版数组、单个命令对象和含 `commands` 数组的对象，保留配置顺序。

```json
{
  "commands": [
    {
      "name": "Open image editor",
      "command": "image-editor {image}",
      "description": "Edit the screenshot",
      "workingDirectory": "~/Pictures",
      "closeOnStart": true
    },
    {
      "name": "Inspect region",
      "command": "region-inspector {slurp}",
      "closeOnStart": false
    }
  ]
}
```

`workingDirectory` 兼容 `cwd`，`saveImage` 兼容 `needsImage`；错误类型不隐式转换。
`closeOnStart` 默认开启。出现 `{image}`、`{imagePath}` 或 `{imageUrl}` 时自动保存
图片并用引用后的单个 shell 参数替换；启用保存但没有图片字段时追加路径。
`{slurp}` 使用全局逻辑区域字符串。仅需要区域的命令不编码或写入图片。
路径中的波浪号只展开当前用户目录，不执行环境变量或 shell 替换。

两个入口默认没有快捷键，与原版默认策略一致；可单独配置：

```toml
[keys]
open_with = "F6"
extensions = "F7"
```

## 界面与交付边界

面板沿用原版八像素内边距、四像素标题间隔和左侧优先定位。
打开方式宽度为 280–340，扩展为 300–380；小输出优先限制到可见范围。
Qt 6.12.0 本机采样的列表字体为十六像素，实际行高为四十像素；列表预留高度仍按
每项三十六像素计算，上限三百六十像素。因此三项列表也有十二像素滚动范围。
非空面板高度为 85、145、397，分别对应一项、三项和二十四项。

左键在同一行按下并释放才执行动作；右键和中键不启动外部程序，不修改下方工具。
滚轮和列表按键不能穿透到画布。Escape、外部点击、重复点击入口和窗口失活关闭面板。
Home/End、方向键和翻页键可以到达全部条目；滑块支持拖动及轨道翻页。
打开面板先提交文字、关闭属性弹层，动作选择后先关闭面板再交付进程。

外部程序得到最终标注 PNG，包含当前导出外框，与用户保存、复制使用同一图像策略。
临时 PNG 使用随机名称和 `0600` 权限；启动失败自动删除，启动成功后文件独立存在，
与原版保留交付文件的生命周期一致，不能在消费方延迟读取前立即删除。
外部动作不追加截图历史，也不执行常规默认保存和复制。

需要关闭时先隐藏覆盖层并释放输入；只有实际 exec 成功后才结束截图。
启动失败重建原表面并恢复同一冻结图片、选区和标注，显示统一限时提示。
启动成功只表示程序完成 exec，不代表扩展自身业务已经执行成功。
扩展 shell 的后续退出码与原版分离启动一致，不进入截图窗口。

## 光标放大区诊断

真实私有会话的第一轮截图记录了静止图片编辑器中仍存在圆形放大区：
`target/wayland-external-actions-sway-r1/extension-before.png`。
两项最小状态测试随后复现完成选区与普通标注仍返回可见，原始失败记录位于
`target/cursor-magnifier-phase-before.log`。

根因是显示函数只检查开关与取色，没有检查选择或编辑阶段。
现在初始选择和鼠标调整支持放大区，静止区域编辑、普通标注及全图编辑不显示。
取色仍有独立职责；弹层、文字菜单与 OCR 阶段隐藏放大区。
显隐切换恢复旧圆环及说明范围，包含描边外缘；重复同步静止状态不增加损坏。
这项修复不增加轮询或持续计时器。

## 验证入口与范围

```sh
uv run --no-project python scripts/external-actions-reference.py \
  --root target/qt-external-actions-new
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
uv run --no-project --with pillow --with numpy python scripts/wayland_text_sway_runtime.py \
  --root target/private-external-actions-new -- \
  uv run --no-project --with pillow --with numpy python scripts/wayland-external-actions.py \
  --binary target/release/mark-shot --root target/wayland-external-actions-new
```

原版采集器提取完整且未修改的生产函数，包括桌面参数、配置读取、面板内容与几何。
外围仅提供隔离目录和不执行动作的接收端，没有完整链接原版截图窗口。
冻结样本为 `tests/fixtures/qt-external-actions.json`，包括八组面板、六组命令、
四组 URL 和五组配置；源码、探针与编译产物指纹分别保留。
Rust 对照检查实际参数、URL、配置和非空面板几何。
局部绘制测试在 0.75、1、1.2、2 倍下逐像素比较透明面板、文字及图片图标。

私有 Wayland 脚本执行真实列表、右键隔离、左键交付、末项导航、区域独立命令、
带框图片、工作目录、实际 exec 失败后的恢复和保存，并检查空闲帧、线程与 CPU。
运行记录按二进制 SHA-256 关联，不把第一轮失败视为成功验证。

本轮本地二进制为 46,258,168 字节，SHA-256 为
`a01f6e80556706fa9839091d50680f70155a0cb46e5dfc2d3f859ceac478e43d`。
汇总索引为 `target/external-actions-verification.json`。

| 验证 | 本轮结果 |
| --- | --- |
| 工作区 Rust 测试 | 974 项通过，10 项原有忽略，无额外过滤 |
| Python 工具测试 | 199 项通过 |
| 格式、全部目标与功能的严格 Clippy、优化构建 | 通过 |
| 当前 Qt 函数与冻结样本 | 八组面板、六组命令、四组 URL、五组配置匹配 |
| Sway 与 niri 的实际输入、PNG、进程与失败恢复 | 两组最终完整场景通过，右键实际悬停命中另存截图 |
| 动态依赖 | 没有 Qt、GTK 或 WebView |

最终真实记录分别为 `target/wayland-external-actions-sway-final/result.json` 和
`target/wayland-external-actions-niri-final/result.json`。在 0.8 秒静止采样内，两组均为
单线程，CPU 计数和表面损坏提交不增长；RSS 分别为 52356、51992 KiB。
这只描述本机当前场景，不代表全部设备、冷启动或完整合成器性能矩阵。

第一轮面板差分同时包含旧圆形放大区的移除，保留原始失败画面，没有放宽整个画布断言。
最终 niri 首次尝试因测试目录过长，IPC 路径超过 Unix 套接字限制而退出；运行器已回收
对应会话。短目录 `target/p-ext-niri-final` 的重试通过，失败日志保留于
`target/private-external-actions-niri-final/niri.log`，不修改产品协议或补发指针动作。
两项旧 Python 测试仍模拟 `wtype`，已改为在协议键盘的就绪确认边界模拟外部输入，
继续验证同一设备、完整键码、正常回收与超时只回收所属进程；真实键盘回归没有删除。

Qt 和 Rust 的完整逐像素一致、系统所有图标主题及 XPM、工具提示、中文空状态排版、
原版区域键盘调整放大区、跨输出面板交互及全部合成器矩阵仍需覆盖。
整体重写继续保持未完成。
