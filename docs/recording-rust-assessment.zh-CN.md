# 视频/GIF 录制的 Rust 重写与性能、内存评估

日期：2026-11-01。范围：当前 Mark Shot 仓库，基线 `1e3877d`（0.1.62）。用户提到的“sai”按当前项目理解。本报告记录优化前的评估，评估阶段未修改生产录制代码；后续实施和验证见[逐步优化记录](recording-optimization.zh-CN.md)。原有剪贴板修复保持不变。

### 当前工作区复核

第 3 节描述的是原始基线的问题，不是当前实现仍然存在的全部问题。本次变更已经实现提前限帧、有界跨线程交接、采集像素预算及 PAL8 心跳复用。当前取舍应基于这些改进，而不是将它们重复计作 Rust 迁移的预期收益。

| 已有证据 | 对迁移决策的含义 |
| --- | --- |
| 4K、30 帧交接压力：旧模式约 966.51MiB，合并通知后约 80.08MiB | 证明限制在途对象有效；不是完整应用降幅，也不是 Rust 的收益 |
| 4K GIF、每四帧更新：24 帧处理约 3801ms 降至 1573ms，文件字节一致 | 重复量化消除有收益，现有 C++ 已经实现；不能外推到持续变化的画面 |
| 正常 4K 视频：第三阶段前后峰值约 802.50/800.97MiB | 采集预算没有消除编码器内存；仅迁移外层所有权不足以解释显著下降 |
| VA-API 帧池从 24 改为 4，单帧持有时 GTT 不降，申请第五帧失败 | 不能直接缩池发布；该实验不等于完整硬件录制测试 |

剩余优先方向是采集到编码之间的 GPU 数据搬运，以及真正变化的 GIF 画面所需的量化、差分策略。Rust 可用于局部实现或加强生命周期约束，但目前没有等价 Rust 实现的对照结果，不能承诺语言迁移的提速比例。

内存估算应按唯一像素分配及实际寿命统计：4K BGRA 一帧约 31.64MiB，NV12/YUV420 约 11.87MiB，PAL8 索引约 7.91MiB 加调色板，均不含行对齐。共享引用不能重复相加，跨 FFI 深复制却会新增整帧。FFmpeg 内部工作帧、采集池、GPU/驱动和进程常驻资源应分别归因；Rust 的结构体大小或无垃圾回收特性不能替代这项核算。

若启动 Rust 试验，应固定采集源、编码器、画质、线程数、队列预算和输出时间轴，以当前改良版 C++ 为对照；同时记录端到端 CPU、峰值与稳定期 RSS/PSS、GPU 计数、迟交及停止延迟。只证明类型安全或维护性时，应按维护性收益报告。原始数据和分阶段验证范围见[逐步优化记录](recording-optimization.zh-CN.md)。

## 1. 结论

**存在明确的性能和内存优化空间，但不建议为了性能把整套录制链路重写成 Rust。** 当前大部分重计算由 FFmpeg/x264、平台采集和 GPU/CPU 数据搬运完成。把 C++ 调用改成 Rust 绑定，不能自动消除这些开销。

建议将三个问题分开决策：

| 路线 | 判断 | 收益来源 | 主要风险 |
| --- | --- | --- | --- |
| 保留 C++，修正帧率、跨线程队列和缓冲预算 | 优先 | 少采集、少处理无效帧，限制全部在途像素 | 时间轴、丢帧语义和停止顺序回归 |
| 保留 FFmpeg，局部引入 Rust 管理帧与有界数据流 | 有条件值得 | 更明确的所有权和状态约束，便于维持内存上限 | FFI、线程亲和性、生命周期和额外复制 |
| 为 GIF 增加独立的差分/量化实现 | 值得做受控试验 | 减少调色板重算、重复帧处理和全帧扫描 | 色彩、透明度、帧处置及许可证 |
| 将同一套 FFmpeg 调用逐行翻译成 Rust | 不建议作为性能项目 | 主要是维护和类型安全收益 | 工作量大，没有已证实的速度或内存收益 |
| 重写采集、音频、编码、封装和 UI | 当前不建议 | 只有同时重构数据流才可能显著获益 | 跨平台行为和发布链路风险最高 |

