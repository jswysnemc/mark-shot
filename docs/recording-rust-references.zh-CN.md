# 视频与 GIF 录制：Rust 方案一手资料核验

核验日期：2026-11-01。初始代码基线：`1e3877d`（0.1.62 准备提交）；增量核验范围包括当前工作区已实现的录制优化阶段 1～4，以及第五阶段单设备帧池实验。本次增量核验只更新本文，未修改生产代码、操作桌面或重新执行整套基准。

本文只记录官方文档、上游源码与许可证能够支持的结论，不提供未实测的提速百分比。Mark Shot 的完整代码分析与本机基准见同目录的录制评估报告。

## 1. 结论摘要

1. **把当前 C++ 调用层换成 Rust 的 FFmpeg 绑定，不等于更换编码器。** 编码、像素转换、调色板量化仍由同一套原生库执行。若帧表示、队列深度和复制路径不变，没有资料依据承诺明显降低 CPU 或内存。
2. **Rust 有价值的方向是明确帧所有权、限制跨阶段缓存、实现可替换的处理算法。** 当前 C++ 已完成有界帧交接、采集像素预算和 PAL8 心跳复用；不能把这些已获得的收益再次计入未来 Rust 迁移。收益来自数据流与实现约束，而不是 Rust 自动压缩像素或消除 GPU 上传。
3. **GIF 值得独立评估，但不能把“Rust GIF 库”当成同一种性能方案。** `gif` 提供基础编码与索引帧控制；`imagequant` 专注量化；`gifski` 追求跨帧视觉质量，包含多阶段并发与前瞻，未必符合最低内存、最低延迟目标。
4. **GPU 内存直接进入硬件编码器可能比语言迁移更有价值。** 但必须满足设备、像素格式、modifier、同步和生命周期要求，并保留 CPU 后备路径。
5. **许可证和编码器专利问题独立于实现语言。** Rust 包的宽松许可证不能覆盖其调用的 FFmpeg、x264 等组件；`gifski`、`imagequant` 也不是 MIT-only 依赖。

## 2. 当前实现与上游接口的对应关系

