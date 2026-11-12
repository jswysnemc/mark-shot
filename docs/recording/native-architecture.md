# 进程内录制实现

2026-11-12，`feat/native-barcode-scanning`。录制和导出不创建外部媒体进程；参考项目用于协议、时间和资源策略调研，不作为运行时可执行程序。

## 模块与数据流

- `capture/wayland`：持久 ext-image-copy 或 wlr-screencopy。普通方向、同一 DRM 设备和兼容修饰符时使用双 DMA-BUF；合成器写入的帧与当前可读帧分开持有。失败回退 SHM。
- `capture/portal`、`capture/pipewire`：持有授权会话和连续 PipeWire 流。当前视频仅协商 MemFd；不盲目 mmap 非线性 DMA-BUF。来源位置、逻辑尺寸和物理流尺寸匹配后才能套用预选区域。
- `capture/x11`：MIT-SHM 1.2 文件描述符共享；服务端不支持时 GetImage。复用像素缓冲，支持 XFixes 指针，无 OpenGL 前提。
- `session`：控制、单调时钟、限帧、静态画面延续和结束封装。协议事件在编码期限前推进，不能把请求和完成各等待一个帧周期。
- `session/audio`：PipeWire 原生时钟与有界队列；首次连接不可用时使用动态加载的 PulseAudio 客户端。源数据跨启动或恢复边界时裁剪旧样本。中途断流明确失败。
- `crates/shot-media`：稳定私有 C ABI，不向 Rust 暴露 libav* 结构体。一个可替换共享库承担 Linux 编码、MP4/MKV 和 GIF；Windows 实时编码继续使用系统 Media Foundation。

## 显存与同步

GBM 分配与每个导出 FD 都由 Rust 句柄拥有，协议缓冲先销毁，再释放内存对象。
VAAPI 导入包含格式、尺寸、平面、FD、偏移、步长、大小和修饰符。编码端使用自己的 NV12 表面。
输入帧完成 GPU 裁剪和颜色转换后同步，再归还采集缓冲；编码器可以继续持有自己的输出表面。
静态重复帧读取已完成缓冲，不能读取正在采集的另一张缓冲。

目前直接 Wayland 的 DMA-BUF 快速路径要求普通方向；旋转或 Y-invert 使用 CPU 兼容路径。
Portal 使用共享内存，X11 使用 CPU 读取；这两条路径没有实现免 CPU 回读。NVENC 目前接受 CPU 转换后的帧。
Windows WGC/D3D11 路径已有独立资源所有权，尚未在本机进行 Windows GPU 验收。

## 失败边界

- 自动模式在首帧失败且没有已提交视频时允许一次软件重建；已有视频后不覆盖原会话。
- 显式硬件模式不改成软件编码；显存捕获不可用可以使用 CPU 采集加硬件编码。
- 授权等待响应停止，授权关闭进入失败状态；没有首帧不会发布“录制中”。
- 可用视频仅在正常收尾后原子发布，失败保留工作目录。析构尝试写尾部，分片MP4不保证任意断电无损。
- 音频后续时间戳参与对齐，间断补静音、重叠裁除；超过预算报错，不无界补齐。
- 录制/导出不下载组件，不启动音频服务，不修改GPU或桌面配置。

## 交付

Linux媒体库约7.38MiB；Windows导出库约5.20MiB。构建未启用 GPL、nonfree、libx264；软件H.264使用BSD OpenH264。
源码包保留完整库源码、构建脚本和清单，只省去OpenH264测试视频。完整程序和源码包体积另计，见验证记录。
系统依赖包括桌面协议库、驱动、libva/libdrm/libgbm；PulseAudio客户端按需加载。
