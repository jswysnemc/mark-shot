# OCR 原图页面对照

## 原版依据

`tests/fixtures/qt-ocr-preview-reference/main.cpp` 只读链接原版 `OcrSourcePreview`，
直接向实际组件发送滚轮、鼠标按下、移动、释放和双击事件。探针只在编译自身头文件时
公开状态读取，不改变原版实现、对象文件或配置。

冻结记录 `tests/fixtures/qt-ocr-source-preview.json` 包含八组场景，比较适配尺寸、
指针中心缩放、左右键和中键职责、拖动边界、双击复位、缩放上限、窗口调整及纯像素滚动。
原图菜单也来自实际组件，两行顺序为 Edit、Pin image，逻辑尺寸为 168×66。
探针使用临时配置和离屏平台，不连接用户座席。

## 原生实现

- 原图使用四个逻辑像素的内边距；适配、中心和移动范围沿用 Qt 的整数尺寸与包含右下边界的矩形规则。
- 滚轮围绕指针按 1.12 倍缩放，范围为适配大小至六倍；放大时至少保留 48 个逻辑像素可见。
- 左键只在放大时平移；右键和中键不启动平移，双击恢复适配大小与居中位置。
- 放大缓冲始终只覆盖可见视区，不随倍率扩大；原图透明像素透出父背景，不添加棋盘背景。
- 原图右键使用独立原生菜单。菜单销毁后，编辑和贴图交接未经缩放、平移或正文修改的 PNG。
- 标题栏 Copy 在所有页面都复制原文。表格拥有独立复制按钮和原生格式菜单，不再复用标题栏按钮。
- 原图键盘字符不启动编辑器，也不触发系统字库扫描。
- 贴图纯点击不创建拖动预览；实际位移后才更新预览，未移动的释放不重配表面。
- 按钮释放打开菜单时，抓取使用本次真实按下的序号，与 Qt 一致；同座席、同表面、同按钮或键码的配对只能消费一次。
- 菜单打开、失焦和设备移除清除未完成配对；合成器移动抓取截获释放后，下一次真实按下替换旧记录，不沿用旧序号。

## 文件职责

| 文件 | 职责 |
| --- | --- |
| `src/features/ocr_result/source_preview.rs` | Qt 整数几何、缩放与平移状态 |
| `src/features/ocr_result/source_preview_draw.rs` | 有界视区缓冲、透明度与裁切 |
| `src/features/ocr_result/source_image_menu.rs` | 原图菜单与匿名图像交接 |
| `src/features/ocr_result/table_menu.rs` | 独立表格格式菜单与导出 |
| `src/features/ocr_result/source_menu.rs` | 共用原生菜单生命周期、输入和交付 |
| `src/backend/wayland/pin/gesture.rs` | 贴图实际移动与纯点击的区分 |
| `src/backend/wayland/window/input_grab.rs` | 按下到释放的一次性序号配对、来源校验与失效回收 |
| `scripts/wayland-ocr-source-preview.py` | 私有 niri/Sway 的实际像素、菜单与交接回归 |
| `scripts/wayland_ocr_preview_checks.py` | 同时比较尺寸、颜色和透明度 |

## 证据与边界

实际 Qt 探针重复采集后与冻结记录一致，Rust 测试逐事件回放这些状态。
预览版 6 的本地下载产物在新增回归中复现标题栏 Copy 提供 PNG 而非原文，失败证据保留。
像素比较器曾只检查 RGBA 差异图的透明通道，颜色变化没有进入断言；新增用例先复现问题，
再改为比较全部通道，不以降低像素精度解决测试失败。
候选取消对照也使用两张具有相同菜单焦点的真实截图，包含原光标、完整候选、新光标与
下划线；不把原光标失焦后的正常消失误判为候选残留，也不屏蔽光标像素。

## niri 弹层输入边界

本机私有 niri 会话在键盘关闭菜单后丢失首次绝对指针移动，后续按下使用旧坐标。
原图回归中的标题栏复制因此失败；单元格输入法回归中的重新点击也受到影响。
延长等待、更换为私有 X11 指针或 Wayland 虚拟键盘、提前发送销毁请求均没有消除问题。
这些失败保持独立证据，不通过补发移动或强制更换表面角色使测试显示通过。

`target/p7-qt-popup-reference.cpp` 只读链接实际 `OcrTablePane` 和 `ContextMenu`，
在同一类私有会话复现相同行为。Qt 协议记录 Space 按下序号为 14、释放序号为 15，
菜单抓取使用 14；菜单关闭后的首次源复制点击仍取得表格 TSV，而不是 `base`。
证据位于 `target/ocr-preview-qt-niri-r1/result.json` 和对应协议日志。
该探针的外围宿主及源复制按钮用于协议诊断，不代替完整原版 OCR 窗口对照。

当前读取的上游 [PopupPointerGrab 实现](https://github.com/Smithay/smithay/blob/master/src/desktop/wayland/popup/grab.rs)
在抓取结束后的 `motion` 分支解除抓取后直接返回，没有转发本次移动，符合上述真实日志。
读取的上游源码不是本机安装二进制的完整构建凭证，因此以实际 niri 与 Qt 对照结果界定本轮限制。

## 未完成范围

本轮仅进行本地重构和验证，不提交、推送或发布到 GitHub。
这些结果不代表整个 OCR 窗口已经与 Qt 一致。单行标题栏基础布局已接入，完整范围见
`docs/ocr-titlebar-compatibility.md`；更多父子菜单已接入，范围见
`docs/ocr-more-menu-compatibility.md`。全部图标、翻译、
结果窗口置顶、完整焦点遍历、原图工具提示、表格完整几何、主题及逐像素
对照仍需完成。纯像素滚动只通过原图状态接口对照，真实触摸板事件的协议分类尚未全面验证。
多设备组合按键和完整合成器、输出矩阵也不据本轮回归宣称完成。
原文与表格的独立复制期限现已接入，完整窗口采集及该项边界见 `docs/ocr-titlebar-compatibility.md`。
