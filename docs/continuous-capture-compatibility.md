# 持续捕获会话与原生取帧

持续捕获使用 `src/backend/capture_session.rs` 的 `CaptureSession` 接口，提供
`next_frame()` 和幂等 `close()`。捕获链进入时验证候选首帧，失败后关闭候选，成功后
保留首帧并固定该会话；长截图期间不重复选择后端。完整控制与原图交接见
[长截图工作流](scroll-capture-compatibility.md)。

## 会话与资源

Portal 在一个会话内完成一次 CreateSession、SelectSources、Start 和 OpenPipeWireRemote。
每个输出持有一个 PipeWire 流，每个流使用单个原生线程。只有明确的取帧请求才激活流，
收到图像并归还缓冲后停用。暂停不复制整帧，也不周期轮询。取消当前帧请求不关闭授权，
后续请求可以使用同一会话恢复。退出时关闭流、回收线程并关闭 Portal 会话。

来源响应必须成功；来源取消与拒绝都在 Start 前结束，并关闭会话。旧恢复令牌失败后
先关闭旧会话再重新选择。Portal 来源必须完整且唯一地对应输出；逻辑裁切使用 Wayland
输出尺寸，不能把物理视频尺寸当作逻辑尺寸。像素读取检查格式、偏移、行距、映射范围和
尺寸，支持负行距，并返回紧密 RGBA。

image-copy 与 wlr-screencopy 在持续会话中使用独立 Wayland 连接，按输出注册表全局
标识重新绑定原目标。`src/backend/wayland/capture/isolation.rs` 持有连接、输出与事件
队列，并在关闭时释放。该连接仅用于后台捕获；覆盖层保留原连接。KWin 使用其 D-Bus
捕获适配器。原生 shm 请求通过取消通知和资源所有权在成功、失败与取消路径回收缓冲
和协议对象。

## 后台任务与已修正问题

`src/app/scroll/task.rs` 使用单个后台任务等待帧，以通知管道唤醒覆盖层。方向切换、
区域拖动与退出取消等待，请求编号保证过期结果不改变新状态。暂停停止新请求，并接收
已经在途的帧；复制、保存与标注通过 `src/app/scroll/completion.rs` 等待在途帧完成，
等待期间事件循环继续处理输入，取消仍立即结束。保存取消后恢复发起保存前的状态。
后台任务结束时交回捕获会话，由运行循环统一异步关闭。捕获前等待隐藏或移动后的表面
呈现，避免把控制面板拼入原图。布局变化后重新计算按钮悬停。

初版后台任务与覆盖层共用 Wayland 连接，两个读取等待可能相互阻塞。私有原生回归
曾导出 280×686 图像，而实际页面条带组成 280×712；其中第 520 行开始跳过了 26 行。
第二次运行只拼接一次位移。最小复现移除主题、保存与标注操作，仅保留静止 1.5 秒后
连续滚动和复制，得到 280×502，而期望为 280×664。移除静止等待后同场景通过。

持续原生会话改用独立连接后，同一静止恢复场景及完整工作流通过全部原始 RGB 像素
比较。回归位于 `scripts/wayland-scroll-capture.py` 的 `idle_resume()`。错误记录保留于
`target/continuous-scroll-scale1-r1/`、`target/continuous-scroll-scale1-r2/` 和
`target/continuous-scroll-minimal-before-r2/`；错误候选为
`target/continuous-capture-candidate-r1/mark-shot`，不替换这些记录。

扩展到 wlr-screencopy 1.5 倍时，另一次运行导出 420×924，而期望为 420×996；
保存的全部页面帧对应正确位移，导出图完整保留前 924 行，只缺末尾 72 行。重复运行
通过，证明该问题涉及时序。旧暂停操作丢弃在途请求，旧完成操作直接结束任务，可能
在后台已经捕获末段但界面尚未接收结果时丢失该帧。完成事务改为上述非阻塞收尾方式。
这次失败记录位于 `target/continuous-scroll-wlr-scale15-r1/`。

## 原生证据

当前候选为 `target/continuous-capture-candidate-r3/mark-shot`，SHA-256：
`5e7b9d55dc38d09917e8b359a7fc9ccb64d41ff9f36c0f819e582d05f6a7c3a0`。

| 协议与比例 | 结果入口 |
| --- | --- |
| image-copy，一倍 | `target/continuous-final-image-scale1-r2/checks/result.json` |
| image-copy，1.5 倍 | `target/continuous-final-image-scale15-r2/checks/result.json` |
| wlr-screencopy，一倍 | `target/continuous-final-wlr-scale1-r2/checks/result.json` |
| wlr-screencopy，1.5 倍 | `target/continuous-final-wlr-scale15-r2/checks/result.json` |

上述矩阵使用私有 Sway、原生可追溯滚动页面和独立原始 wlr-screencopy 读帧客户端。
静止恢复、纵向与横向增长、大选区、暂停、按钮左右键职责、预览浏览、保存取消、
标注交接、双主题恢复、实际保存与取消恢复均通过。1.5 倍的静止恢复图为 420×996，
纵向图为 420×1068，横向图为 996×420，大选区图为 1256×920。原图像素比较不设置
误差容限，暂停期间捕获请求与缓冲提交保持稳定。

主题回归等待实际画面改变和完整恢复，不以快捷键发送完成代替像素呈现。原始即时
截图曾在按键已经到达但新画面尚未提交时错误报告主题切换失败，现场保留于
`target/continuous-final-image-scale15-r1/` 和对应的 wlr-screencopy 两组目录。

最终汇总清单为 `target/verification-continuous-capture-r1/result.json`。默认工作区
1228 项通过，全特性 1229 项通过，各忽略 19 项；严格 Clippy、格式与相关 Python
检查通过。1616 项源码、资源、脚本和构建输入前后一致，最终候选无 Qt 动态依赖。
该候选尚未替换 `target/release/mark-shot` 或系统入口。

Portal 最终 release 验证程序为 `target/continuous-portal-candidate-r1/portal-session`，
指纹为 `9d20ba9263fc58efc1cb761f764469ffb54338533a9f42de00ae9581cadec6b5`，证据位于
`target/portal-session-final-r2/checks/result.json`。独立私有 D-Bus Portal 服务、真实
私有 PipeWire 与 GStreamer 红色视频源驱动生产 Portal 后端。多次取帧只授权一次，
64×32 缓冲恰为 8192 字节，全部 RGBA 像素均为 `[255, 0, 0, 255]`；取帧时线程数
保持 3，关闭后为 2，暂停期间流线程 CPU ticks
保持 0。矩阵还覆盖未交付帧的请求取消后复用、失效恢复令牌重试、来源取消与来源拒绝。

## 仍需覆盖

无 Layer Shell 的普通窗口回退已接入并通过私有协议缺失验证，最新候选及原生矩阵见
[普通窗口长截图](layerless-scroll-compatibility.md)。实际多输出与 GNOME/KDE 等其他
合成器完整运行矩阵仍未完成。私有 Portal 服务证明协议、像素和资源生命周期，不证明全部真实桌面的
授权界面、来源元数据或集成行为。控制面板的全部 Qt 光栅细节也未完成逐像素对照。
整体重写目标保持进行中。
