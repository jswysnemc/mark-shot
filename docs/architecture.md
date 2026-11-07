# 原生重写架构

## 范围

- Wayland 与 Windows 均为正式平台目标，共享业务与渲染逻辑，隔离原生后端；Windows 主应用已接入，完整交互仍未完成
- 产品完全移除 Qt 依赖，Qt 目录仅作为只读行为与性能对照；不维护产品 X11 后端
- Linux 截图后端优先使用 ext-image-copy-capture、wlr-screencopy；KDE 使用 ScreenShot2，Windows 使用原生显示器与 GDI 捕获
- 不具备原生捕获协议时回退到桌面 portal，滚动截图使用持续捕获和图像拼接
- 软件绘制界面，无 GPU 界面运行时；静态截图和动态标注分层，按损坏区域提交帧
- OCR 使用 oar-ocr / PP-OCRv5，默认按需加载；模型下载必须经过明确用户操作
- 不实现扫码与录像，不将滚动截图视为录像功能

## 模块

```text
src/
  app/                 截图会话、输入调度、帧节奏、导出流程
  backend/session.rs  平台会话入口，业务不直接依赖 Wayland 连接
  backend/platform/   Linux 与 Windows 的后端工厂
  backend/window.rs   共用窗口、菜单、输入、滚轮与模态契约
  backend/wayland/     捕获协议、覆盖层、普通窗口、贴图窗口、剪贴板
  backend/windows/    原生显示器、位图、窗口、输入、剪贴板与贴图
  config/             配置值、界面参数、读取校验、迁移
  editor/             编辑状态、撤销、损坏区域、原图滤镜缓存
  renderer/           标注、文字、像素合成、输出图像
  tools/              按工具划分的输入行为
  ocr/                模型目录、下载、推理、文字布局与选择
  scroll/             滚动捕获会话与预览
  features/laser_trace/ 激光草稿、消散状态与裁切绘制
  features/marker/     二十四种形状标记的路径与屏幕、导出共用绘制
  features/ruler/      长度、角度、控制点、刻度与测量说明
  features/magnifier/  持久双框放大镜的几何、联动缩放和冻结原图采样
  features/number/     独立序号的样式、计数、几何、控制点和分块绘制
  features/line/       端点、有限控制点、平滑路径、独立线型与编辑变换
  features/arrow/      四种头部、尺寸、弧长取点及统一闭合轮廓
  features/filter/     五种原图滤镜、像素网格、旋转控制点与按需区域合成
  features/text/       文字样式、布局几何、字符选区与光标周期
  features/annotation_selection/ 共用选框颜色、几何及删除按钮位置
  features/annotation_rotation/ 独立角度、原版缩放和原始手势计算
  renderer/rotation_tiles.rs 普通旋转正文的固定物理网格合成
  renderer/rotated_text.rs 可见原始字形与单次旋转采样
  ui/annotation_selection/ 虚线、定点栅格、缩放与旋转控制点和短期删除缓存
  app/annotation_selection.rs 删除按钮位置、鼠标归属及局部损坏同步
  features/selection_size/ 图像几何、比例联动、整数验证与面板避让
  app/selection_size/  尺寸应用、字段输入、按钮持有、点击序列、光标调度与原生比例列表
  app/selection_size/field_clipboard.rs  释放复制、有界粘贴请求与编辑身份校验
  backend/wayland/overlay/state/primary_selection/  会话内主选择来源与非阻塞管道
  backend/wayland/overlay/state/pointer_coalescing.rs  保留每个输出极值的移动合并
  types/pointer_motion.rs  合并轨迹范围，不保存逐点列表
  ui/selection_size/   固定布局、词语边界、字体度量、整数视口、矢量路径与物理缓存
  fonts/fixed_layout.rs  选区字段与图片翻译共用的定点字形前进量
  renderer/glyph_run.rs  普通缓冲与指定布局共用的字形缓存绘制
  features/ocr_result/ 独立 OCR 结果、原图、表格与格式导出
  features/capture_history/ PNG 历史、存储预算、预览、搜索及独立窗口
  features/pinned_image/ 原始贴图像素、旋转、菜单动作和独立任务交接
  features/external_actions/ 桌面应用、扩展配置、参数与私有图片交付
  upload/             HTTP 上传、命令执行、有限结果与任务生命周期
  features/           其余桌面集成模块
  ui/                 工具栏、属性面板、输入框、提示、文字编辑器与共用文件选择器
  ui/external_panel/  打开方式和扩展的侧边列表、滚动与有限图标缓存
  utils/              文件锁与辅助进程生命周期
crates/
  shot-image/         纯 Rust 图像缓冲、盒式模糊、像素化和颜色滤镜
  shot-stitch/        纯 Rust 长截图拼接
tests/                命令行与跨模块回归测试
scripts/              安装、性能、私有 Wayland 与 Wine/Xvfb 验证
```