**先修正时间轴，再比较性能。** 本次已经复现高频输入被按较低目标帧率全部编码的问题。单纯提高吞吐量，可能反而让更多多余帧进入错误时间轴。

## 2. 当前链路已经具备什么

采集端输出 `RecordingFrameSample`，经过主线程上的 `RecordingFrameGrabber`、`RecordingController`，进入 `RecordingAsyncWriter` 的工作线程，再调用按需加载的 FFmpeg 后端。

- [wlroots 采集](../qt/src/platform/wayland/wlroots_screencopy_worker_frame.cpp) 使用 `wl_shm` 映射和共享所有者，直接向编码侧提供 BGRA 视图；不能误称这条路径已经做了一次额外的应用层全帧复制。
- [PipeWire 采集](../qt/src/screen_capture_pipewire_raw_frame.cpp) 的 BGRA 快路径将共享内存逐行复制到池化缓冲；DMA-BUF 路径优先 GPU 裁剪后读回 BGRA，不是保留 GPU 表面直接编码。
- [Windows WGC](../qt/src/platform/windows/windows_wgc_stream.cpp) 使用长生命周期帧池，但经过 `CopyResource → Map → memcpy → Unmap`，输出 CPU BGRA。
- [帧载荷](../qt/src/recording/recording_frame_payload.h) 已支持 `QByteArray` 隐式共享和 `shared_ptr` 保持映射生命周期；复制样本对象不等于复制整张图。
- [异步写出器](../qt/src/recording/recording_async_writer.cpp) 已有工作线程、队列容量和背压；PipeWire/WGC 在背压时会跳过复制或读回。
- [视频转换器](../qt/src/recording/libav/libav_video_scaler.cpp) 已有常驻切片线程池，不是逐帧创建线程。视频补帧复用已转换像素，硬件路径还复用上次上传结果。
- [编码器策略](../qt/src/recording/recording_memory_policy.cpp) 已限制常用软件编码预设的线程数；高质量预设保留自动并发，避免用积压换内存下降。
- FFmpeg `AVFrame` 已采用引用计数和可写性检查。Rust 的资源所有权有维护价值，但不能把现有 C++ 当成没有 RAII、没有共享缓冲的基线。

当前 Rust 工作区包含几何、图像、拼接、配置、编辑和 FFI 模块，没有独立录制核心。录制主要仍是 C++ 加 FFmpeg。

## 3. 优先级最高的发现

### 3.1 高频输入没有在视频写出端被正确归一到目标 FPS

代码证据：

- [WGC 适配器](../qt/src/recording/recording_windows_wgc_capture_stream.cpp) 写入 `request.targetFps`，但底层 [WGC 实现](../qt/src/platform/windows/windows_wgc_stream.cpp) 不读取它；读回前只有背压检查。
- [wlroots worker](../qt/src/platform/wayland/wlroots_screencopy_worker_frame.cpp) 发布帧后立即请求下一帧，没有按 `m_options.fps` 限频。`copy_with_damage` 等待画面变化，不等于限制为用户指定的 FPS。
- [VideoRecordingWriter](../qt/src/recording/video_recording_writer.cpp) 只调用 `duplicatesBefore` 补帧，随后无条件写入当前样本。[RecordingFramePacer](../qt/src/recording/recording_frame_pacer.cpp) 没有“丢弃过早样本”的返回语义。
- [libav 写出器](../qt/src/recording/libav_recording_process.cpp) 每次写入分配 `m_nextPts++`，时间基固定为 `1/fps`。

**实测：** 向真实生产写出链路提交 120 帧，时间戳按 60Hz 生成，覆盖 0～1983ms；目标设置为 30fps。输出仍为 120 帧、`r_frame_rate=30/1`，`stream.duration=3.966667s`，而对应输入应约为 2 秒。

这一问题同时影响正确性、CPU 消耗、输出大小和缓存占用。背压只有负载过高时才生效，不能作为帧率控制。PipeWire 的预读取限帧和轮询后端的调度保护存在，但不能套用到 WGC/wlroots。

