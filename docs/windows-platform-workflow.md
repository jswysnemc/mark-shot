# Windows 主应用与原生工作流

Windows 主应用已接入共享截图、编辑与渲染流程。此前 R3 候选在私有 Wine 和不可见 Xvfb
中完成区域截图、明暗主题恢复、画笔标注、保存、系统剪贴板、基本贴图、真实模型下载、
OCR 推理与独立结果编辑回归。
这些证据来自 Windows 可执行文件实际运行，不代表真实 Windows 11 的兼容性或性能验证。
完整 Windows 交互目标仍未完成。

## 当前源码同构建复验

完整三组流程候选为 `target/current-windows-candidate-r1/mark-shot.exe`，44,253,696字节，SHA-256：
`05bd9e34ddd8c02e644ac132ae077bbe7e1cdb660bad352632dbf28f5f83a829`。
同目录保存 `DirectML.dll`、`build.json`、`source-manifest.json` 与 `source-inputs.tar.gz`。
1,198项代码和资产在构建期间记录摘要，构建后再次核对一致；文件清单不包含本地配置和运行模型。
这份候选包含旧全局主题继承、删除悬停、滚轮锚点及菜单主题快捷键等后续修改。

同一EXE依次通过三个新建私有Wine会话，全部结果绑定上述摘要：
- `target/current-windows-basic-r1/workflow.json`：区域捕获、编辑明暗恢复、画笔、保存/剪贴板和基本贴图
- `target/current-windows-pin-r1/workflow.json`：键盘菜单、滚轮位置尺寸恢复、原图像素、外部主题、菜单内快捷键、另存为及编辑交接
- `target/current-windows-ocr-r1/workflow.json`：正式模型首次下载、真实推理、结果Unicode编辑与撤销、最小窗口、外部主题及模型复用

没有以早期候选的结果替代本次回归。共享删除悬停和旧配置矩阵仍主要来自Linux实测，
此三组Windows流程不能扩展为全部工具、混合DPI或真实Windows桌面交互已通过。

后续补齐贴图Ctrl+V：与原版及Wayland相同，从剪贴板启动另一张独立贴图，不替换当前图像。
复用 `--pin-clipboard` 入口，菜单打开时不传递给底层窗口。
旧候选在 `target/windows-pin-paste-red-r1` 实测无法创建新窗口；修复候选
`target/windows-pin-paste-candidate-r1/mark-shot.exe`，SHA-256
`47e4365eb737f1815b6ab2cb883e40234c76e1f0b3a61c29cecba2948a865452`，
在同构建 `target/windows-pin-paste-green-r1/workflow.json` 验证两个窗口独立、原件逐像素相同、
关闭新窗口后保留原窗口，以及菜单期间屏蔽粘贴。完整贴图与
`target/windows-pin-paste-basic-r1/workflow.json` 基础流程通过，Windows严格Clippy与格式检查通过。
同一文件后续完成 `target/windows-pin-text-r2/workflow.json` 文本贴图验证：英文三行含空行
生成152×104图像，中文标点及补充平面字符生成192×104图像，实际导出可见字形已检查；
深浅快捷键确认配置变化后再次复制，原件RGBA逐字节不变，关闭文本子窗后原窗口和原件保持不变。
私有测试前缀使用MiSans VF及MiSans L3，摘要保存在报告，没有加入产品包。
空白文本直接执行同一剪贴板入口返回1并明确报告无图像或文本，没有新增窗口。
这不代表从存活贴图按Ctrl+V遇到空剪贴板时已有可见错误提示；该交互仍未完成。