单个源文件保持在 750 行以内，超过阈值前按职责拆分。应用流程、帧绘制、文字编辑与
文字渲染分开；配置模板与配置类型分开；OCR 协议、序列化及回归测试分开。

## 资源生命周期

1. 普通截图仅初始化平台后端、图像缓冲与界面字体，不初始化 OCR
2. 系统字体与图标缓存在后台准备，不阻塞第一帧
3. 静态截图使用独立背景表面，动态界面只提交局部损坏区域
4. 无输入或后台工作时事件循环阻塞等待，不进行定时空转
5. Linux 图片剪贴板通过独立数据提供进程维持；Windows 将 PNG、DIBV5 与 Unicode 数据交给系统，关闭编辑器不会丢失内容
6. 贴图为独立进程，关闭截图覆盖层即释放编辑器缓冲
7. OCR 默认仅保留当前会话模型；Linux 可选后台模式有空闲退出期限，Windows 当前使用按需 CPU 引擎
8. 结果窗口使用独立进程，不加载 OCR 模型；首帧确认后退出截图进程，释放识别资源
9. 结果文档通过有长度上限的二进制帧传递；PNG 使用原始字节，不转为 JSON 像素数组
10. Linux 普通窗口使用共享内存缓冲、视区与分数缩放协议，Windows 使用原生消息循环与 DIB；两者空闲时阻塞
11. 激光只保存有容量限制的逻辑采样和缓存路径；完成笔迹期间轮询，过期后恢复阻塞等待
12. 捕获使用单线程异步调度，最多四个按需阻塞线程；工作线程空闲 250 ms 后退出，不保留默认十秒空闲线程
13. 放大镜标注只保存几何与比例，读取冻结原图的零复制视图；绘制和导出均不再次捕获桌面或初始化 OCR
14. 独立序号的下一编号属于会话历史，不写入工具偏好；中文字体随应用内置，不增加启动线程或识别模型依赖
15. 线条控制点随标注快照保存，当前定点和双击候选只属于编辑会话；两类样式弹层动画结束后恢复阻塞等待
16. 滤镜按冻结原图区域处理，单项像素缓存最多 4 MiB、总量最多 16 MiB；取消、删除和历史替换立即释放无效项
17. 历史窗口直接进入普通窗口运行时，不创建捕获异步运行时或识别模型；临时工作线程通过 eventfd 唤醒窗口，任务结束后退出
18. 历史预览逐行缩小，原图预览缓存最多 8 MiB、缩略图最多 4 MiB；不持久保存缩略图或 JSON 索引
19. Linux 辅助进程通过短期启动进程和独立会话交接，Windows 使用无控制台独立进程，不为长期剪贴板或 OCR 服务保留等待线程
20. 正文光标仅在实际键盘焦点属于编辑会话时保留半周期截止；失活、字段或菜单焦点和事务结束取消周期唤醒
21. 输入法候选只占独立显示预览，不进入正文或导出；隐藏候选光标取消周期唤醒，原生 leave 清理候选和目标身份
22. 上传只在主动请求时创建工作线程和通知管道；HTTP 直接交付 PNG，命令输入使用私有临时文件，任务结束回收线程、所属进程组与文件
23. 限时提示把稳定停留与淡入淡出分开；稳定画面只等待到期期限，不保留每帧计时器
24. 尺寸面板与字库在首次可用选区后创建；结束字段编辑后取消光标期限，静止面板不重新排版或提交帧
24. 读取桌面像素成功后才映射截图覆盖层，避免合成器把空白窗口收入截图；字体、图标和图层准备仍与捕获并行
25. 截图位置翻译复用一次性识别子进程，完成和取消时实际回收模型与全局并行线程池；区域像素和语言缓存属于当前截图会话

Windows 窗口和覆盖层在创建它们的界面线程消费事件，不跨线程移动 HWND。原生资源、
共享契约、同构建回归和未接入动作见 [Windows 工作流](windows-platform-workflow.md)。

