# Mark Shot 录制恢复调研

更新：2026-11-12；原调研日期：2026-11-11。基线：Rust `e8c8966`；Qt `934d8f3`。本轮覆盖 Linux Wayland/X11、Windows，暂不实施 macOS。性能数字只有标注实测环境、提交和方法时才表示测量结果。

## 结论与实施选择

高性能首先取决于原始像素经过哪些内存，而不是实现语言。主路径应保持 GPU 表面直到编码，避免 GPU → CPU BGRA → CPU NV12 → GPU 的整帧往返。控制消息可以跨进程，原始视频帧不应经过 JSON、PNG 或 stdin rawvideo 管道。

录制业务独立于截图：主程序管理选区、设置、暂停和输出，录制任务拥有设备与编码器。停止后整体释放。Windows 采用 WGC/D3D11 与 Media Foundation，使用系统 H.264/AAC 能力；Linux 使用进程内协议采集和私有共享媒体库，直接 Wayland 兼容时使用 DMA-BUF/VAAPI，其余提供共享内存与软件编码回退。媒体库单独记录许可证与来源。GIF 放在录制结束后的转换阶段，避免实时量化挤占采集线程。具体后端可用性必须通过实际启动确认，失败保留原因，不能用编码器枚举代替运行成功。

小体积分别衡量主程序、录制组件、压缩下载、解压安装和系统依赖。系统已有图形驱动和编解码框架时可减少分发量；动态链接只转移依赖，不会让依赖消失。完整 libobs 的场景、直播、插件和预览能力超出本产品需求，不作为嵌入依赖。

## 源码核查

已实际下载以下源码，固定到提交，未用 README 性能宣传代替测量：

| 项目 | 提交 | 作用 |
|---|---|---|
| wf-recorder | `c5de47440e8e81c92befb696cb69819cdb3bfe8a` | damage、VFR、DMA-BUF/VAAPI、缓冲池 |
| wl-screenrec | `32f7810f3ef7543f4460c65851687435f6a24be1` | 编码表面反向导出给 compositor、限帧 |
| OBS Studio | `6e22cced38142703d5fe3bbb80f3f97ab50d4a08` | 跨平台纹理、同步、音频时钟、封装 |
| GPU Screen Recorder | `f97d1e392816503282d20a0804c3ce3fd2943767` | Portal/X11 GPU 路径、独立组件与控制 |

详细依据见 [开源源码比较](recording-source-audit.md)、[OBS 审计](recording/obs-audit.md)、[Qt 与 Rust 差距](recording/qt-audit.md)、[平台 API](recording/platforms.md)、[打包与画质](recording/packaging.md)。这些附录记录调研时状态，实施后的行为以录制使用说明和验证记录为准。

## 为什么 wf-recorder 看起来很省资源

1. 默认以 `copy_with_damage` 等待画面变化，静态桌面不持续生成相同视频帧；默认 VFR。`frame_handle_damage` 并不提取矩形编码，因此不是“只编码变化区域”。指定 `-r` 是输出 fps filter 补/丢帧，不能当作采集前限帧优化。
2. 默认编码器其实是 libx264，参数 superfast、zerolatency、CRF 20。superfast 减少编码计算，但压缩效率和画质/码率关系有代价。
3. 只有 VAAPI、合适格式/方向、同 GPU、支持导入时，GBM DMA-BUF 才能映射进 VAAPI；出现 hwupload 的路径仍可能 CPU 映射和上传。
4. 功能窄，没有 OBS 的通用场景合成、滤镜、预览和多源混音。不能由此推断 OBS 实现低效。
5. 复用池从 2 扩张到最多 16 个槽，但不是按内存预算控制。4K BGRA 16 张理论约 506 MiB，尚未包含编码器。源码还有微秒/毫秒轮询等待，并非完全无唤醒。

上述结论来自固定源码，链接见[源码比较](recording-source-audit.md)。低 CPU 不等于低总功耗，硬编码也占用 GPU、显存和带宽。

## 当前 Mark Shot 的真实差距

Qt 已有有界队列、提前限帧、缓冲复用、暂停时钟和录制 UI，恢复时不能将它们重新包装为新增性能收益。核心问题是 Qt WGC 的 STAGING/Map/memcpy 和 PipeWire 的 glReadPixels，随后 VAAPI/QSV 再上传。Rust 的截图/长截图流则每次请求一帧 CPU RGBA，交付后停流；不适合直接承担实时录像。

Rust 已有 Linux X11、Wayland、Windows；旧架构文档排除 X11 的描述已过时。macOS 尚无完整平台工厂，本轮按用户要求排除。可复用的是配置、原生窗口、选区、托盘、通知和文件命名；录制帧与 GPU 生命周期另建契约。[Qt/Rust 源码清单](recording/qt-audit.md)

## 平台边界