Windows窗口最小尺寸此前把逻辑值直接交给 `WM_GETMINMAXINFO`，高DPI下未维持布局下限。
`native/window_limits.rs` 现在按 `GetDpiForWindow` 向上取整转换，零DPI回退96并限制整数边界。
私有Wine真实窗口消息在120、144、192 DPI分别返回逻辑380×340对应的475×425、570×510、760×680；
测试明确断言实际DPI，记录位于 `target/windows-dpi-{120,144,192}-r1/program.log`。
七组73项测试通过、零忽略，Windows严格Clippy通过。
同一发布构建 `target/windows-dpi-candidate-r1/mark-shot.exe`（SHA-256
`3adbc364f60e5523dbbf6d561aff8b60d8377d0015d6dca9fea4f1a0cb32c396`）
通过 `target/windows-dpi-ocr-r1/workflow.json` 和 `target/windows-dpi-pin-r1/workflow.json`。
这两组应用流程仍使用默认DPI；高DPI证据仅覆盖真实窗口的最小尺寸消息，不代表跨显示器DPI切换矩阵。

## 候选与源码

| 产物 | 字节数 | SHA-256 |
| --- | ---: | --- |
| 此前 `target/windows-platform-candidate-r3/mark-shot.exe` | 44,239,872 | `2f04db4e6cf1bbc0849577541d3c92462a2ba9fb087e4936ff15b0c7a5328cd5` |
| `target/windows-platform-candidate-r2/mark-shot.exe` | 44,239,872 | `87377e831660db009ac9e1f2bcda15cbdc64ad1dda5c0670933650fd6d2a15e4` |
| `target/windows-platform-candidate-r1/mark-shot.exe` | 44,239,872 | `dedad983fb30390e0e074c61af84c8529db0fccfee041de6f9557b7a1ce13d0e` |
| 同目录 `DirectML.dll` | 18,527,776 | `9c9e6d822561c6c41b90e6994b3e8857cf1d66dbfb1e0c4c799c7c89b4e92da1` |
| Linux 对应候选 `target/platform-contract-candidate-r1/mark-shot` | 50,097,608 | `f0ea7e0671e05d4b51d35315a3be6d01838faf47b1ecb47ad5668497da0713b0` |

Windows 可执行文件和 `DirectML.dll` 必须保存在同一目录；候选的导入表没有 Qt 依赖。
OCR 当前使用按需 CPU 推理，附带 DirectML 动态库来自链接依赖，并不表示启用了 GPU OCR。
R1 与 Linux 对应候选附带 `source-manifest.json` 和 `source-inputs.tar.gz`，各封存 1,975 项构建输入。
R3 封存 1,982 项工作区文件，编译源码与资产和构建前清单一致，文档包含最终验证结果。
R2 保留的是后续工作区快照，不能视为当时构建输入的精确副本，说明记录于其 `build.json`。
验证脚本后来有修订，当前脚本不能视为初始封存脚本的相同版本。

原发布文件、R20 内存候选和主题 R2 候选保持独立。没有提交或发布本轮修改。
R20 的选用数据与未通过门槛仍以 [内存方案](selected-memory-scheme.md) 为准。

## 模块边界

```text
src/backend/
├── session.rs              平台会话入口
├── platform/               按目标系统创建捕获、窗口与剪贴板后端
├── window.rs               共用窗口、菜单、输入和模态契约
├── wayland/                Linux 原生实现
└── windows/
    ├── native/             显示器、DPI、DIB、捕获、消息循环、窗口和输入法
    ├── capture.rs          原生屏幕捕获与持续会话
    ├── overlay/            截图覆盖层、共享局部合成和输入
    ├── window/             独立窗口、父子菜单和模态运行
    ├── clipboard/          PNG、DIBV5、Unicode 和系统所有权
    └── pin/                贴图运行与基本动作
src/features/pinned_image/menu_draw.rs
                            共用贴图菜单绘制
```

Linux 专用的 Wayland、PipeWire、DBus 和 Unix 依赖已从 Windows 构建隔离。
共享编辑状态不直接引用 Wayland 连接或窗口类型。覆盖层在创建它的界面线程运行，
不跨线程转移 HWND。Windows 辅助进程不显示控制台；外部命令用作业对象管理后代进程。
配置保存使用平台文件锁与原子替换，历史和编辑快照采用可清理的命名文件。
字体使用 Windows 目录和样式、缺字匹配；主题刷新使用目录通知。

## 已完成的实际回归