## 文字预览与局部历史

`editor/text_session.rs` 管理独立预览和一次全局提交，不消耗预览标注编号。
`editor/text_session_format.rs` 分开保存当前文档字体和实际属性来源；后者来自工具默认值
或已有持久对象，普通刷新不覆盖已经局部撤销的文档字体。提交按原版元数据策略使用
最后实际属性来源，不把局部撤销写回工具偏好。

`features/text/frame.rs` 保存原始存储宽高、首个定点和当前预览字形缓存。
`features/text_storage/transform.rs` 按可见选框比例映射已有存储矩形，隐藏高度的真实变化
仍进入拖动历史。`tools/text/source_layout.rs` 按原图字号与定点字体度量计算选择边界，
当前字形缓存只决定绘制损坏范围。旋转绘制重新读取本次字形范围，避免缩放后裁切旧缓存。
`editor/text_session_layout.rs` 在重新编辑时采用可见宽度，提交时按实际段内换行收紧存储框；
`types/annotations/text_frame.rs` 排除缓存和等价缺省表示造成的历史变化。

`editor/text_session_history.rs` 用正文增量与前后字体两种操作组合局部历史。正文记录
保留输入时的字体；格式记录不复制正文。字符选区与光标属于局部编辑，格式动作清除选区，
格式撤销不恢复旧选区。控件字号入口排除导航、刷新和边界同值，独立透明度入口不追加
字体操作。字号字段 Return 先确认数值再提交文字，不进入截图导出动作。

`features/text/caret.rs` 保存正文光标可见相位与下一截止，当前原版完整周期为一秒，
每五百毫秒切换。键盘输入恢复可见相位但不推迟截止，正文鼠标按下和释放重新建立周期；
字符选区不隐藏光标。`app/text_caret.rs` 根据主表面实际焦点、字段和菜单状态同步计时，
只通知光标附近的动态损坏，不使持久标注图层失效。Wayland 键盘离开事件携带输出身份，
旧输出的延迟离开不能清除新输出已经获得的焦点。

`renderer/clipped.rs` 在部分损坏帧中复制当前背景到局部像素缓冲，再绘制动态预览、选区
边框与标注控制点，只覆盖该范围。它不把透明中间层再次合成到背景，避免字形和半透明
边缘反复叠加。完整像素范围直接复用原画布，不分配第二份整屏缓冲。文字绘制继承原始
像素相位，分数比例下局部帧与完整帧保持一致；光标使用正文颜色和透明度，字段光标
仍沿用控件自身样式。

`backend/wayland/overlay_popup.rs` 只管理原生菜单角色、缓冲和抓取；
`app/text_menu_surface.rs` 在角色关闭请求提交后交付文字命令。菜单和主画布分别路由输入，
输入事件携带菜单会话编号，过期事件不能作用于新菜单。新座席能力绑定后立即刷新连接，
即使没有画布重绘，也能收到新键盘映射，不依赖周期轮询。

## 原生文字输入法

`backend/wayland/text_input/` 按键盘 seat 可选绑定 text-input-v3，分开管理对象、
请求状态和事件分派。`state.rs` 保存对象级提交计数及最近一百二十八个字段边界，
不是每帧历史；旧 done 序号按原字段交付，不能改投接收时的新会话。离开后的
重新进入保留停用要求，先 disable 再 enable，兼容保留旧启用标志的合成器。

`app/text_input.rs` 按正文会话、原生焦点和输入拥有者分配目标。截图字体、颜色和
通用步进字段通过 `app/text_input/fields.rs` 替换正文目标；识别界面与菜单停用
正文协议，普通键盘失焦不等同于原生 leave。周边文字从已确认
编辑器读取，同段落使用段落范围，跨段落使用文档偏移；借用窗口比较避免闪烁时
复制文字，实际发送最多四千字节。光标矩形来自显示布局并转为表面逻辑坐标。

`editor/text_composition/` 完整验证候选及替换范围，再记录独立确认历史。候选使用
单独标注和编辑缓存，取消只清理显示，不恢复已经删除的选区。`renderer/text_preedit.rs`
按实际字体装饰度量绘制下划线，保留回退字体和分数比例。
`ui/text_field/` 分开维护单行历史、候选与验证，`renderer/line_edit.rs` 共用字形布局，
原生快照缓存逻辑光标；数字用途不带多行提示。普通独立窗口的输入法协议仍需接入，
不能复用正文预览编号作为协议目标。