边界：本次复现的是写出接口级行为，并确认存在无目标 FPS 保护的上游路径；没有录制真实 Windows/niri 桌面，不能断言所有 60Hz 会话都出现两倍时长。实际还受画面变化和采集吞吐量影响。已有容器最后一帧时长元数据偏差约一个帧间隔，也不能解释这里接近两倍的差异。

建议在昂贵采集/读回之前按单调时间控制输入，并在写出端保留时间戳归一化保护。验收需覆盖 60/144Hz 输入到 12/30/60fps、时间戳抖动、暂停恢复和丢帧。此项不需要 Rust。

### 3.2 编码队列有上限，不代表整个采集管线有上限

[PipeWire](../qt/src/recording/recording_pipewire_capture_stream.cpp) 和 [WGC](../qt/src/recording/recording_windows_wgc_capture_stream.cpp) 在采集线程中逐帧 `QMetaObject::invokeMethod(..., Qt::QueuedConnection)`，之后才由主线程把帧放入有界编码队列。

当主线程阻塞时，采集线程可能尚未收到编码背压，Qt 待处理事件仍持有像素。四槽 [BGRA 缓冲池](../qt/src/recording/recording_bgra_buffer_pool.cpp) 在所有槽位被共享时会分配新块，而不是覆写旧帧；这保证安全，但不是“四帧总上限”。

**结构性压力实验：** 使用生产 BGRA 池和相同的 Qt 排队方式，暂不处理主线程事件，连续投递 24 张 4K 图像。RSS 从约 11.45 MiB 升至约 772.24 MiB；池本身只报告约 128 MiB 容量，其他图像由排队回调持有。随后 24 个回调全部处理完成。此实验没有启动真实采集，也没有经过完整控制器，证明的是该队列模式的保留行为，不是正常录制峰值。

应将总量控制放到跨线程交接之前：有界帧队列或“最新待处理帧 + 至多一个唤醒事件”，让主线程只处理状态和通知。具体选择必须定义丢弃旧帧还是新帧、时间轴如何补齐、停止时如何排空。

这可以用 C++ 实现，也适合未来的 Rust 小型录制核心；性能收益来自有界交接，不来自 `Arc` 替换 `shared_ptr`。

### 3.3 niri/wlroots 的共享内存池与通用 BGRA 池不是同一个池

[WlrootsScreencopyBufferPool](../qt/src/platform/wayland/wlroots_screencopy_buffer_pool.cpp) 最多保留 16 个共享内存缓冲，没有通用 BGRA 池那种闲置回收策略，也没有按帧字节数调整容量。

4K BGRA 单帧裸数据约 31.64 MiB，16 帧约 506.25 MiB。这个数是紧密排列像素的容量模型，不是本次测得的真实 niri RSS；行对齐、页驻留及下游共享会改变实际数值。共享帧不能再同时计入另一份“编码队列像素”而重复相加。

建议统一定义总帧预算、最小缓冲数量和闲置回收机制，保留 compositor/编码器尚未归还的租约。不要简单将 16 改为 3；心跳、正在捕获的帧和编码中的帧也可能同时持有引用。

### 3.4 GIF 的主要成本在调色板与索引编码，不只是格式转换

当前 [GIF 滤镜](../qt/src/recording/libav/libav_gif_palette_filter.cpp) 为：`split → palettegen(stats_mode=single) → paletteuse(new=1, Bayer)`。上游实现表明，每帧新调色板还会重建颜色映射并释放旧缓存，详见 [一手资料](recording-rust-references.zh-CN.md)。

本机实际滤镜图还显示两条 `RGB24 → BGRA` 自动转换支路，分别进入 palettegen 和 paletteuse。临时绕过它们的结果见第 5 节：有局部内存收益，但不是显著且普遍的提速。

更有价值的候选是：

- 对心跳重复的同一 `sequence`，复用量化结果或合并 GIF 帧延迟，避免再统计调色板。
- 保留可信的变化区域。当前 wlroots 的 [damage 回调](../qt/src/platform/wayland/wlroots_screencopy_worker_frame.cpp) 忽略了矩形参数。
- 在允许的画质误差内复用调色板，只对实际变化区域量化；场景切换时完整重建。
- 区分“实时低延迟 GIF”和“停止后进一步压缩”的成本模型，不能把整段 RGBA 缓存起来再声称低内存。