Linux 通用 Wayland 通过 Portal 授权，再连接 PipeWire。DMA-BUF 需要格式、modifier、plane、设备与同步共同匹配，失败时应明确回退；mmap 不是导入 GPU 的替代。wlr-screencopy/ext-image-copy 只适用于支持协议的 compositor。X11 另走对应路径。默认不申请 KMS 管理权限，不安装驱动或休眠配置。[Portal ScreenCast](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html)、[PipeWire DMA-BUF](https://docs.pipewire.org/1.4/page_dma_buf.html)

Windows WGC 提供纹理，D3D11 保留自有表面并裁剪/转换后交给编码器。GPU CopyResource 可以是合理的所有权边界，目标是避免 CPU 整帧读回，并非宣称绝对零复制。Media Foundation 打开 hardware transforms 不保证实际选中硬编码，需要记录实际能力与启动失败。WASAPI loopback 和麦克风是不同源，设备时间映射到视频时钟，暂停从两者时间轴扣除。HDR 必须有明确转换，不能把八位拷贝称作 HDR 支持。[微软 API 证据](recording/platforms.md)

## 时间、资源与失败契约

- 输入保存时间戳及其时域；使用单调时钟，不依赖墙钟。拒绝倒退时间，避免暂停后重放旧缓存。
- 采集回调不保留无界 raw frame 队列。编码慢时合并/丢弃过期画面，保留最新帧；不能丢掉最后变化后长期静止的最终画面。
- 设备资源由所属线程和明确租约管理。编码器使用完前不得覆写纹理，关闭时停止回调、排空编码、写尾部，再销毁设备。
- 正常停止与取消启动不同。失败保留可恢复工作文件，给出后端、阶段和原始原因。磁盘满、设备移除、权限撤回、编码器初始化失败要可观察。
- 容器选择影响恢复：faststart 仅移动索引，不防中断。MKV、fragmented/hybrid MP4 提高恢复机会，也不保证任意断电零丢失。[FFmpeg formats](https://ffmpeg.org/ffmpeg-formats.html)、[OBS hybrid MP4](https://obsproject.com/kb/hybrid-mp4)

## 画质与资源预算

兼容基线是 SDR H.264、8 位 4:2:0、AAC，保留原生分辨率。高帧率提高运动清晰度也增加像素吞吐；静态文档可以 VFR。4:2:0 的彩色细字损失不能靠码率完全恢复。编码器 preset、lookahead、B 帧及 multipass 会改变质量、延迟、内存和功耗，不能用同 CRF/CQ 数值跨编码器宣称等质量。[NVIDIA 编码指南](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-video-encoder-api-prog-guide/index.html)

| 裸帧 | 1080p | 4K |
|---|---:|---:|
| BGRA | 7.91 MiB | 31.64 MiB |
| NV12 | 2.97 MiB | 11.87 MiB |

4K60 BGRA 单次数据量约 1.991 GB/s；memcpy 读写合计约 3.981 GB/s。这里只是算术，未计 GPU 总线、颜色转换、缓存和驱动。预算还应包括捕获池、编码器内部 surface、音频、文件缓冲和 GUI。

## 打包与许可

主程序继续按仓库许可证发布。媒体库使用 LGPL FFmpeg 与 BSD OpenH264，不引入 GPL 编码器；依赖通知、固定源码及构建脚本随包提供。动态链接、静态链接或子进程都不自动免除组件分发义务。不要把使用 libx264 的 FFmpeg 构建称作纯 LGPL，也不要删除授权文件来缩小安装包。[FFmpeg 法律说明](https://ffmpeg.org/legal.html)

最小 FFmpeg 必须按实际功能裁剪：视频结束后转 GIF 需要 demuxer、decoder、scale、palettegen/paletteuse 和 GIF muxer，只有编码器的构建不够。Windows 使用系统编码时无须为实时 MP4 链路捆绑完整 FFmpeg；额外容器/导出使用同进程的可替换共享库。

## 验收方法与目前证据边界

比较必须固定采集 API、同一 GPU、区域、分辨率、实际 FPS、codec、质量、音轨、指针与预览。至少覆盖静态文字、滚动文字、视频运动；记录所有相关进程 CPU 核秒、RSS/PSS、GPU encode/copy/compute、显存、掉帧、文件码率和解码质量。不能以 wf-recorder 静态 VFR 对比 OBS 动态 60fps 宣称倍数收益。

功能验收覆盖启动失败、倒计时取消、暂停恢复、停止封装、磁盘错误、空音频、短录制、长时 A/V 漂移、尺寸变化和异常退出。CI 证明构建、测试和产物完整性；没有真实 GPU/桌面时不能宣称已验证全部硬编和采集路径。后续验证结果单独记录，调研阶段尚未进行公平性能实验。

## 复现

源码下载采用浅克隆；OBS 只展开 libobs、插件、前端与平台图形模块。GSR clone 地址是 `https://repo.dec05eba.com/gpu-screen-recorder`，`git.dec05eba.com` 是源码浏览站。

网络检索使用 smart-search deep 规划、Context7/Exa 查找和 fetch 读取官方正文。综合 search 服务 TLS 失败，未更改服务设置；协议 mirror 与本地固定源码补充证据。具体命令和 URL 见平台、打包附录。未抓取成功的页面及营销性能表不作为结论依据。