私有回归同时核对协议坐标和实际文字位置。当前 niri 的菜单关闭后首次移动问题在原版
Qt 窗口中也能重现；不能通过补发移动或缩减完整场景来声称兼容，具体证据记录在
`docs/text-editing-compatibility.md`。

## 工具栏与贴图边界

`ui/toolbar/{items,layout,state,icons,draw}.rs` 分别承担按钮定义、几何、状态、原版 SVG 映射
和缓存绘制。`app/input/pointer_policy.rs` 统一区分左键控件确认、右键取消与中键范围切换，
`app/panels/toolbar.rs` 管理主副栏避让；绘制和命中使用同一组按钮矩形。

`features/pinned_image/{model,menu,actions,recognition}.rs` 分别保存原图像素、菜单规则、业务交接
和一次性推理进程，不创建图形窗口。`backend/wayland/pin/` 按运行时、协议、输入、菜单、绘制、分数缩放
和后台 OCR 分开。菜单采用 xdg-popup，置顶采用 layer-shell，普通窗口采用 xdg-toplevel。

`features/pinned_image/resize.rs` 与 `placement.rs` 分别保存八方向等比几何、固定边和输出选择、
可见交集。`backend/wayland/pin/geometry.rs` 只同步逻辑几何与协议，`gesture.rs` 负责左键分流，
`preview.rs` 保存无输入拖动表面，`output.rs` 管理首次定位与输出生命周期。
完整图像尺寸和可见表面尺寸分开；负坐标只改变绘制偏移，不重新缩放整张原图。
首次定位只执行一次，输出更新不重新套用初始选区或重置用户缩放。

拖动预览使用 layer-shell 且关闭键盘交互和输入区域；原主表面在整个手势期间保持位置、
输出及尺寸不变，继续接收隐式抓取的指针事件。释放后主表面按目标输出重建，提交图像后
再删除预览。预览有自己的配置、帧回调和分数比例，不启动计时器或常驻线程。
普通窗口缩放只记录待绘制尺寸，几何、视口和图像在同一次缓冲提交生效，不能提交新几何
但继续使用旧缓冲，否则合成器可能把旧尺寸反馈给仍在进行的系统缩放。

OCR 交接线程只在用户启用文字选择后创建，匿名文件通过已有有界协议把任务交给一次性
推理子进程；完成后回收进程并通过 calloop channel 通知窗口。模型与全局并行线程池不留在
贴图中，旋转或关闭可以取消任务；父进程结束时内核同时终止推理子进程。边框和文字选择
只绘制到显示缓冲，导出始终读取原图。贴图另存为暂时降低层级并关闭输入，再进入独立
窗口循环；共用窗口退出先析构表面再刷新连接，避免父进程继续运行时窗口残留。

完整界面差异与验证边界见 `docs/interface-compatibility.md`。

## 标注缓存

画笔按需备份当前手势触及的 64×64 原始像素块。提交前恢复像素，再完整绘制一次，
避免半透明笔迹重复叠加；取消和草稿撤销精确恢复原始像素，不重建其他标注。

`renderer/annotation_stack.rs` 管理已有标注的暂存绘制。拖动标注离开持久缓存后，
在损坏范围内按原始顺序插入草稿，只重绘相交标注，并把透明临时层合成一次。较新的
半透明标注和放大镜不会因拖动改变层序，也不会重复覆盖已经绘制的内容。