不能直接叠加 `new=1` 与旧索引区域复用：调色板变动可能使同一索引代表不同颜色。也不能把 `stats_mode=diff` 当作保持原有实时语义的直接替换。必须验证颜色、透明度、帧处置方式和持续时间。

### 3.5 硬件编码仍有明显的数据搬运空间

VA-API/QSV 当前是 CPU BGRA → CPU NV12 → `av_hwframe_transfer_data` → 硬件编码；NVENC/AMF 等则走当前允许软件输入帧的路径，也不等于复用原始采集纹理。

[硬件帧上下文](../qt/src/recording/libav/libav_hw_encoder_context.cpp) 设置 `initial_pool_size=24`。按紧密排列 4K NV12 估算，24 张表面的裸像素约 284.77 MiB；这是池配置的容量参考，不是已测 VRAM，也不能假定所有设备同时实际驻留 24 张表面。

值得验证的优化包括：WGC 区域录制先在 GPU 裁剪再读回、兼容设备上的 GPU 色彩转换、采集纹理直接进入编码器、按实际在途需求调整硬件表面池。它们可能比语言迁移更有价值，但本次没有 GPU/Windows 实测，不能给出节省百分比。

## 4. 本机基准：编码器配置比调用语言更能解释内存差异

环境：AMD Ryzen 7 8845H，16 逻辑处理器，Linux x86_64，Qt 6.12.0，FFmpeg 9.0.2，GCC 16，独立 Release/LTO 构建。

方法：复用生产录制源文件、异步写出器、缓冲池和动态后端；临时扩展现有 `recording_memory_benchmark.cpp`，支持 GIF 和固定画面。每个配置独立进程，三次测量取中位数，使用实际解码帧数核验。视频 120 帧，GIF 36 帧。输入为不透明合成渐变，不截取用户桌面。

| 场景 | 峰值 RSS MiB | 总耗时 ms | 连续供帧吞吐 fps | CPU 时间 s |
| --- | ---: | ---: | ---: | ---: |
| 视频 1080p/30，均衡默认 | 372.54 | 529 | 226.84 | 3.01 |
| 视频 4K/30，均衡默认 | 801.98 | 2346 | 51.15 | 10.76 |
| 视频 4K/30，强制自动编码线程 | 2255.34 | 1387 | 86.52 | 15.40 |
| 视频 4K/30，强制两个编码线程 | 646.70 | 4046 | 29.66 | 10.24 |
| 视频 4K/30，编码队列限制为一帧 | 738.78 | 2402 | 49.96 | 10.51 |
| 视频 4K/60，均衡默认 | 563.80 | 818 | 146.70 | 5.32 |
| 视频 4K/30，高质量默认 | 2527.08 | 1772 | 67.72 | 21.18 |
| GIF 720p/12，变化画面 | 92.11 | 722 | 49.86 | 0.81 |
| GIF 1080p/12，变化画面 | 135.61 | 1429 | 25.19 | 1.70 |
| GIF 1080p/12，固定画面 | 132.95 | 1071 | 33.61 | 1.33 |

这些是编码压力基准的吞吐量，不是实际桌面 FPS。CPU 时间为所有线程 user+system 之和，可能大于墙钟时间。高质量档墙钟更短而 CPU 时间、内存更高，是并发策略不同，不代表高质量计算更少。

主要解读：

- 自动线程与当前默认相比，峰值差约 1453 MiB；这种差异无需改语言即可产生。它重现并支持仓库已有 [录制内存优化](recording-memory.md)，不是本次新实现的收益。
- 两线程节省约 155 MiB，却使连续供帧吞吐下降至约 29.66fps，没有稳定的 30fps 余量。240 帧实时供帧三次测试中，默认均无迟交；两线程分别出现 0、49、110 个迟交样本，最大落后分别为 0、21、120ms。六个输出均解码为 240 帧；迟交不等于丢帧。
- 队列从三帧改为一帧只减少约 63 MiB；801.98 MiB 不是“主要由三帧队列组成”。编码器内部工作缓冲仍是大项。
- 4K/60 均衡档使用更快的软件 preset，不能把它与 4K/30 的内存差单独归因于 FPS。
- GIF 固定画面的文件约 194 KiB，变化画面约 7365 KiB，但 CPU 时间只从 1.70s 降到 1.33s。小文件不代表避免了逐帧量化。