R3 主工作流证据为 `target/windows-platform-verification-r6/workflow.json` 与 `result.json`；
R1、R2 的既有回归结果继续保留。
验证脚本是 `scripts/windows-platform-workflow.py`，运行器是
`scripts/windows_test_runtime.py`。每次测试创建新的 Wine 前缀和 Xvfb，清除用户桌面相关
环境变量；独立原生 Windows 样本窗口只提供确定像素，主应用执行捕获、编辑与输出。

| 场景 | 实际断言 |
| --- | --- |
| 显示器发现 | 发现唯一 1280×800 输出，原生尺寸与比例正确 |
| 区域捕获 | 200×160 PNG 与独立屏幕样本对应区域逐字节一致 |
| 浅色主题 | 浅色、深色、浅色往返后整幅界面精确恢复 |
| 首次 OCR 引导 | 未安装模型时显示实际模型选择面板 |
| 原图复制 | 主题和模型面板不进入完整 1280×800 剪贴板图像 |
| 画笔与保存 | 实际鼠标绘制只改变笔迹范围；保存 PNG 与剪贴板逐字节一致 |
| 贴图复制 | 系统剪贴板图片进入独立贴图，再次复制仍保留完整尺寸与原始像素 |

画笔默认平滑系数为 0.7，导出笔迹会滞后原始鼠标终点。该行为不作为事件丢失证据；
需要核对每个原始终点的测试应显式设置 `tools.pen.smoothing = 0`。

Linux 对应候选通过两组私有 Sway 回归：

- `target/platform-priority-verification-r1/result.json`：尺子悬停、三类滚轮提示、
  中文贴图菜单及主题恢复；贴图空闲 RSS 稳定为 39,088 KiB，单线程，CPU 与提交数不增长。
- `target/platform-font-verification-r1/result.json`：明暗字体列表、滚动、搜索、
  外部主题刷新、正文编辑和完整原图导出。

Linux 工作区共 1,363 项测试通过、19 项忽略，覆盖 74 个套件；全目标严格 Clippy 与格式
检查通过。Windows 工作区全部目标的严格 Clippy 通过，包括主库、二进制、测试与示例，
日志为 `target/windows-all-targets-clippy-r6.log`。主库六组共 66 项 Wine 测试通过、零项忽略，
包括五项原生资源、31 项配置、九项历史存储、两项保存和十九项 OCR 控件与编辑事务；
证据为 `target/windows-unit-verification-r2/tests.json`。仅适用于 Linux 的 Portal、协议、
Unix 路径和实际外部进程测试按目标限定，不作为 Windows 实测。

Windows 无界面捕获曾超过十秒外层期限。诊断确认约十一秒后正常退出，捕获、编码和保存
阶段约十至二十五毫秒，独立运行时初始化约两毫秒；完整启动延迟仍未定位。
目前捕获期限为二十秒，像素断言保持原要求，不能据此宣称已达到快速启动目标。

## 真实 OCR 与结果交接

同一 R2 可执行文件通过 `target/windows-ocr-verification-r4/workflow.json` 和
`target/windows-ocr-verification-r5/workflow.json` 两组实际流程：

- 新前缀模型目录为空，空闲和右键不启动下载；右键关闭面板后重新定位下载按钮。
- 三项模型从代码中的正式发布地址下载，不预置模型、不替换 URL，大小与 SHA-256 全部正确；
  安装后没有残留 `.part` 文件，下载完成自动继续识别。
- 内置 CPU 引擎识别固定图片的三行原文，独立结果窗口出现后识别进程正常退出。
- Windows Unicode 剪贴板提供者退出后，结果窗口仍可粘贴中文、多行、标点与 `𠮷`；
  全文复制、撤销重做、原图页签切换和 380×340 最小窗口保持完整正文。
- 关闭结果窗口后系统保留复制内容；再次识别复用相同模型，结果文字一致且模型文件不改变。

最终源码构建为 R3 后，使用同一 R3 文件重新执行基础截图与含测试字体的完整 OCR 流程，
两组均通过，证据为 `target/windows-platform-verification-r6/workflow.json` 和
`target/windows-ocr-verification-r6/workflow.json`。没有以旧候选结果代替最终构建回归。