`features/annotation_selection/chrome.rs` 保存共用颜色、正文选框边界和删除按钮几何；
各工具继续提供独立控制点。`ui/annotation_selection/control_raster.rs` 在固定像素原点
绘制单个小型控制点，父缓冲只裁切最终合成范围，避免分数比例下重复圆角采样出现差异。
`delete_cache.rs` 保留约 24×24 逻辑像素的按钮缓存，位置或输出比例改变才重新生成。
`app/annotation_selection.rs` 同步旧新损坏范围并管理左键按下与释放；缓存不进入
持久标注、撤销或导出。删除先保存选中身份，红色按钮与键盘线条定点删除保持不同职责。
该状态管理单项删除；普通旋转角度和冻结画布边界由标注保存，旋转手势通过
`features/annotation_rotation/` 与 `editor/annotation_drag/generic.rs` 组合。
`renderer/rotation_tiles.rs` 固定物理像素网格，文字保留实际字体缓存并仅分配可见区域。
多选采用 `features/annotation_collection/selection.rs` 保存有序有效编号，单项索引由持久数组
投影。`geometry.rs` 组合生产可见边界，`transform.rs` 从手势初始快照映射各类成员。
`editor/annotation_collection/` 分离状态、手势和批量属性，`app/annotation_collection/`
负责按钮、焦点、修饰键和删除位置，`ui/annotation_collection/` 负责数量标题及通用属性布局。
`transform.rs` 在共同变换或批量改宽后同步放大镜轴心，再刷新派生正文边界；轴心按照
加宽并裁切后的未旋转联合框计算，损坏登记与缓存裁切必须使用同步后的模型。

共同拖动保留持久数组顺序，`renderer/collection_stack.rs` 在损坏区域合成全部相交对象，
结束时恢复缓存。单项与群组历史保存编号集合，颜色属性区分首次输入立即捕获与已提交
事务，面板关闭不重复提交。旋转和未旋转线条统一通过 `renderer/line_handles.rs` 绘制骨架点；
端点状态不额外改变样式，内部控制点选中时按原版扩大半径。完整原生状态及发布边界见
[annotation-collection-compatibility.md](annotation-collection-compatibility.md)。

`features/canvas_navigation/` 分离双精度视图状态、原图投影、滚轮职责及滚动条几何。
`editor/file_canvas.rs` 使用同一模型生成完整窗口初始适配，并继续独立保存原图像素。
`editor/canvas_navigation/` 保存共享工作区视图、输入反解、屏幕控制点容差与每输出背景身份。
`app/canvas_navigation/` 处理中键、滚轮、Ctrl 边沿、九百毫秒提示和损坏映射。
`renderer/canvas_navigation.rs` 从冻结原图重采样背景，并直接合成当前矢量标注及完整草稿；
现有背景缓冲缓存当前视图，局部重绘复用相同像素，不放大低分辨率预览或持久标注缓存。
UI 指针保留窗口坐标，工具读取稳定画布坐标；视图变化不改写标注、裁切或撤销记录。
删除按钮、尺寸 HUD、工具栏、输入法候选及翻译浮条按视图重新定位。当前依据和边界见
[canvas-navigation-compatibility.md](canvas-navigation-compatibility.md)。

## 测量模块

```text
features/ruler/
  model.rs          测量模式、定点、原图像素与绘制比例
  geometry.rs       长度、面积、夹角、旋转与边界
  drag.rs           控制点、整体移动和等比调整
  angle_draft.rs    三点角度定点状态
  ticks.rs          有限数量的 1、2、5 刻度
  labels.rs         完整测量说明、窄范围分行和标签避让
  body_draw.rs      固定像素原点的按需分块绘制
editor/ruler_session.rs  工具进入、模式切换与取消规则
editor/ruler_labels.rs   悬停与说明缓存同步
renderer/ruler_overlay.rs 只在当前损坏区域合成说明和临时悬停
```

测量比例与绘制比例分开保存。导出改变坐标和绘制尺寸，但宽、高、对角线、面积与夹角
保持原图单位。正文和说明使用固定像素原点，分数缩放、旋转和局部重绘不会改变采样相位。
悬停十字线、原图坐标和颜色只用于编辑屏幕，不进入导出图像。

## 双框放大镜

```text
features/magnifier/
  model.rs          双框中心、宽高、倍率、框形和旋转轴心
  geometry.rs       完整源框、原图边界、连接线与旋转坐标
  drag.rs           两组控制点、独立移动和固定倍率联动缩放
  draw.rs           双线性取样、形状裁切、向内边框和固定像素分块
renderer/frozen_image.rs      本地原图、多输出捕获及导出坐标的零复制视图
renderer/magnifier_handles.rs 圆形、矩形双框与旋转控制点
tools/magnifier.rs            创建、提交、取消和已有框拖动
editor/magnifier_edits.rs     倍率、框形属性与单次撤销事务
cli/capture_runtime.rs       捕获工作线程的空闲释放策略
```