## 5. GIF 的临时对照与 API 耗时定位

只在仓库外复制两个 GIF 源文件，将 scaler 和滤镜入口从 RGB24 改为 RGB32；生产实现保持原样。两版本交替运行，每场景三次，所有 12 对 GIF 逐字节一致，24 个输出均解码为 36 帧。

| 场景 | 原实现 RSS MiB | RGB32 RSS MiB | 原耗时 ms | RGB32 耗时 ms |
| --- | ---: | ---: | ---: | ---: |
| 720p，变化画面 | 92.23 | 85.62 | 732 | 725 |
| 1080p，变化画面 | 135.27 | 125.85 | 1414 | 1393 |
| 1080p，固定画面 | 132.88 | 131.44 | 1061 | 1048 |
| 4K，变化画面 | 342.15 | 345.77 | 5433 | 5389 |

1080p 的峰值减少约 7%，但耗时差约 1.5%，不足以脱离重复测量波动承诺稳定提速；4K 内存反而略增。结论是值得局部验证，不是“去掉转换就全面更快、更省内存”。此对照使用不透明像素；RGB32 可能改变原先 RGB24 丢弃 alpha 的语义，透明度、BGRx 未定义 alpha、色彩范围和其他平台还需检查。

额外用临时函数拦截器记录 FFmpeg 入口墙钟耗时，不作为内存基准：

- GIF 1080p、36 帧：filter pull 累计约 1025ms，codec send 约 340ms；外部 sws_scale 的各线程调用时间合计约 51ms。
- 视频 4K/30、120 帧：codec send 约 1898ms，codec receive 约 228ms；外部 sws_scale 各线程调用时间合计约 1166ms。

不同线程及嵌套调用不能直接相加或换算为 CPU 占比；拦截器还会改变库加载行为。这不是采样火焰图，但与“GIF 的主要工作在原生量化/编码，Rust 包装层不是主要计算者”的判断一致。

## 6. 真正适合 Rust 的范围

### 可考虑：有界帧数据流与生命周期管理

若在正确的时间轴和 C++ 基线之上仍决定引入 Rust，建议只迁移帧预算、准入控制、时间戳策略和缓冲租约管理。采集适配器、Qt UI、FFmpeg 编码器先不迁移。

候选拆分，尚未实施：

```text
crates/shot-recording-core/src/
  frame.rs           # 帧描述、CPU/GPU 载荷和释放约束
  budget.rs          # 整条管线的字节预算与在途配额
  queue.rs           # 有界交接、合并通知和背压
  timeline.rs        # 限帧、补帧、暂停及时间戳规则
crates/shot-ffi/src/
  recording.rs       # 窄 C 接口与错误边界
qt/src/recording/
  rust_recording_adapter.cpp  # Qt 事件与 Rust 数据流之间的薄适配
```

这不是为了增加一个新框架：不需要让每帧进入 Tokio 异步运行时，不需要把所有 QImage 再复制进 Vec，也不应保留旧管线再叠一层完整缓存。

现有 [RecordingBackendApi](../qt/src/recording/backend/recording_backend_api.h) 仍是 Qt/C++ ABI，不能直接由 Rust 结构体替代。需要保留 C++ `RecordingWriter` 适配器，或采用 CXX 不透明类型桥接。跨语言接口至少定义像素格式、stride、有效范围、时间戳、所有者与释放回调；GPU 帧另需设备标识、同步与释放线程约束。

FFmpeg、驱动和 Qt 的大块内存仍不会因为 Rust 管理了外层对象就消失。RAII 已有的正确释放行为也不能算作迁移新增收益。Rust 的可证实优势应是更容易维持不变量，而不是预先承诺更低 RSS。

### 可考虑：独立 GIF 算法试验

官方库能力与许可证详见 [资料报告](recording-rust-references.zh-CN.md)：