R1 实际下载和推理成功后曾出现交接失败：固定三秒期限到期时辅助窗口尚未确认，
随后结果窗口仍然出现，截图编辑器也保留。固定结果帧诊断记录于
`target/windows-ocr-ready-r2/ready.json` 和 R3 同目录；有效确认约 3.36–3.42 秒后到达，
版本命令本身约 3.66 秒。R2 为 Windows 独立结果和历史窗口提供十五秒有界期限，
收到真实首帧确认立即返回，没有提前确认或跳过绘制。Linux 继续使用三秒期限。
该修复解决交接失败，不代表冷启动速度已经达标。

R4 使用空的 Wine 字体目录，Unicode 数据与历史正确，中文画面显示缺字方框。
R5 在本次私有 Windows 字体目录安装 MiSans VF 与 MiSans L3 测试字体，实际画面完整显示
中文和补充平面字符。字体来源摘要记录在流程清单中，没有加入产品资源或宿主字体目录。
`target/windows-font-glyph-verification-r1/glyphs.json` 通过产品 Windows 字体匹配确认全部
十一项字形编号非零，普通中文选择 MiSans VF，`𠮷` 选择 MiSans L3。
这些检查验证字体目录和缺字回退，不等同于真实 Windows 字库或输入法候选验证。

最终核对确认 R20、主题 R2、已有平台候选与原始参考文件保持指纹。
原始参考文件是 `target/single-instance-candidate-r1/mark-shot`，SHA-256 为
`4852decc52218af893df3cc617613e68feaef9fd1f5455c9dcf6d70fb2c24c53`。
`target/release/mark-shot` 在本轮 OCR 工作之前已是 Linux 平台候选，摘要为
`f0ea7e0671e05d4b51d35315a3be6d01838faf47b1ecb47ad5668497da0713b0`；本轮未写入该路径。

## 尚未完成

2026-11-07 贴图补充回归使用同一 R3 可执行文件通过，结果为
`target/windows-pin-verification-r2/workflow.json`，脚本为 `scripts/windows-pin-workflow.py`。
实际覆盖菜单按下不执行、跨行释放取消、Escape 只关闭菜单、左右旋转原图逐像素恢复、
放大和复位不改变复制分辨率、打开菜单期间外部明暗配置刷新及浅色整帧恢复、
另存为取消后恢复贴图、保存 PNG 与原件完全一致、独立编辑交接与再次复制、菜单关闭退出。
首轮失败来自测试把完整路径填入只允许文件名的字段；第二轮按产品契约分别填写目录与文件名，
没有放宽产品校验。验证保存目录限定在本轮工作目录。以上仍是私有 Wine/Xvfb 实测。

后续贴图键盘候选补齐 Up/Down 循环、Home/End 定位、Enter/Space 确认和 Escape 取消，
跳过分隔线及未接入动作；键盘接管清除未完成鼠标按下，避免重复执行。
共享导航逻辑为 `src/features/pinned_image/menu_navigation.rs`，Windows 运行时只负责事件交付。
旧 CI 文件在 `target/windows-pin-keyboard-red-r1` 实测 Enter 不关闭菜单；修复候选
`target/windows-pin-navigation-candidate-r1/mark-shot.exe`（SHA-256
`be0a09c90dcf4832423650056e810c988f46702df3c10c8798a812bf085f48cc`）
在同一构建完成贴图键盘及原有菜单、截图保存、完整首次 OCR 与独立主题流程：
`target/windows-pin-keyboard-green-r2/workflow.json`、`target/windows-pin-keyboard-basic-r1/workflow.json`、
`target/windows-pin-keyboard-ocr-r1/workflow.json` 均通过。初次修复验证遇到测试工具对同位置
鼠标移动等待事件超时，改为读取实际指针坐标确认后再次验证；没有重放不确定点击。
工作区 1,369 项通过、19 项忽略，Python 225 项通过，两平台严格 Clippy、格式及 Ruff 检查通过。
候选仍是本地独立构建，不把此前 CI 成功记录移用到本次修改。