光标取色放大镜仍使用 `ui/magnifier/`，不与持久双框标注共用状态。两框保持固定倍率，
各自移动时另一框中心不变；任一框缩放后反算另一框尺寸，再约束完整几何到原图范围。
矩形各八个控制点，圆形各四个，旋转把手独立命中；Ctrl 保持矩形长宽比，Shift 吸附
四十五度旋转。旋转保留记录的原始取样区域，并同时旋转双框正文。

冻结图像视图不包含先前标注，保留原始分辨率与预乘透明度。单个临时像素块最多
1024×1024，即 4 MiB；对应裁切遮罩最多 1 MiB，使用后立即释放。倍率字段固定保留
两位小数，属性输入、选择、颜色及按钮均复用现有紧凑组件。

## 独立序号

```text
features/number/
  model.rs                  七种样式、序号几何数据和有上限的会话计数
  format.rs                 字母、罗马数字、天干和中文标签转换
  geometry.rs               圆框、指向尾部、旋转边界和原图比例
  drag.rs                   圆心、尖端、外框缩放及旋转控制点
  draw.rs                   固定像素原点的按需分块绘制
tools/number.rs             单击、拖动、提交与取消，不承担计数持久化
editor/history_snapshot.rs  标注和下一编号共用撤销快照
editor/number_edits.rs      样式、笔宽和重置的独立历史事务
app/panels/number.rs        复用属性输入、选择弹层和重置组件
app/input/ui_hit.rs         属性、弹层和工具控制点的优先命中
renderer/number_handles.rs  外框、圆心、尖端和旋转把手
renderer/number_text.rs     屏幕与原图共用的裁切标签绘制
assets/fonts/MarkShotNumerals.otf  中文及天干的二十三字内置子集
```

独立序号与旧编号箭头分别计数、分别保存笔宽。提交后推进编号，取消不消耗编号，删除
不填补空缺；重置不删除标注，撤销同时恢复几何与计数。新会话从一开始，工具偏好仅保存
样式、颜色和笔宽。计数耗尽后拒绝创建，不溢出或重复使用编号。

序号和文字按固定像素原点分块，旋转、分数缩放及局部重绘保持采样相位一致。中文字体
子集为 4616 字节，字体名称独立，使用 OFL 1.1 许可证；重建脚本只属于开发工具，应用
不执行 Python，也不等待系统字体扫描。

## 箭头与线条

```text
features/line/
  model.rs             两端点、有限控制点、独立线型及原图比例
  geometry.rs          二次曲线、连续多点路径、插入投影和方向吸附
  polyline.rs          弧长、切线、截取及有上限的均匀重采样
  drag.rs              定点、外框、旋转和原始快照变换
  draw.rs              虚线切分与单次填充的闭合描边轮廓
features/arrow/
  style.rs             四种头部及稳定配置编号
  metrics.rs           按长度与笔宽约束头部尺寸
  fletched.rs          沿中心线计算实心箭身和头部
  path.rs              实线、虚线、点线与单向、双向头部组合
  draw.rs              屏幕与导出共用的正文绘制
tools/line.rs          箭头和直线共用创建、提交与草稿取消
editor/line_session.rs 当前定点和双击状态，不进入偏好或历史
editor/line_edits.rs   样式、控制点插入和删除的独立撤销事务
app/panels/line.rs     共用头部和线型弹层的输入与生命周期
ui/line_properties.rs 实际路径预览与紧凑选择网格
ui/settings_panel/action_icon.rs 属性按钮预览与展开标记
renderer/line_handles.rs 外框、端点、控制点及当前定点高亮
renderer/annotations/handles_outline.rs 精确外框与已有工具的默认外框绘制
renderer/fixed_path.rs 固定像素原点的按需分块填充
```

骨架始终保留索引零、一起止点，索引二起保存控制点。单控制点是真正的二次曲线控制
参数，不是必须经过的折线点；多控制点按相邻中点组合连续二次曲线。已有标注的样式
修改不写入工具默认值，箭头头部、箭身线型和普通直线线型分别持久化。

本次实现增加容量保护：最多 128 个控制点，曲线细分最多 8192 点，均匀重采样最多
512 段；这些上限不是原版常量。临时像素块最多 1024×1024，即 4 MiB，按路径完整边界
确定像素原点。箭身与头部合为一次填充，避免半透明接缝重复叠加；局部重绘和原图缩放
使用相同相位。

旋转把手位于全部骨架点上方，避免与外部控制点重叠。精确外框不重复添加命中留白，
编辑损坏区域包含统一描边、阴影和全部定点，取消选择不会留下角标或控制点残影。

