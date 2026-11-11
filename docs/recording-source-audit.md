# 开源录制源码比较

本页引用实际下载源码的固定提交。未编译比较这些项目，不能把架构分析当作 CPU、功耗、内存或包体积实测。

## wf-recorder

- [默认 libx264](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/meson_options.txt#L1)，[superfast/zerolatency/CRF20](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/frame-writer.cpp#L50)。默认不是硬件编码。
- [copy_with_damage 与空 damage 回调](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L313)。等待变化减少帧，不实现变化矩形编码。
- [微秒 PTS](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/frame-writer.cpp#L770)、[指定帧率时添加 fps filter](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/frame-writer.cpp#L266)。VFR 静态省工作，CFR 会补帧。
- [VAAPI 与 compositor 同设备检查](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L1254)，[DRM_PRIME 映射](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/frame-writer.cpp#L848)。GBM buffer 到 AVDRMFrameDescriptor，再映射 VAAPI，映射缓存复用；Y_INVERT 不支持，跨 GPU 禁用。
- [hwupload 分支 CPU map](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L693)。出现 DMA-BUF 并不必然排除 CPU 访问。
- [缓冲池初始两个、忙时扩展](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/buffer-pool.hpp#L75)，[最多16槽](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L201)。降低分配频率，不提供整个会话内存预算。
- [采集协议要求](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L759)。需要 wlr-screencopy/xdg-output；其 PipeWire 代码是音频，不等于支持通用 Portal 视频。
- [writer 等待](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L635)与[采集等槽](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L1389)包含短 sleep 轮询，不能描述为零唤醒。

借鉴变化驱动、VFR、复用、直接映射；不照搬固定16槽、轮询和设备假设。MIT 源码与外部媒体依赖分别审查。

## wl-screenrec

- [VAAPI device/frame context](https://github.com/russelltg/wl-screenrec/blob/32f7810f3ef7543f4460c65851687435f6a24be1/src/avhw.rs#L35)，池初始5张，实际在途引用仍须统计。
- [map_drm](https://github.com/russelltg/wl-screenrec/blob/32f7810f3ef7543f4460c65851687435f6a24be1/src/main.rs#L405)：从编码友好的硬件表面导出 DRM_PRIME 给 compositor 写入，再硬件转换/编码。这比截图后上传更直接。
- [硬件 filter](https://github.com/russelltg/wl-screenrec/blob/32f7810f3ef7543f4460c65851687435f6a24be1/src/filter.rs#L21)：crop、scale、旋转在 GPU 路径；软件输出增加 hwdownload，不再是全 GPU。
- [限帧器](https://github.com/russelltg/wl-screenrec/blob/32f7810f3ef7543f4460c65851687435f6a24be1/src/fps_limit.rs)：保留 on_deck 帧，防止限帧丢掉最后一次变化，结束时 flush。值得独立复现其不丢末帧的契约。
- 支持 wlr-screencopy 或 ext-image-copy-capture 与 linux-dmabuf。默认 VAAPI，Vulkan 是实验 feature；旋转依赖驱动。使用 ffmpeg-next/ffmpeg-sys-next，不是无需 libav 的纯 Rust 编码器。[README](https://github.com/russelltg/wl-screenrec/blob/32f7810f3ef7543f4460c65851687435f6a24be1/README.md)

README 的 CPU 对比由作者明确标为非严谨测试，且对应旧版本，不用于本轮倍数结论。项目 Apache-2.0 不代表 FFmpeg 依赖同许可。

## OBS

[完整审计](recording/obs-audit.md)确认 PipeWire DMA-BUF/modifier/显式同步、WGC/DXGI 纹理复制、GPU 合成/颜色转换、NVENC/VAAPI texture encode、有界 core 纹理、音频时钟和已编码 packet 封装子进程。

OBS 并非总是 CPU 读回：raw_active 才走下载，PASS_TEXTURE 且支持 NV12/P010 时可以直接纹理编码。VAAPI 创建 surface 后导出 planes 到 GL，NVENC D3D11 使用共享 texture/keyed mutex，再复制进已注册编码表面。回退到 CPU 输入时仍会上传，因此应描述具体路径。

场景合成、预览、插件、多源音频是额外能力，不能仅凭安装包或默认任务管理器截图判定效率。借鉴所有权、同步和能力探测，不复制 GPL 源码或嵌入完整 libobs。

## GPU Screen Recorder

固定源码：[提交](https://git.dec05eba.com/gpu-screen-recorder/commit/?id=f97d1e392816503282d20a0804c3ce3fd2943767)。这是独立 GPL-3.0 组件，支持 Portal、X11 和 KMS 等不同路径，不能混称为一种捕获机制。

- `src/pipewire_video.c:on_process_cb` 排空队列、保留最新含视频 buffer，并逐个更新 cursor metadata；DMA-BUF 保存 fd/offset/stride，读取 crop/rotation/damage。buffer 在回调结尾归还；这不同于 OBS 显式 timeline 契约，不能据此宣称所有驱动同步等价。[源码](https://git.dec05eba.com/gpu-screen-recorder/tree/src/pipewire_video.c?id=f97d1e392816503282d20a0804c3ce3fd2943767)
- `src/encoder/video/vaapi.c` 从 av_hwframe_get_buffer 取得编码 surface，vaExportSurfaceHandle 导出，按 plane/modifier 创建 EGLImage/GL texture，颜色转换直接写目的 texture，surface 映射有缓存上限。[源码](https://git.dec05eba.com/gpu-screen-recorder/tree/src/encoder/video/vaapi.c?id=f97d1e392816503282d20a0804c3ce3fd2943767)
- `src/recorder/recorder.c` 在录制循环处理 damage、时序、颜色转换、编码与 packet；软件编码是另一条路径。不能只看到 GPU 软件名就认定硬编成功。[源码](https://git.dec05eba.com/gpu-screen-recorder/tree/src/recorder/recorder.c?id=f97d1e392816503282d20a0804c3ce3fd2943767)
- `recording_clock.c` 以单调时钟减去暂停累计偏移；CLI SIGUSR2 暂停，SIGINT 停止，较新版本有 IPC 状态和绝对暂停命令。[时钟](https://git.dec05eba.com/gpu-screen-recorder/tree/src/recorder/recording_clock.c?id=f97d1e392816503282d20a0804c3ce3fd2943767)
- Portal 不要求 KMS helper 特权；KMS 屏幕捕获可能要求管理权限。项目安装默认选项包含 capabilities、systemd 和 NVIDIA 休眠配置，Mark Shot 打包应禁用这些系统修改。[构建选项](https://git.dec05eba.com/gpu-screen-recorder/tree/meson_options.txt?id=f97d1e392816503282d20a0804c3ce3fd2943767)
- Portal 捕获源码仍有 region crop TODO，不能把通用 `-region` 参数宣称为 Portal 任意选区支持。GPU/驱动、桌面协议和来源尺寸变化仍需兼容验证。[Portal 实现](https://git.dec05eba.com/gpu-screen-recorder/tree/src/capture/portal.c?id=f97d1e392816503282d20a0804c3ce3fd2943767)
- 静态 FFmpeg 脚本只启用使用的编码器、容器和音频滤镜，仍启用 libx264/GPL，并带网络/直播相关组件。Mark Shot 只用本地录制时应按实际能力进一步裁剪，额外 GIF 解码与滤镜需要重新启用。[构建脚本](https://git.dec05eba.com/gpu-screen-recorder/tree/extra/build_ffmpeg.sh?id=f97d1e392816503282d20a0804c3ce3fd2943767)

独立 helper 可以将设备和编码器释放与主界面隔离；GPU 原始帧留在 helper 内，不用将 BGRA 穿越进程。打包时保留源码、修改说明、许可证和可复现命令。官网“最快”等宣传不作为本项目性能承诺。
