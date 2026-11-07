# Windows 主应用与原生工作流

Windows 主应用已接入共享截图、编辑与渲染流程。当前 R3 候选在私有 Wine 和不可见 Xvfb
中完成区域截图、明暗主题恢复、画笔标注、保存、系统剪贴板、基本贴图、真实模型下载、
OCR 推理与独立结果编辑回归。
这些证据来自 Windows 可执行文件实际运行，不代表真实 Windows 11 的兼容性或性能验证。
完整 Windows 交互目标仍未完成。

## 候选与源码

| 产物 | 字节数 | SHA-256 |
| --- | ---: | --- |
| 当前 `target/windows-platform-candidate-r3/mark-shot.exe` | 44,239,872 | `2f04db4e6cf1bbc0849577541d3c92462a2ba9fb087e4936ff15b0c7a5328cd5` |
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

- 中文输入法候选、确认、焦点转移和字符输入消息的真实 Windows 完整矩阵；当前已验证中文粘贴、编辑与系统剪贴板。
- 贴图右键菜单、旋转、缩放、另存为和编辑交接已完成上述基础回归；显示边框、文字选择及部分
  翻译动作仍未接入，相关动作明确禁用，真实 Windows 和完整输入矩阵仍需验证。
- 独立窗口的系统交互缩放、菜单抓取与多级菜单、模态行为、混合 DPI 和透明剪贴板实测。
- Windows 滚动截图控制入口；原生持续捕获会话已有实现，覆盖层入口目前明确不支持。
- DWM 精确窗口边界、遮挡独立捕获及圆角透明度；活动窗口当前使用可见窗口的
  `GetWindowRect` 与 `BitBlt`。
- Windows 单实例命令转交、延迟计划替换和持久 OCR 服务。当前单实例只有文件锁占用，
  重复启动会报告已有实例；OCR 使用按需 CPU 引擎。
- 真实 Windows 11 的启动、稳定内存、峰值、退出资源、多显示器与输入法矩阵。

以上未完成项不以编译或 Wine 基本工作流通过代替。