- `gif` 0.14.2 许可宽松，适合承载已量化的索引帧和局部矩形；默认 `Frame::from_rgba` 官方明确不是速度优化路径，不能直接视为升级。
- `imagequant` v4 已是 Rust，并提供 C 接口。可以从现有 C++ 中局部接入，完全不需要同时重写录制调度；它采用 GPLv3+或商业授权。
- `gifski` 1.34.0 追求高质量跨帧处理，含多阶段线程和前瞻，不能默认比当前方案低内存、低延迟；采用 AGPLv3+或另行授权。
- `ffmpeg-next` 是绑定层，仍依赖 FFmpeg；本机 FFmpeg 9.0.2、Ubuntu/Windows 的不同版本和 Rust 最低版本都需实际构建验证。

“引入用 Rust 写的量化库”与“把整个软件改成 Rust”是两个不同决定。算法、许可和质量标准应先于语言偏好。

## 7. 优化优先级与验收门槛

1. **先保证正确性：** 修正过量输入的限帧和输出时间轴；所有性能对照须验证解码帧数、PTS/DTS、实际持续时间和音画同步。
2. **先控制内存来源：** 对跨线程事件和共享内存池设端到端预算，测试主线程阻塞 50/200/1000ms、背压、停止、失败和取消，不能只测编码队列计数。
3. **再优化 GIF：** 重复帧、变化区域、调色板复用与格式转换分别单变量比较；不能以颜色失真、错误索引或缩短动画换取速度。
4. **再验证 GPU 数据路径：** 分别测试 Windows、niri/wlroots、GNOME/PipeWire、多显卡和软件回退，记录 CPU RSS、GPU 内存和停止延迟，而非只记录一个内存数字。
5. **最后决定 Rust 迁移范围：** 用同一输入、同一编码器/质量/线程预算，对照 C++ 改良版与 Rust 版。若只有类型安全收益，应按维护项目立项，不应称为已证明的性能优化。

建议基准覆盖 720p/1080p/4K、12/30/60fps、静态 UI、小区域变化、滚动文本、渐变、复杂运动；至少包括 30 分钟持续录制、分辨率改变和暂停恢复。报告中没有据此给出工期，因为跨平台验证与 GPU 兼容范围尚未确定。

## 8. 实验证据与限制

原始数据、合成媒体、临时基准源码和 CMake 保存在 `build/recording-rust-assessment/`，属于忽略的构建产物：

- `manifest.json`：代码版本、环境、范围。
- `results/matrix.json`：33 次现有实现基准。
- `results/rgb32-comparison.json`：24 次 GIF 单变量对照及文件哈希。
- `results/realtime.json`：6 次实时供帧，全部输出另经 ffprobe 核验为 240 帧。
- `results/video-60hz-input-30fps-probe.json`：时间轴复现。
- `results/queue-pressure.json`：跨线程帧保留压力实验。
- `results/gif-trace.log`、`results/video-trace.log`：滤镜图与入口耗时。
- `recording-tests.log`：现有 11 项相关录制回归测试通过。

以下是评估时的独立合成基准命令，不启动桌面采集。它依赖当时的源码 API；后续第三阶段将 `acquire()` 改为有界 `tryAcquire()`，不能直接用当前源码重建这份旧程序。复验应使用对应版本的独立源码快照和新的输出目录，保留原始二进制、媒体与日志。当前 API 的合成程序见 `qt/tests/recording_memory_benchmark.cpp` 和第三阶段基准目录。

```sh
cmake -S build/recording-rust-assessment \
  -B build/recording-rust-assessment/build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build/recording-rust-assessment/build --parallel 4
# 脚本会重新生成同名合成媒体与测量记录
python3 build/recording-rust-assessment/run_matrix.py
```

未测量或尚不能下结论的范围：真实桌面采集、Windows 行为、音频、GPU 编码与显存、复杂运动画质、Rust 替代实现、长期泄漏或分配器碎片。三次中位数不能消除系统负载、温度和调频影响；没有使用固定 CPU 亲和性。RSS、私有驻留、Windows 提交量和 VRAM 不能混为一谈。

**最终判断：有值得投入的优化，但应优先修正数据流与算法。Rust 可作为局部实现工具，当前证据不支持以性能为由启动整体重写。**