菜单存活时主题快捷键此前绕过主窗口处理，已在 `target/windows-menu-theme-red-r1` 实测失败。
新增 `src/backend/windows/window/theme_input.rs` 后，主窗口和两条菜单事件路径共用主题入口，
切换不会关闭或替换菜单，也不会交付菜单动作。同一候选
`target/windows-menu-theme-candidate-r1/mark-shot.exe`，SHA-256 为
`62538b5cbeab7f44300c9e5555a1db8daaeb6434099c15fb48123f3407fc8cc1`，
通过完整贴图流程与含真实模型下载、独立主题刷新的 OCR 流程：
`target/windows-menu-theme-green-r1/workflow.json` 和 `target/windows-menu-theme-ocr-r1/workflow.json`。
Windows 全目标严格 Clippy 与格式检查通过；实际界面证据仍限定私有 Wine。

滚轮后续候选按照原版 `PinnedImageWindow::wheelEvent` 的角度每格1.08倍、像素八十单位一格
补齐 Windows 缩放，并通过一次性逻辑位移保持鼠标原图锚点。当前DPI换算只在原生窗口进行，
同帧多个输入累计位移；横向滚轮和菜单期间输入不改变底层图像。
旧菜单主题候选在 `target/windows-pin-wheel-red-r1` 实测保持320×200不缩放；新候选
`target/windows-pin-wheel-candidate-r1/mark-shot.exe` 的 SHA-256 为
`02ffcf5578b071760177f77eadea3cae0ae1ab31433b2fb7cf06017bbf12cca0`。
同一文件通过 `target/windows-pin-wheel-green-r1/workflow.json` 和
`target/windows-pin-wheel-ocr-r1/workflow.json`。实际窗口从(180,180,320,200)变成
(173,176,346,216)，反向滚轮精确恢复原位置和尺寸，复制原图逐字节一致；完整贴图和OCR回归通过。
工作区1,371项通过、19项忽略，两平台严格Clippy及格式检查通过；混合DPI、连续原生输入
与真实Windows桌面矩阵尚未验证，不能从单比例Wine结果外推。

透明剪贴板补充五项Windows测试：全部256种透明度与未预乘颜色、顶向/底向DIBV5、
二十四位DIB行填充、不完整头部/溢出调色板/截断像素拒绝，以及真实系统PNG与DIBV5所有权。
系统测试销毁隐藏拥有窗口后再次读取，再只交付DIBV5验证回退，最后用Unicode替换并确认旧图像消失。
测试只在显式私有会话中执行，不改动用户剪贴板；四项纯格式测试加入Windows CI分组。
`target/windows-alpha-unit-r2/tests.json`五项通过，随后同一测试程序在
`target/windows-alpha-full-r1/tests.json`完成七组71项、零忽略。Windows严格Clippy通过。
本轮没有更改产品编码逻辑，结果是新增实际覆盖，仍不等于与真实Windows第三方应用的互操作矩阵。

- 中文输入法候选、确认、焦点转移和字符输入消息的真实 Windows 完整矩阵；当前已验证中文粘贴、编辑与系统剪贴板。
- 贴图右键菜单、旋转、缩放、另存为和编辑交接已完成上述基础回归；显示边框、文字选择及部分
  翻译动作仍未接入，相关动作明确禁用，真实 Windows 和完整输入矩阵仍需验证。
- 独立窗口的系统交互缩放、菜单抓取与多级菜单、模态行为、混合 DPI，以及透明剪贴板与真实Windows第三方应用互操作。
- Windows 滚动截图控制入口；原生持续捕获会话已有实现，覆盖层入口目前明确不支持。
- DWM 精确窗口边界、遮挡独立捕获及圆角透明度；活动窗口当前使用可见窗口的
  `GetWindowRect` 与 `BitBlt`。
- Windows 单实例命令转交、延迟计划替换和持久 OCR 服务。当前单实例只有文件锁占用，
  重复启动会报告已有实例；OCR 使用按需 CPU 引擎。
- 真实 Windows 11 的启动、稳定内存、峰值、退出资源、多显示器与输入法矩阵。

以上未完成项不以编译或 Wine 基本工作流通过代替。
