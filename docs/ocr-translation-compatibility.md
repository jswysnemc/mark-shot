# OCR 翻译窗口对照

本阶段在本地将翻译任务接入 OCR 结果窗口，保留独立原文、译文与目标语言编辑器。
没有执行 Git 提交、推送、发版或修改远程工作流，没有使用用户翻译凭据或外部付费服务。
服务层和剩余接入差异见 `docs/translation-service-compatibility.md`。

## 模块职责

```text
src/features/ocr_result/translation/
  state.rs                请求身份、成功快照、取消和过期提示
  controller.rs           请求、交付、语言保存和非阻塞交互取消
  layout.rs               原版断点、分栏、语言与操作行几何
  notice.rs               提示换行字形，测量与绘制使用同一缓冲
  draw.rs                 译文编辑区、操作行、进度和统计
  input.rs                原文、译文、语言与表格输入拥有者
  language_menu.rs         独立原生列表、滚轮、键盘和释放选择
  language_menu/text.rs    原版字体、省略号与普通/选中两态缓存
src/config/translation/
  decode.rs               超时字段存在性和严格类型解码
  encode.rs               序列化时保留缺省或显式覆盖的区别
```

## 已接入行为

- 标题栏翻译开关首次展开空译文时发起任务；已有译文收起、重开不再次请求
- `Ctrl+Enter` 发起翻译，执行按钮在任务期间显示取消；关闭窗口回收当前及已经取消的任务
- 忙碌时原文仍可编辑；译文保留选择、导航与复制，但拒绝修改，目标语言禁用
- 原文和目标变化显示过期提示，不用返回结果覆盖继续编辑的原文
- 译文拥有独立撤销、全文复制、右键菜单、字素统计和复制反馈期限
- 译文聚焦时 `Ctrl+Shift+C` 复制译文；标题栏全文复制始终复制原文
- 切换原图页后恢复文字页焦点；程序替换译文身份后旧菜单不能修改新结果
- 自由目标语言以 Enter 确认，保存沿用历史与翻译共用的原子更新锁，保留未知字段及厂商配置
- 八行语言列表使用真实 `xdg_popup` 与输入抓取，第九项通过 End、滚轮和左键释放可达；右键不选择
- `<620` 使用纵向分栏，`<480` 隐藏语言标签；分栏支持拖动，几何随窗口调整

## 原版对照

`tests/fixtures/qt-ocr-translation.json` 冻结六种宽度、每宽度十二个业务状态，共 72 个状态。
原生状态序列和全部可见分栏、目标输入、操作行、编辑器、提示、进度矩形逐项对照。
补充记录 `tests/fixtures/qt-ocr-translation-focus-popup.json` 核对切页恢复焦点和语言列表。
最新只读 Qt 探针位于 `target/qt-ocr-translation-r6/`，两次完整采集结果一致，原有状态记录没有变化。

列表字体为 DejaVu Sans 16 像素、行高 19 像素；每项高 30 像素，八行视区高 240 像素，
包含边框的弹层高 250 像素。文字视区为 `[5, 5, width-16, 240]`，
右侧滚动区域为 `[width-11, 5, 6, 240]`，最大滚动值为一，页步长为八。

第一次私有 Sway 回归通过交互断言，但截图检查发现过期提示绘制没有换行。
新增像素测试复现 620 像素宽度下提示越过分栏边界，旧 Sway 截图也只呈现一行。
修正后布局与绘制共用有界提示缓冲，并裁切到提示矩形；真实截图断言要求完整呈现两行。
不将首次交互通过记录视为最终视觉验证。

niri 回归还复现点定位弹层的分数坐标边界问题：四舍五入把弹层左上角移动到指针右下方，
指针没有进入菜单，关闭后首次移动仍报告旧位置。增加等待或更换输入设备均未消除失败。
点定位改为向下取整，使原始指针仍位于锚定整数单元；控件和子菜单定位规则不变。
坐标测试先失败再通过，私有 niri 协议随后记录菜单进入、关闭恢复和正确的首次移动，未增加重试。

