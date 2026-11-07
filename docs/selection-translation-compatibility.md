# 截图内翻译兼容与验证

截图编辑器已接入原图位置翻译。语言弹层、结果浮条、各目标语言缓存、原文恢复、完整译文
复制与原图加翻译数据的贴图交接分别使用独立模块；截图复制、保存及上传使用当前显示。
完整界面和全部合成器目标仍未完成，不将本专项当作重写完成。

## 原版语义

- 首次翻译识别当前带标注选区，保存冻结原图用于取色和恢复；识别坐标位于区域内部。
- 成功后才建立活动翻译会话；失败、空结果和取消保留当前画面。
- 每种语言保存独立结果，切换已经完成的目标不重新识别或请求。
- 原文与译文切换只替换区域像素；退出翻译逐字节恢复原始片段，不更改标注历史。
- 复制浮条使用完整段落文字，显示省略不会截断剪贴板内容。
- 选区移动或缩小时翻译仍属于初始区域；浮条只在当前选区与该区域相交时显示。
- 转贴图先用原图导出标注，再按新裁切原点换算 token、底板、行框、字号和基线。
  贴图保留原图，使用自己的显示切换；其他导出继续使用截图当前画面。

`src/features/selection_translation/` 管理区域和缓存，
`src/ui/selection_translation/` 管理列表与浮条，
`src/app/selection_translation/` 管理任务、协议、输入和交接。

## 原生输入与布局

语言列表宽度 132–240 像素，标题 20 像素、语言行 26 像素；结果浮条高度 30 像素。
文字使用按需加载的独立 Noto Sans 字库，不改变启动时的内置字库。宽度使用原版
1/64 像素字宽量化；文字按钮向上取整，语言列表按最近整数度量。

语言弹层使用原生 `xdg-popup` 和抓取，关闭角色后才启动任务。当前目标支持自定义语言，
上下键、Tab、Home、End 与确认键只作用于列表。完整左键按下和释放才能选中；右键
不触发语言或浮条动作。弹层外点击关闭并消费当前点击，避免落入画布手势。
明暗切换保持当前抓取，浮条和列表复用工具栏配色及原版 SVG。
键盘打开菜单时先传递真实按键序列号，并保留同一座席的按下记录；不会复用已经关闭的
鼠标菜单抓取。Sway 和 niri 的第二轮协议检查曾同时捕获这一缺陷，修复后重新验证。

浮条保持原版六像素间距、八像素边距和四像素障碍物留白；优先区域外侧右对齐，
受阻时平移，最后放进区域右上角。当前避让对象为主栏、副栏、属性栏及实际可见的尺寸面板。
尺寸面板语义与物理裁切的后续调整见 [selection-size-compatibility.md](selection-size-compatibility.md)。

## 识别资源回收

初次实现复用了截图 OCR 运行时。真实 Sway 预检连续复现识别后五条线程，
内核线程名明确包含 `mark-ocr-0` 到 `mark-ocr-3`。原因是
`ocr::parallelism::initialize` 创建了全局 Rayon 线程池，释放识别后端不能销毁该池。

截图翻译现复用贴图已有的一次性识别进程。共用实现提取到 `src/ocr/worker.rs`，
旧贴图入口保持兼容；输入和输出使用有界匿名文件，进程退出时释放模型和并行线程。
截图通过一次完成通知等待结果，取消和窗口退出负责真实回收；静止画面没有识别轮询。
首次模型引导先暂停覆盖层、释放输入，下载完成后继续同一截图翻译，取消恢复截图。

失败记录保留在 `target/wayland-selection-translation-sway-preflight/`、
`target/wayland-selection-translation-resources-r1/` 和 `r2/`。
`r2/checks/thread-probe.json` 保存具体线程名。
资源最小复现的修复结果见 `target/wayland-selection-translation-resources-green/checks/`：
恢复单线程，连续一秒 CPU 时间和缓冲提交数没有增长。该报告使用中间构建，完整会话
另按最终发布构建记录。

## 桌面捕获顺序

niri 第五轮完整流程在真实桌面捕获时报告无文字。最小复现保留在
`target/selection-translation-capture-red/` 和 `target/selection-translation-capture-wait/`：
背景贴图已经实际呈现，等待其首帧也不能消除失败。协议记录显示截图进程先创建并提交
空白覆盖层，再发出 `wlr-screencopy` 请求，因此捕获了自身窗口。