## 原图滤镜

```text
crates/shot-image/src/
  blur.rs                  三次盒式模糊及消费输入缓冲的接口
  pixelate.rs              区域内像素网格的预乘平均
  filters.rs               灰度、反色、提亮与强度换算
features/filter/
  model.rs                 五种稳定模式、原图强度、旋转与绘制比例
  geometry.rs              原图网格、向外对齐范围及旋转坐标映射
  raster.rs                原图核心块、算法留量和可选完整区域缓存
  draw.rs                  固定输出相位、遮罩、缩放与半透明合成
  drag.rs                  八个控制点、移动、等比缩放和旋转吸附
tools/filter.rs            原图最小尺寸、区域提交及草稿取消
editor/filter_cache.rs     四 MiB 单项与十六 MiB 总量的像素缓存
editor/filter_edits.rs     模式隔离、强度同步和单次属性事务
renderer/filter_handles.rs 旋转外框与控制点绘制
ui/filter_properties.rs    五模式统一弹层与真实算法小型预览
app/panels/filter.rs       弹层输入、互斥关闭、动画和生命周期
```

滤镜读取 `FrozenImage`，不读取已经绘制标注的图层。原图核心块为 512×512，插值读取
一像素留量；模糊额外读取两倍半径，最大为 96 个原图像素。超过单项缓存上限时按块处理，
不创建工作区级模糊副本。旋转不重算原图效果，移动、模式或强度变化才更新取样缓存。

每个输出像素只归属一个原图核心块，覆盖层只合成一次。`Painter` 记录原始画布的小数
像素相位，子缓冲继承该相位；局部视口只跳过不可见块，不改变完整正文的遮罩扫描起点。
这同时处理非整数原图导出原点、分数缩放、旋转和局部重绘。

灰度、反色和提亮保持取样像素的 alpha；像素化与模糊直接处理预乘通道。效果按标注
透明度进行 `SourceOver` 合成，最终半透明区域可能更不透明，不能承诺滤镜区域的 alpha
始终与原图相同。未修改区域保持原始颜色与透明度。

模式修改只影响选中区域，强度修改沿用原版同步后续默认值的规则。偏好保存 `filter_mode`，
只读兼容旧 `pixelate` 开关，未知模式单独忽略；原图强度不随导出比例变化。`M`、`X`
进入滤镜，光标取色放大镜使用无冲突的 `Shift+M`。

## 激光模块

```text
features/laser_trace/
  state.rs                 草稿、完成笔迹、时间与容量限制
  draw.rs                  裁切三层描边，避免局部重绘叠加透明度
tools/laser.rs             左键手势、工具退出与损坏区域通知
renderer/stroke_path.rs    画笔、荧光笔和激光共用平滑路径计算
```

临时笔迹独立于 `Annotation` 和撤销快照，不烘焙进持久标注图层。导出流程只读取持久标注，
与原版即时导出行为一致。完成路径在消散期间复用；单笔迹、总点数和完成数量均有上限，
草稿在指针输入时更新，不因按住鼠标而进行空闲轮询。

## OCR 结果模块

```text
backend/wayland/window/     普通窗口运行时、协议分发与输入设备生命周期
ui/text_editor/            多行编辑、剪贴板请求、增量撤销与裁切绘制
renderer/editor_text.rs    标注文字和输入控件共用的字形、选择与光标绘制
features/ocr_result/
  document.rs              原文、文字框与未经标注的原图快照
  transport.rs             有界结果帧、辅助进程与首帧就绪确认
  table.rs                 按文字框位置推断行列，保留缺失单元格
  table_geometry.rs        动态行高、累计坐标与可见行定位
  export.rs                TSV、CSV、Markdown 与 HTML 字段转义
  table_view.rs            按需初始化单元格编辑器与滚动表格
  layout.rs                紧凑布局、窄窗口适配与控件命中
  window.rs                页面、文字编辑、复制与窗口动作
  draw.rs                  标题、原图、菜单与当前页面绘制
```

文字页与表格页保存各自的编辑状态，切换原图不会重新创建文字编辑器。文字修改采用增量
历史，不为每次按键复制整份文档。表格只绘制可见行列，多行单元格采用有上限的动态行高。
ASCII 结果仅使用随应用提供的字体；显示或粘贴其他文字时再补充系统字体。

## 截图历史模块