## 资源与配置

交互取消只发出通知，将任务句柄暂存到待回收集合，不阻塞窗口等待网络或进程。
临时五毫秒期限只用于确认线程真实退出，集合清空后停止计时；结果通知不能代替线程退出。
窗口事件循环结束后，销毁过程取消并等待当前任务及已经取消的任务完成资源清理。
实际阻塞命令测试同时核对直接子进程、后代进程和私有输入文件，应用退出时直接子进程与输入已不存在。

缺省全局超时不注入厂商配置，腾讯等服务保留原版三十秒默认值。
用户显式填写全局六十秒仍属于有效覆盖。默认 TOML 模板和序列化均不凭空添加全局超时；
原有默认模板一致性测试和新增字段存在性往返测试分别覆盖这两个入口。

## 当前构建验证

检查索引位于 `target/ocr-translation-window-local-r1/validation.json`。
优化构建为 45989800 字节，SHA-256 为
`d343ab4748ca3ebee758c852e634242e5a1a519f6c9ea31b3531cd589ba287ac`。
892 项 Rust 测试通过、10 项忽略；一项既有尺子失败单独记录，与未修改 HEAD 相同。
翻译相关 57 项测试及新增点锚定边界测试通过，Python 180 项、严格 Clippy、格式和差异空白检查通过。
动态链接不包含 Qt、GTK 或 WebView。

Sway 1.2、Sway 1.0 和 niri 1.2 使用同一优化构建，分别通过交互、运行中关闭及取消后立即关闭。
记录位于 `target/ocr-translation-window-sway-r3/`、
`target/ocr-translation-window-sway-scale1-r2/` 和 `target/ocr-translation-window-niri-r4/`。
各组均使用单次虚拟指针输入，完整呈现两行提示，右键菜单关闭后首次操作正确交付。
空闲结果窗口保持单线程、CPU 时钟不增加，没有周期性表面提交。

同构建还通过复制反馈、更多菜单、按钮输入、原文与表格输入法、历史窗口回归，
对应 `target/ocr-translation-window-feedback-r2/`、`target/ocr-translation-window-more-r2/`、
`target/ocr-translation-window-control-r2/`、`target/ocr-translation-window-ime-r2/` 和
`target/ocr-translation-window-history-r2/`。全部私有合成器和显示服务器已经退出。
这些验证不包括实际付费翻译、OCR 模型推理或完整界面逐像素一致。

## 验证入口

```sh
cargo test --locked -j 2 --lib translation
uv run --no-project --with pillow --with numpy python -m unittest discover -s scripts -p 'test_*.py'
uv run --no-project --with pillow --with numpy python scripts/wayland_text_sway_runtime.py \
  --root target/tsu-new -- \
  uv run --no-project --with pillow --with numpy python scripts/wayland-ocr-translation.py \
  --binary target/release/mark-shot --root target/translation-new
```

证据目录必须使用新名称，不覆盖旧构建结果。私有 niri 可使用
`scripts/wayland_test_runtime.py`，运行目录使用短路径，避免 Unix 套接字路径超限。
命令夹具 `scripts/wayland_ocr_translation_command.py` 只读取私有输入并返回可控结果，
测试期间不初始化 OCR 模型，不使用实际翻译服务。

## 剩余差距

- 译文与语言输入法已经按独立身份接入，但候选字形和失焦恢复的完整真实投递矩阵仍需补充
- 独立置顶配置、中文界面、完整焦点遍历、占位文字与滚动条细节尚未全部对照
- 当前语言列表字体、尺寸和输入行为有证据，全部抗锯齿、字形及逐像素相等仍未完成
- 原图译文位置覆盖、图形设置、完整 helper 查找和 OpenAI 插件/builtin 差异仍保留
- KDE、GNOME、其他合成器及完整性能矩阵尚未覆盖

完整重写目标仍未完成。