`src/app/capture_setup.rs` 现先取得桌面帧，再在独立线程映射覆盖层；字体、图标与基础
图层仍并行准备。捕获失败时不会创建空白截图窗口。完整会话检查原生帧 `ready` 必须早于
覆盖层 `get_toplevel`，同时仍验证真实区域 OCR、译文和物理分辨率导出。

niri 第四轮另捕获了私有 Xvfb 的输入送达问题：X11 坐标已变化，Wayland 事件仍使用旧
位置，导致语言按钮保留悬停色。回归工具现使用同一设备先移动到相邻坐标，再送达目标，
点击前完成移动；整图比较仍要求所有像素一致。最小记录见
`target/selection-translation-hover-red-r2/` 和 `target/selection-translation-hover-fixed/`。

## 验证

实际 Qt 6.12.0 生产控件及位置模块生成八个语言位置、二十个浮条位置、五种语言标签的
全部按钮尺寸，以及右键与键盘样本。英文和 Fusion 样式由探针固定，字体为本机
Noto Sans。`tests/selection_translation.rs` 对这些位置和尺寸使用严格相等比较。
参考 JSON 仅按条目压缩格式，原始输出指纹与规范化文件指纹分别保留。

新增内部测试覆盖真实 PNG 导出、原图恢复、透明图片预览、裁切坐标、跨输出缩放、
左右键序列和缓存保持。全特性 Rust 工作区 1028 项通过、10 项既有忽略，共 67 套；
格式及全部目标、全部特性严格 Clippy 通过，Python 单元套件 199 项通过。

此前截图翻译发布文件 SHA-256 为
`3117af18a6b4e73fdf5778df74613d6373f8af4210e592a5f555cd85c9ba1ca5`，大小 48,252,936 字节，
动态依赖全部可解析且没有 Qt。`scripts/wayland-selection-translation.py` 在隔离 Sway 和
niri、1.2 分数缩放及翻转输出上分别完成七次实际请求，验证原生抓取、左右键隔离、明暗
恢复、冻结像素替换、完整复制、语言缓存、退出重入、空响应、命令取消与实际回收、贴图
原图交接、引导取消、真实模型下载后继续同一截图，以及原生桌面区域翻译和物理像素导出。

两套完整流程各记录八次空闲采样，共 16 次；所有采样保持单线程，连续一秒 CPU 时间和
缓冲提交数均不增长。下载的三个模型资产逐文件核对 SHA-256，两套结果一致。
完整构建、Qt 参考、模型及资源证据见 `target/verification-selection-translation/result.json`；
实际会话见 `target/wayland-selection-translation-sway-r5/checks/result.json` 和
`target/wayland-selection-translation-niri-r6/checks/result.json`。

同一隔离 niri 会话各做三次温启动采样：捕获顺序修正前后，内部首缓冲提交中位数由
11.49 ms 变为 15.79 ms；修复后父进程观察中位数为 20.81 ms，空闲 RSS 约 50.3 MiB。
记录见 `target/selection-translation-performance-before/report.json` 和 `after/report.json`。
这些数字不代表冷启动或合成器实际呈现时间。

后续选区尺寸发布构建 SHA-256 为
`9da1119d85d29ab2f4858008411570e60ce3e406e48bdd8cdd0c6c09fccc9156`，大小48,344,072字节。
这份构建重新在私有 Sway 和 niri、1.5分数缩放及翻转输出上各完成七次完整请求，实际下载
模型后继续翻译；另16次空闲采样保持单线程，CPU 时间与缓冲提交计数没有增长。回归工具
改为等待正式主题配置与当前菜单新缓冲，避免文件同步较慢时提前检查。新的结果见
`target/wayland-selection-size-translation-sway-r3/checks/result.json` 和
`target/wl-size-tr-niri-r3/checks/result.json`，统一构建清单见
`target/verification-selection-size/result.json`；此前清单及性能记录保持原指纹。

## 仍需覆盖

全部语言字体、完整 Unicode 与字形像素、工具提示、原版选区尺寸提示避让、其他合成器，
以及大选区多语言缓存的内存上限尚未完成专项对照。上述字体尺寸结果不代表完整像素一致。