```text
features/capture_history.rs   完成图片自动记录与独立窗口入口
features/capture_history/
  identity.rs                UTC 毫秒名称、随机标识与顺序约束
  png_metadata.rs            PNG 结构、CRC、比例与可选文字索引
  storage.rs                 固定目录描述符、私有文件与链接防护
  store.rs                   记录查询、原图读取、删除和数量容量清理
  preview.rs                 有界逐行缩小与透明度处理
  model.rs                   搜索、选择及受预算约束的预览缓存
  jobs.rs                    临时后台任务、取消与 eventfd 唤醒
  window.rs                  普通窗口生命周期和后台结果接收
  input.rs / actions.rs      输入分派与窗口动作
  layout.rs / draw.rs         紧凑布局、列表与原图预览绘制
  transport.rs               首帧交接和匿名原图编辑快照
config/history.rs            策略读取和保留其他配置的原子更新
ui/file_picker/              共用保存对话框、目录导航与覆盖身份校验
ui/fonts.rs / single_line.rs 按需字体与单行输入
backend/wayland/window/wake.rs 普通窗口的可选 eventfd 通知
utils/helper_process.rs      独立会话启动、标准流与执行错误传递
utils/fs_lock.rs             有期限的文件锁等待
```

自动记录复用导出 PNG 的 IDAT，只更新历史元数据。目录首次创建使用 0700，记录使用
0600；通过已打开目录描述符读写，拒绝末级目录链接、非法文件名、符号链接和多重硬链接。
写入和清理串行执行，超大新图在写入前失败，不删除已有记录。

另存为在打开对话框时冻结原图字节，源记录随后删除也能保存。覆盖现有文件前核对文件
身份，确认默认取消。目录新建只刷新当前列表并恢复原文件名，目录行采用单击导航。

重新编辑传递有长度限制的 PNG，子进程写入匿名 0600 文件，并通过 `/proc/self/fd`
进入原图编辑器；文件生命周期随子进程结束，不留下命名临时截图。长图仍按原始像素
导出，高度超过 16384 不直接拒绝，解码受 64 Mi 像素和 256 MiB 预算约束。

历史入口检查旧忙碌标志、未提交草稿和已有标注拖动。画笔等旧工具仅保存草稿，因此不能
只依赖按下标志。首帧确认后取消原截图会话，不记录取消图片。

## 持续画布导航模块

```text
features/canvas_navigation/  原图投影、中心约束、滚轮职责与整数滚动条模型
editor/canvas_navigation/    工作区共享视图、稳定坐标映射、控制点命中与输出背景身份
app/canvas_navigation/       滚轮、中键、Ctrl 边沿、滚动条输入、损坏通知与倍率提示期限
renderer/frame_scene.rs     恒等视图缓存场景
renderer/canvas_navigation.rs  冻结原图重采样、完整标注及草稿合成
ui/canvas_navigation/        屏幕滚动条与限时倍率气泡
```

持久标注、原图裁切与撤销使用稳定画布坐标；视图变化只改变输入映射及渲染投影。
屏幕指针继续服务界面控件，各输出共用工作区视图，再组合自己的原点与设备比例。
非恒等视图按原始图层顺序绘制完整矢量正文和草稿，冻结原图采样到已有背景缓冲。
局部重绘恢复相同像素网格，避免反复叠加透明像素。

已有文字的独立编辑预览替代其持久原件。R4 将原件编号显式传入导航合成入口并过滤
正文迭代，不修改标注数组或历史；否则原件和预览会在非恒等视图中重复绘制。
倍率提示到期清除旧区域，随后不保留定时唤醒。候选指纹及验证范围见
[canvas-navigation-compatibility.md](canvas-navigation-compatibility.md)。

`features/annotation_selection/rotation.rs` 读取通用及专用旋转连接点。界面绘制和导航
命中共用反向倍率，连接起点随正文投影，外移向量保持窗口单位。普通标注与群组为
26 逻辑像素；专用连接点暂时保留既有间隔。选择工具的专用回归和原生旋转流程限于
各自记录的范围，完整原版样式单位、尺子独立入口及属性栏避让继续对照。

## 兼容策略

保留原版常用命令行参数和截图 JSON 输出约定。旧配置应显式迁移到新的 TOML 文件，
不覆盖原 JSON 配置。平台限制必须报告具体原因，不用空实现冒充已支持能力。