| 本地事实 | 对性能评估的含义 |
| --- | --- |
| [视频写出实现](../qt/src/recording/libav_recording_process.cpp) 调用 `avcodec_send_frame`、`avcodec_receive_packet`，由 FFmpeg 完成编码 | 换成 Rust 绑定后，这部分工作不会自动消失 |
| [GIF 写出实现](../qt/src/recording/libav_gif_recording_process.cpp) 复用 `AVFrame`，写入前调用 `av_frame_make_writable` | 已经使用 FFmpeg 的引用计数与可写性机制，不能把它描述成每帧必然创建全部对象 |
| [GIF 调色板滤镜](../qt/src/recording/libav/libav_gif_palette_filter.cpp) 使用 `stats_mode=single`、`new=1`、Bayer 抖动；[GIF 写出器](../qt/src/recording/libav_gif_recording_process.cpp#L342) 已跳过同一正采集序号的重复量化 | 进入滤镜的新画面仍承担直方图、调色板和颜色映射成本；已命中的心跳缓存不能再次作为新优化收益 |
| [硬件编码上下文](../qt/src/recording/libav/libav_hw_encoder_context.cpp) 使用 `av_hwframe_transfer_data` 上传软件帧 | 当前“硬件编码”不等于采集到编码全程无 CPU 像素路径 |
| [录制后端接口](../qt/src/recording/backend/recording_backend_api.h) 接收 `RecordingOptions`、`QString`，返回 C++ `RecordingWriter*`，明确要求相同 Qt/编译器 | 不是 Rust 可以直接实现并替换的稳定 C ABI，需要适配器或重新设计数据接口 |

## 3. FFmpeg 已经提供引用计数与背压接口

### 3.1 `AVFrame` 引用计数不是 Rust 独有能力

FFmpeg 官方 send/receive 文档建议使用引用计数的 `AVPacket` 和 `AVFrame`，否则 libavcodec 可能复制输入数据；`av_frame_get_buffer` 分配的是引用计数帧。[S1]

`av_frame_make_writable` 的官方定义是：帧可写时不操作；存在共享引用导致不可写时才分配新缓冲并复制。非引用计数帧视为不可写。[S2]

因此，需要区分：

- 复制 `AVFrame` 引用或 Rust 包装对象，不一定复制像素。
- 保留上一帧或让编码器继续持有引用，可能让下一次写入触发复制。
- Rust 的 `Vec::clone`、临时 RGBA 缓冲或跨 FFI 的重新打包，同样可能增加真实像素复制。
- 用 Rust 所有权表达“帧只归一个阶段所有”，只有确实减少共享可写场景或重复缓冲时，才会降低内存。

Rust 官方说明所有权决定资源何时释放，移动不等于深复制，`clone` 可显式复制堆数据；它没有保证相同媒体算法与数据表示使用更少内存。[S3] 最后一项性能判断属于基于相同数据量的工程推论，不是语言基准结果。

### 3.2 send/receive 的背压不能简单替换成无限异步队列

FFmpeg 接口规定发送端返回 `EAGAIN` 时应接收输出；接收端返回 `EAGAIN` 时应提供新输入。两端不能同时返回 `EAGAIN`，也不能靠等待时间让状态自动推进。[S1]

Rust 迁移应保留明确的提交、排空、结束和取消状态。额外套入任务运行时或无界通道，不会让编码器容量增加，反而可能把原本有限的编码器背压转化成应用层帧堆积。

## 4. GIF 的主要优化空间是算法与帧间策略

### 4.1 新画面的逐帧调色板仍有优化空间

阶段 4 已复用同一正采集序号的 PAL8 结果，所以下述逐帧成本只适用于实际进入滤镜的输入。当前实现不比较不同序号之间的像素内容，也没有周期调色板、局部重映射或 GIF 帧延迟合并，不能把心跳复用写成通用重复画面消除。

FFmpeg 官方定义：[S4]

- `palettegen=stats_mode=single` 为每一帧计算新的直方图。
- `stats_mode=diff` 统计与上一帧不同的部分；它改变统计权重，不等同于“实时每帧局部调色板”的无条件替换。
- `paletteuse=new=1` 为每个输出帧使用新的调色板。
- `paletteuse=diff_mode=rectangle` 只重新处理变化矩形；官方明确说明，小范围变化时可能提高速度，并可能改善误差扩散稳定性与 GIF 压缩。

上游 `vf_paletteuse.c` 进一步显示，`new=1` 的 `load_palette` 会清空调色板、映射和颜色查找缓存，释放缓存条目，再调用 `load_colormap`。[S5] 因此，“语言不变、减少不必要的调色板更新”也是有效的候选路线。

但不能直接宣布周期调色板、首帧调色板或 `diff_mode=rectangle` 一定更好：

- 首帧不一定包含后续窗口、视频或渐变中的颜色；量化误差可能明显增加。
- 相同 RGB 区域在不同调色板中可能对应不同索引。上游矩形逻辑会复制上一输出帧的索引区域；与逐帧新调色板组合时必须验证颜色正确性，不能只看编码成功或文件变小。[S5]
- 固定全局调色板的离线两遍流程，不能直接替代低内存实时录制；先缓存整段视频会改变内存和结束延迟模型。
- 变化区域比较本身需要扫描像素或可信的 damage 元数据，并非零成本。

### 4.2 格式往返的源码线索与实测核验

当前 Mark Shot 在启用调色板滤镜时，GIF scaler 输出 `RGB24`；此次读取的上游 `paletteuse` 源码声明输入格式为 `RGB32`。[S5]

主报告随后通过本机 FFmpeg 9.0.2 的 `avfilter_graph_dump` 确认了两条 `RGB24 → BGRA` 自动转换支路，并在仓库外进行了直接输入 RGB32 的对照。1080p 合成样本的内存略降，耗时变化很小，4K 峰值内存反而略升，不能据此宣称普遍提速。具体测量范围、透明度限制和数据见 [录制评估报告](recording-rust-assessment.zh-CN.md)。源码线索本身仍不能代替其他版本与设备上的实测。

### 4.3 `gif`：适合受控的基础编码，不自动提供高质量跨帧优化

核验的 `gif` 文档与 Cargo 版本为 0.14.2，许可证为 `MIT OR Apache-2.0`。[S6][S7]

它暴露帧尺寸、位置、延迟、透明索引、处置方式和索引像素缓冲，可以承载局部矩形帧及全局或局部调色板。调用方仍需决定哪些帧可以合并、如何处理旧画面、何时更新调色板。

`Frame::from_rgba` 与 `Frame::from_rgb` 官方明确说明未针对速度优化；必要时采用 NeuQuant 降到 256 色，各帧调色板独立。`from_rgba_speed` 的速度参数为 1～30，更高速度会牺牲画质；任意 alpha 也不能原样保留，因为 GIF 仅有 1-bit 透明语义。[S6]

所以，直接把现有 `palettegen/paletteuse` 改成 `Frame::from_rgba`，不能视为有证据支持的性能升级。更有针对性的试验是：由调用方提供已经量化的索引像素，测量基础 GIF 写出、调色板算法和帧差策略各自的开销。

### 4.4 `imagequant`：可以局部接入，不必重写整个录制模块

上游说明 libimagequant v4 已改为 Rust 实现，但仍通过 `imagequant-sys` 导出供 C 程序使用的接口。[S8]

这意味着可以在现有 C++ GIF 写出器中局部试用其量化算法，而不同时迁移采集、线程调度、音频、窗口与 muxer。官方也明确它不负责图像编码或解码；若仍用 FFmpeg 写 GIF，需要将量化结果接入既有 PAL8、时间戳和编码流程，而不是把它当作完整 GIF 写出器。[S8] 它的潜在价值主要是量化质量及可配置速度，而不是已经证明比当前 FFmpeg 更省 CPU 或内存。

许可证为 GPL v3 或更高版本，并有历史版权条款；非 GPL 使用可向作者获取商业许可。[S8][S9] 应将许可证选择列为试验前置条件。

### 4.5 `gifski`：高质量候选，不应默认列为低内存候选

核验的版本为 1.34.0（官方 docs.rs 页面标注 2026-10-20 发布）。项目强调跨帧调色板和时间抖动，支持直接接收 RGBA 帧，不必为了接入它先将帧保存为 PNG 文件。[S10][S11]

需要特别注意：

- `Collector` 与 `Writer` 应并发运行；未启动 writer 时，继续提交帧会阻塞。[S11]
- 官方 Collector 文档警告，乱序帧会缓存在 RAM 中，帧索引大缺口会造成高内存使用。[S11]
- 核验源码的入口队列容量为 `min(5, max_threads)`，并注明供去噪前瞻使用；管线还包含 resize、diff、quantize、remap、LZW、write 等阶段，部分中间对象持有 RGBA 图像、重要性图或量化结果。[S12]
- 入口队列有界不等于整个管线最多只有五帧；工作线程和各阶段在途对象也需要计入峰值。
- 一个 3840×2160 RGBA 帧的裸像素约为 31.64 MiB；假设某时刻仅入口就驻留五个这种帧，裸像素即约 158.20 MiB，尚未包含其他阶段。此处是容量模型，不是 gifski 的实测 RSS。

它可以作为“更高 GIF 画质或更好压缩”的可选编码器评估，但必须同时测量速度、峰值内存、录制中延迟和停止排空延迟，不能只比较文件大小。

官方提供 C API 和静态、动态库构建说明，现有 C++ 也能局部接入，不要求先建立 Rust 录制核心。其可选视频解码功能会额外使用 FFmpeg；直接接收采集 RGBA 帧不需要为了 GIF 编码启用该视频解码功能。即使 GIF 路径替换为 gifski，Mark Shot 的视频和音频录制仍依赖现有 FFmpeg，不能据此宣称整个应用已移除 FFmpeg。[S10][S11]

许可证明确为 AGPL 3 或更高版本，作者提供另行许可的可能性。[S10][S13] 不能把该库按普通 MIT 依赖接入而不审查分发义务。

## 5. GPU 路径：收益来自减少跨域搬运，不来自 Rust 语法

### 5.1 FFmpeg 的 transfer 与 map 不是同一件事

官方将 `av_hwframe_transfer_data` 定义为向硬件表面或从硬件表面复制数据。`av_hwframe_map` 则尝试建立映射，要求硬件帧上下文和设备兼容；不支持时可返回 `ENOSYS`。[S14]

`AV_HWFRAME_MAP_DIRECT` 要求映射和解除映射都不发生复制，但文档也提醒直接映射的访问性能可能低于普通内存。[S14] 因此，不应把“零复制”当成无条件更快，更不能用函数名称代替实际传输测量。

### 5.2 PipeWire DMA-BUF 必须处理格式、modifier 和同步

PipeWire 1.4.11 官方文档要求生产者与消费者协商格式及 modifier，并建议提供共享内存后备格式。[S15]

文档特别指出：DMA-BUF 可能使用设备特定的平铺和压缩；直接 `mmap` 不一定得到线性像素，也不保证与异步硬件读写正确同步。对独立显卡缓冲盲目进行 CPU 映射可能很慢，应使用合适的 EGL、Vulkan、VA-API 等接口处理。[S15]

Rust 实现仍需要处理缓冲归还、文件描述符所有权、同步完成、多显卡设备匹配以及格式回退。所有权类型有助于表达这些约束，但不能取消约束本身。

### 5.3 Windows 存在保留 GPU 表面的技术路线，但不是现成的通用替换

Microsoft Desktop Duplication API 将更新提供在 DXGI surface 中，并提供移动区域和脏矩形信息；这为保留 GPU 数据和减少局部更新工作提供条件。[S16]

NVIDIA 的 NVENC 编程指南支持 DirectX 11 设备，并允许将外部分配的 DirectX 资源注册、映射后作为编码输入。客户端仍需查询支持的像素格式，遵守注册、映射、使用和解除映射的生命周期。[S17]

这证明“采集表面到硬件编码器”的路线在特定组合下可行，不证明任意 WGC/DXGI 纹理、任意显卡和任意 FFmpeg 构建都能直接共享。CPU readback、CPU 色彩转换和重新上传是否能够省去，必须按 Windows、VA-API、QSV、NVENC 等实际设备路径分别验证。

## 6. Rust 绑定与跨语言接口

### 6.1 `ffmpeg-next` 仍然链接 FFmpeg

项目自述为 safe FFmpeg wrapper，docs.rs 明确导出 `ffmpeg_sys_next`。[S18] 本次读取的主分支 Cargo 版本为 9.0.1，具有向 `ffmpeg-sys-next` 传递静态构建、GPL、nonfree 等配置的 feature。[S19]

读取时 README 的兼容范围文字仍写到 FFmpeg 8.0，与主分支包版本说明不完全同步。不能据此断言新 FFmpeg 已兼容或必然不兼容；迁移必须针对当前 Linux、Windows 包版本实际构建并验证所用接口。

维护状态、绑定完整性和打包方式也是成本。将 C++ 换成 Rust wrapper 后，系统仍需提供兼容 FFmpeg 库，或者由构建流程明确生成并分发对应版本。

### 6.2 现有后端不能直接换成 Rust 动态库

[本地后端接口](../qt/src/recording/backend/recording_backend_api.h) 的版本字段只约束内部 API 版本，不使 Qt/C++ 对象变成稳定跨语言 ABI。

两条可行的接入路线：

- 保留现有 C++ `RecordingWriter`，由一个小适配器调用 Rust，数据接口采用像素指针、长度、stride、时间戳、像素格式和释放回调；明确借用仅在调用期间有效，或显式转移长期所有权。
- 通过 CXX 暴露不透明 C++ 类型，但需要显式处理固定位置、异常、线程安全及生命周期约束。CXX 不自动为不透明 C++ 类型提供 `Send`/`Sync`，不能把已有 Qt 对象随意跨线程传递。[S20]

不可行的捷径是把 `QString`、`QImage`、C++ 虚函数对象或 `std::shared_ptr` 的内存布局直接声明成 Rust 结构体。新的接口还必须明确谁分配、谁释放，避免跨动态库分配器不匹配。

## 7. 许可证核验

| 组件 | 核验结果 | 对本项目的要求 |
| --- | --- | --- |
| 当前 Mark Shot | [仓库 LICENSE](../LICENSE) 为 MIT | 不代表全部打包依赖都只受 MIT 约束 |
| FFmpeg | 默认 LGPL 2.1 或更高；可选 GPL 部分会影响最终构建许可证；部分外部库另有条件 [S21] | 按实际 configure 参数和链接组件核验，不按调用语言判断 |
| ffmpeg-next | wrapper 的 Cargo 与 LICENSE 标注 WTFPL [S19][S22] | 不覆盖 FFmpeg 本体及编码器依赖许可 |
| gif 0.14.2 | MIT 或 Apache-2.0 [S7] | 本身较易接入，但量化等新增依赖仍需逐项核验 |
| imagequant | GPL v3 或更高，另有商业授权 [S8][S9] | 若要求非 GPL 分发，先解决授权问题 |
| gifski 1.34.0 | AGPL v3 或更高，另有授权选项 [S10][S13] | 审查组合与分发义务，不能直接视为 MIT-only 产物 |

FFmpeg 官方另强调专利问题与软件许可证不是同一个问题。[S21] 本文提供工程选型依据，不作特定分发方式的法律合规结论。

### 7.1 本次选型需要保留的许可边界

- `gif` 的 `MIT OR Apache-2.0` 是二选一许可，不等于 `imagequant` 或 `gifski` 也使用相同许可。将 `gif` 与 GPL 量化器组合后，仍需审核量化器及组合分发条件。[S7][S8]
- MIT 项目并非在技术上无法使用 GPL/AGPL 依赖，但不能继续把包含这些依赖的分发产物描述为只受 MIT 约束。保留原有自有代码的 MIT 声明，也不能代替履行其他组件及适用组合的义务。
- `imagequant` 的商业许可与 `gifski` 的另行授权是上游提供的选择，不是已取得的授权；具体范围必须以实际合同和锁定版本为准。[S8][S10][S13]
- 经由 C API、CXX 或 `ffmpeg-next` 调用，不会改变被调用库的许可证。FFmpeg 官方不提供以付款换取专有许可的选项；实际构建是否包含 GPL/nonfree 组件、采用何种链接与分发方式，仍需独立核验。[S19][S21]

## 8. 阶段 1～4 后的状态与剩余验证

### 8.1 原评估中已有实现和测试支撑的改进

本节对照当前源码与[逐步优化记录](recording-optimization.zh-CN.md)，不是重新执行基准后的新测量。初始报告中的缺陷和数字属于旧基线，不能原样当作当前版本仍存在的问题。

| 原评估问题 | 当前代码与证据位置 | 已解决范围及边界 |
| --- | --- | --- |
| 高频采集输入导致额外视频帧和错误时长 | [节奏器](../qt/src/recording/recording_frame_pacer.cpp#L21)、[视频写出决策](../qt/src/recording/video_recording_writer.cpp#L155)、[WGC 提前限帧](../qt/src/platform/windows/windows_wgc_stream.cpp#L222)、[wlroots 请求调度](../qt/src/platform/wayland/wlroots_screencopy_worker_frame.cpp#L167)；优化记录第一阶段 | 目标时间格接纳、过早样本丢弃、显式 PTS 和有上限补帧已实现；合成两秒 60Hz 输入到 30fps 已回读为 60 帧、2 秒。WGC 源码有保护，不代表已经在 Windows 编译运行；实际音画同步尚未验证 |
| Qt 排队事件各自保留完整帧，绕过编码队列上限 | [帧交接器](../qt/src/recording/recording_frame_handoff.cpp#L11)；优化记录第二阶段 | 等待交付的位置只保留最新帧并合并唤醒，三个原生采集适配器均已接入；不再因这一交接方式随输入数保留全部历史帧。代价是丢弃未交付的中间画面，不是相同画面数量下的无损内存压缩 |
| wlroots 槽数和 BGRA 槽数不能约束大帧字节数，空闲像素保留 | [预算策略](../qt/src/recording/recording_capture_memory_policy.cpp#L9)、[BGRA 池](../qt/src/recording/recording_bgra_buffer_pool.cpp#L13)、[wlroots 池](../qt/src/platform/wayland/wlroots_screencopy_buffer_pool.cpp#L30)；优化记录第三阶段 | 已增加分配前字节准入和空闲回收。默认 96MiB 受两帧下限调整，不是固定的全进程上限；仍不覆盖 FFmpeg、显存、临时回退图像及所有会话外引用 |
| 静止心跳重复做 GIF 格式转换和量化 | [PAL8 缓存](../qt/src/recording/libav/libav_gif_frame_cache.cpp#L12)、[缓存命中路径](../qt/src/recording/libav_gif_recording_process.cpp#L342)；优化记录第四阶段 | 同一正序号共享最后一张量化结果，仍按每次输入更新 PTS 并编码；不新增整段历史缓存。未知序号、新画面及固定 RGB8 路径不复用；不同序号但像素相同的画面仍重新量化 |

阶段 2 的持有帧停止与重启测试还验证了 wlroots 协议对象与映射寿命分离：[`releaseWaylandBuffer`](../qt/src/platform/wayland/wlroots_screencopy_shm_buffer.cpp#L323) 在连接有效时释放协议对象，映射留给最后一个像素持有者。这是 C++ 已修复的生命周期问题，不应再作为 Rust 能独占解决的缺陷。

收益数字应保持实验边界：第四阶段的 4K 分组重复输入为 3801ms → 1573ms，持续变化输入为 3964ms → 4048ms；它支持“避免重复量化有效”，不支持“全部 GIF 内容普遍提速”。第三阶段正常 4K 编码的 802.50MiB → 800.97MiB 也没有证明正常编码所需内存显著降低。详细载荷、三次中位数和原始产物位置见优化记录。

### 8.2 尚未证明的候选

- GIF 格式往返已在本机当前构建中确认；其他 FFmpeg 版本、透明像素及实际桌面内容仍需核验，API 耗时记录不等于完整 CPU 采样。
- 局部变化、滚动文本、渐变、全屏视频四种内容下，固定/周期/自适应调色板各自的画质和压缩代价。RGB32 直接输入仍是临时对照，没有落入当前生产滤镜；PAL8 心跳复用不等于已经解决变化区域处理和重复帧延迟合并。
- `gif`、`imagequant`、`gifski` 在相同分辨率、帧率、线程预算和视觉质量标准下，是否优于**已经包含阶段 1～4 的现有后端**。目前没有替代库的对照实现，不能用旧版未缓存 GIF 作为唯一比较对象。
- GPU 数据面改造在本机及 Windows 设备上可省去哪些复制；其同步、驱动兼容和失败回退成本。优化记录第五阶段已有单设备 VA-API 帧池上传、回读实验，但没有采集到编码的完整 GPU 数据路径对照。
- Rust 小型适配器能否避免额外 RGBA 重排、`Vec` 深复制、重复引用计数和错误的跨线程类型声明。当前尚无 Rust 录制替代实现的性能证据，有界交接和池预算的收益已由 C++ 基线取得。
- 任何上述路线的端到端加速比例、RSS/VRAM 峰值和长时间录制稳定性。本文没有以官方宣传或不同机器的测试数字替代这些实测。

第五阶段实验对缩小硬件池提出了额外约束：`initial_pool_size` 从 24 改为 4，在仅持有一帧时没有观察到驱动 GTT 计数下降，在尝试持有八帧时第五次申请失败；改为 0 是动态分配，不是固定容量优化。当前[生产硬件上下文](../qt/src/recording/libav/libav_hw_encoder_context.cpp#L142) 保持 24，并仍通过 [`av_hwframe_transfer_data`](../qt/src/recording/libav/libav_hw_encoder_context.cpp#L179) 上传。该单设备结果不能换算为通用显存峰值，更不能支持直接发布缩池改动。

### 8.3 更新后的推荐

保留现有 C++ 采集适配器和 FFmpeg 编解码、封装流程，不为已解决的问题再建一套 Rust 核心。若目标是 GIF 画质或动态内容性能，可分别试验量化器、调色板策略与索引写出器：`imagequant` 是局部量化候选，`gif` 是基础编码候选，`gifski` 是高质量完整 GIF 管线候选，三者不是同层替代品。[S6][S8][S10]

GPU 路线仍有减少真实搬运量的可能，但应按设备验证裁剪、色彩转换、表面共享和生命周期；它并不要求先迁移 Rust。只有在同一输出正确性、质量和资源预算下，Rust 替代实现优于当前改良版的结果，才可计为语言迁移方案带来的运行收益；类型约束、维护性与性能收益应分别记录。

## 9. 检索方法与证据范围

执行了 `smart-search doctor --format json` 和 `smart-search deep ... --format json`。通用搜索后端因 TLS 错误不可用；Context7、Exa、Tavily 检查通过，研究继续使用同一 CLI 的官方文档检索与页面提取，没有改用原生网络搜索。

关键命令形式：

```sh
smart-search context7-library ffmpeg "AVFrame reference counted hardware frames" --format json
smart-search context7-docs /websites/ffmpeg_documentation "palettegen paletteuse AVFrame" --format json
smart-search exa-search "gifski Rust library license memory frame collector imagequant official" --include-domains github.com docs.rs --format json
smart-search fetch "https://ffmpeg.org/doxygen/trunk/group__lavc__encdec.html" --format markdown
smart-search fetch "https://docs.rs/gifski/1.34.0/gifski/collector/struct.Collector.html" --format markdown
```

Context7 初次检索包含大量无关的同名 frame 库，未作为证据；仅选用明确的 FFmpeg 文档库。FFmpeg `group__lavu__frame.html` 页面抓取失败后改读官方 `frame.h`；PipeWire 无版本路径和 1.6 路径返回 404，本文使用实际读取成功的 1.4.11 文档，不将其标为最新版。

证据日期不等于所有资料的发布日期。标为 `master`、`main` 或 `trunk` 的链接可能更新；迁移试验应再锁定实际 crate 版本与上游提交。

阶段 1～4 后的增量核验复用了 `/tmp/recording-primary/` 中已抓取的官方文档、源码和 LICENSE，并对照当前工作区的具体实现及优化记录；未重新查询“最新版本”，也未把旧资料版本号当作更新检查结果。

## 10. 一手资料索引

- [S1] FFmpeg send/receive 编码解码接口：https://ffmpeg.org/doxygen/trunk/group__lavc__encdec.html
- [S2] FFmpeg `AVFrame` 官方头文件：https://github.com/FFmpeg/FFmpeg/blob/master/libavutil/frame.h
- [S3] Rust 官方所有权说明：https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html
- [S4] FFmpeg `palettegen`、`paletteuse` 参数：https://ffmpeg.org/ffmpeg-filters.html#palettegen 、https://ffmpeg.org/ffmpeg-filters.html#paletteuse
- [S5] FFmpeg `paletteuse` 实现：https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/vf_paletteuse.c
- [S6] `gif` 0.14.2 `Frame` 文档：https://docs.rs/gif/0.14.2/gif/struct.Frame.html
- [S7] `gif` Cargo 与 MIT 许可证：https://github.com/image-rs/image-gif/blob/master/Cargo.toml 、https://github.com/image-rs/image-gif/blob/master/LICENSE-MIT
- [S8] libimagequant 官方说明：https://github.com/ImageOptim/libimagequant/blob/main/README.md
- [S9] libimagequant 版权与 GPL 条款：https://github.com/ImageOptim/libimagequant/blob/main/COPYRIGHT
- [S10] gifski 官方说明：https://github.com/ImageOptim/gifski
- [S11] gifski 1.34.0 Collector：https://docs.rs/gifski/1.34.0/gifski/collector/struct.Collector.html
- [S12] gifski 1.34.0 管线源码：https://docs.rs/gifski/1.34.0/src/gifski/lib.rs.html
- [S13] gifski AGPL 许可证与另行许可说明：https://github.com/ImageOptim/gifski/blob/main/LICENSE
- [S14] FFmpeg 硬件帧上下文、transfer/map：https://ffmpeg.org/doxygen/trunk/hwcontext_8h.html
- [S15] PipeWire 1.4.11 DMA-BUF 协商与映射限制：https://docs.pipewire.org/1.4/page_dma_buf.html
- [S16] Microsoft Desktop Duplication API：https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
- [S17] NVIDIA Video Codec SDK 13.0 NVENC 编程指南：https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-video-encoder-api-prog-guide/index.html
- [S18] ffmpeg-next 仓库与导出接口：https://github.com/zmwangx/rust-ffmpeg 、https://docs.rs/ffmpeg-next/latest/ffmpeg_next/
- [S19] ffmpeg-next Cargo 定义：https://github.com/zmwangx/rust-ffmpeg/blob/master/Cargo.toml
- [S20] CXX 不透明 C++ 类型与线程安全要求：https://cxx.rs/extern-c++.html 、https://cxx.rs/bindings.html
- [S21] FFmpeg 官方许可证与专利说明：https://ffmpeg.org/legal.html
- [S22] ffmpeg-next 许可证：https://github.com/zmwangx/rust-ffmpeg/blob/master/LICENSE
