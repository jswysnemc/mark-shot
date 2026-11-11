# Windows / macOS 原生录制链路调研

调研日期：2026-11-11。仅调查，不修改 Mark Shot 代码。资料来自 Microsoft Learn、Apple Developer 与官方示例仓库；原始抓取保存在 `/tmp/mark-shot-recording-evidence/platforms/`。本报告没有运行 Windows 或 macOS 硬件录制基准，所有性能与体积收益都是架构判断，不能冒充实测。

## 结论

对 Mark Shot 的目标，Windows 建议 **Windows.Graphics.Capture → 同适配器 D3D11 纹理池 → GPU 裁切/缩放/色彩转换 → Media Foundation 硬件编码 → MP4**；macOS 建议 **ScreenCaptureKit → IOSurface/CVPixelBuffer → VideoToolbox → AVAssetWriter**。这两条路径依赖系统框架，可避免在基础安装包额外携带完整 FFmpeg、Qt Multimedia 或通用图形运行时。实际增量包体积仍须由最终链接产物测量，不能给出未经构建的 MB 承诺。

Windows 的 WGC 官方录制示例将捕获纹理复制到另一个 D3D surface，以避免编码期间采集缓冲被覆写。因此合理目标是“常态无整帧 GPU→CPU 回读”，不是“端到端零复制”。macOS 的 IOSurface 共享同样不能证明编码器内部没有转换、复制或缓存。不要把使用硬件编码接口等同于实际硬件编码成功。

macOS 15 的 SCRecordingOutput 很适合简单录制，但它公开的配置主要是输出 URL、容器和编码类型等，不能据此保证与跨平台质量预设、暂停续录、动态标注合成、独立音轨、精确码率控制和恢复策略完全对齐。Mark Shot 若需要这些能力，应把自管 VideoToolbox + 写文件链路作为完整实现，SCRecordingOutput 作为可选简化实现，而不是让上层业务只能调用这一种 API。

## Windows

### 采集与兼容边界

| 能力 | 一手资料确认 | 建议 |
| --- | --- | --- |
| WGC 捕获屏幕或窗口 | 返回 D3D11 surface、ContentSize 和 SystemRelativeTime；先调用 IsSupported | 默认后端，运行时探测失败原因 |
| WGC 最低版本 | FramePool Windows 10 1803；CreateFreeThreaded 从 1809 起 | Rust 的后台采集线程使用 free-threaded pool，避免 UI 线程重活 |
| HWND/HMONITOR 互操作 | CreateForWindow 文档规定 Windows 10 1903 / build 18362 | Mark Shot 已有窗口/屏幕选区可以交给 interop；建议现代路径基线 1903+，且检测方法存在 |
| Desktop Duplication | 输出级 API；返回 GPU surface、脏矩形/移动矩形；必须自行处理旋转与独立鼠标形状 | 屏幕捕获兼容后端，不把它当作完整窗口捕获方案 |
| DDA 设备关联 | D3D device 必须在显示输出所在 adapter 上创建；桌面切换、断连、并发限制可能失败 | 跟踪 adapter LUID、输出变化，重建会话，必要时受控回退 |
| 旧系统 | DuplicateOutput 虽有 Windows 7 Platform Update 元数据，但文档明确该环境返回 E_NOTIMPL | 不宣称 Windows 7 可使用 DDA |

来源：
- [WGC capture guide](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)
- [Direct3D11CaptureFramePool](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool)
- [CreateForWindow](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createforwindow)
- [Desktop Duplication API](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api)
- [DuplicateOutput](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgioutput1-duplicateoutput)

**不建议的默认链路：** 每隔 16ms 调用现有截图入口、把 BGRA 拷贝到 Rust Vec，再经管道喂给编码进程。这会把目前静态截图架构的 CPU 像素所有权扩展成持续整帧复制，失去原生纹理接口优势。可以作为受限兼容降级，不能作为“高性能”主路线。

### 编码与缓冲所有权

Windows 官方录制示例的高层链路为 WGC → MediaStreamSource → MediaTranscoder，并显式使用 D3D copy 保留异步编码所需的 frame。Mark Shot 可采用 Media Foundation Sink Writer 或直接使用 MFT 以减少 WinRT 媒体抽象层，但此处是实现选择，不是官方性能比较结论。[官方录制示例](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture-video)

关键点：

1. **归还捕获帧。** WGC frame 归还池后不能继续保留其底层 surface 引用。异步编码需要按完成信号管理自己的可复用纹理池，不应无限保留 WGC frame。[WGC capture guide](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)
2. **启用硬件 MFT。** Source Reader / Sink Writer 默认不使用硬件编码器/解码器，创建时设置 `MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS=TRUE`。这只允许选择硬件 MFT，不保证最终路径一定是硬编。[硬件 MFT 开关](https://learn.microsoft.com/en-us/windows/win32/medfound/mf-readwrite-enable-hardware-transforms)
3. **共享 D3D device。** `MF_SINK_WRITER_D3D_MANAGER` 接受 `IMFDXGIDeviceManager`，将设备交给编码器与 sink；`MFCreateDXGISurfaceBuffer` 能包装 D3D 纹理。不能仅给 Sink Writer 一个 CPU buffer 后声称无回读。[D3D manager](https://learn.microsoft.com/en-us/windows/win32/medfound/mf-sink-writer-d3d-manager)、[DXGI surface buffer](https://learn.microsoft.com/en-us/windows/win32/api/mfapi/nf-mfapi-mfcreatedxgisurfacebuffer)
4. **同显卡优先。** `MFTEnum2` 配合 `MFT_ENUM_ADAPTER_LUID` 可列举指定 adapter 的硬件 MFT；双显卡机器要用此能力减少跨 adapter 转移。即使是同 GPU，还需查询 encoder 支持的输入格式与实际纹理绑定要求。[MFTEnum2](https://learn.microsoft.com/en-us/windows/win32/api/mfapi/nf-mfapi-mftenum2)
5. **硬编不是颜色转换。** 硬件编码、硬件缩放/转换是分开的能力，系统在缺少视频处理器时可能使用软件转换。默认 SDR 路径建议 GPU 转为编码器实际接受的 NV12，再送入 encoder；BGRA 是否可直接交付必须探测。原生接口可避免带入新的通用 GPU UI 框架。[Hardware MFTs](https://learn.microsoft.com/en-us/windows/win32/medfound/hardware-mfts)
6. **真实能力探测。** 列举后实际创建“指定分辨率 + 帧率 + profile + pixel format + rate control”短会话，并输出少量帧，验证 callback、SPS/PPS 和首个 keyframe。记录编码器名称、adapter LUID、失败 HRESULT。该条为工程建议。
7. **异步与有界队列。** 硬件 MFT 使用异步处理模型；输入队列必须有上限，背压时丢旧视频帧保留最新帧或降采样，不阻塞音频回调。最终播放时间仍按真实 PTS，不允许用收到帧数除以目标 FPS 来伪造时间。[MFTEnumEx](https://learn.microsoft.com/en-us/windows/win32/api/mfapi/nf-mfapi-mftenumex)

Media Foundation 已有 H.264 encoder 和 MPEG-4 sink；H.264 encoder 的 ICodecAPI 提供 CBR、VBR、quality、GOP/B-frame 等参数，但具体硬件 MFT 的可选参数不同，因此质量预设需要按能力映射。不能把不同厂商的 QP、quality=70、CRF=23 当作同一画质。MP4 sink 支持 H.264/AAC；fragmented MP4 自 Windows 8 可用。注意“支持 fragment”不能自动推导到所有异常终止文件均可恢复，应做断电/强杀验证。[H.264 encoder](https://learn.microsoft.com/en-us/windows/win32/medfound/h-264-video-encoder)、[MPEG-4 file sink](https://learn.microsoft.com/en-us/windows/win32/medfound/mpeg-4-file-sink)

### 音频与时钟

- WASAPI loopback 捕获指定 render endpoint 的混音，使用 shared mode；Windows 10 1703 起支持 event-driven loopback。它不需要用户手工启用厂商“立体声混音”设备。麦克风使用独立 capture endpoint。默认系统声音与麦克风不能混为一个设备枚举项。[WASAPI loopback](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)
- 仅录指定进程/进程树是另一个能力：Microsoft ApplicationLoopback 示例要求 build 20348+，不能宣传为所有 Windows 10 都可用。可包含或排除该进程树，且不绑定某个 endpoint。[进程 loopback 示例](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/)
- WGC `SystemRelativeTime` 基于 QPC；`IAudioCaptureClient::GetBuffer` 返回的 QPC position 已转换成 100ns 单位，不是原始 QPC ticks。应统一 monotonic 时钟起点，避免混用单位；音频设备时钟漂移需通过缓慢重采样或其它受控校正处理。GetBuffer 还提供 silent/discontinuity 标记，应补静音并记录间断，而非把缺少音频包当作结束。[WGC guide](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)、[GetBuffer](https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudiocaptureclient-getbuffer)

### HDR、授权和失败恢复

- HDR 桌面不能简单截为 8bit BGRA 后按 SDR 编码。Microsoft 推荐 HDR 捕获链路使用 R16G16B16A16_FLOAT；若输出 SDR，要做显式 tone mapping；如果输出 HDR，还要匹配编码 bit depth、transfer function、primaries 和容器 metadata。[WGC HDR 提示](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)
- DDA 常规 DuplicateOutput 的固定 BGRA8 描述不等于 HDR 保真承诺；若专门使用 DuplicateOutput1，应另行查证驱动和格式能力。
- WGC 标识边框可以受用户授权与其它应用状态影响；`IsBorderRequired=false` 不是无条件消除系统标识的承诺。Mark Shot 默认应接受系统隐私指示，不为减少录制边框而强制更改权限。[IsBorderRequired](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.isborderrequired)
- 锁屏、安全桌面、目标关闭、设备丢失、拔显示器、音频设备切换均要停止/恢复状态机处理；不能无限重试同时持续生成黑色视频。

## macOS

### API 版本与能力边界

| 能力 | 官方可用版本 | 产品边界建议 |
| --- | --- | --- |
| SCStream / ScreenCaptureKit 视频流 | macOS 12.3+ | 12.3/12.x 可以视频录制，但系统声音需另外方案 |
| `capturesAudio` 系统/应用音频 | macOS 13.0+ | 若默认要求原生系统声音，建议最低 macOS 13 |
| SCStream microphone output | macOS 15.0+ | 13/14 若需麦克风，单独使用 AVFoundation/CoreAudio 输入 |
| `SCRecordingOutput` | macOS 15.0+ | 可选简化直接落盘路径 |
| `captureDynamicRange` | macOS 15.0+ | HDR 单独探测；不得仅以系统版本推定硬件支持 |
| VideoToolbox 强制硬件编码开关 | macOS 10.9+ | 运行时仍可能因 codec、尺寸、资源占用失败 |

来源：[SCStream](https://developer.apple.com/documentation/screencapturekit/scstream)、[capturesAudio](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/capturesaudio)、[captureMicrophone](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/capturemicrophone)、[SCRecordingOutput](https://developer.apple.com/documentation/screencapturekit/screcordingoutput)、[captureDynamicRange](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/capturedynamicrange)、[RequireHardwareAcceleratedVideoEncoder](https://developer.apple.com/documentation/videotoolbox/kvtvideoencoderspecification_requirehardwareacceleratedvideoencoder)

Apple 文档当前混有较新平台/预发布版本元数据，报告只使用明确的 macOS 可用版本，不将新 API 当成所有旧系统已经支持。

### 推荐链路与低占用设计

1. `SCContentFilter` 选择 display/window/application；使用 `sourceRect` 和输出 width/height 在采集框架完成区域和尺寸控制，不先录整屏再 CPU 裁切。录制 UI 与 Mark Shot 自身窗口可通过 filter 排除，但应验证 popup/独立进程归属。
2. `SCStreamConfiguration` 设置目标帧间隔、像素格式、cursor、音频开关；SDR 编码优先请求 YUV420 bi-planar，例如 420v，省去自己执行 BGRA→YUV。Apple 官方说明采集、缩放、pixel/color conversion 可由硬件加速。[WWDC22 advanced session](https://developer.apple.com/videos/play/wwdc2022/10155/)
3. 获取 `CMSampleBuffer`，取其 IOSurface-backed `CVPixelBuffer` 交给 VideoToolbox；不要转 NSImage、PNG、CPU Vec 再编码。按完成回调及时释放输入，不要以“留住上一帧”无限占用 ScreenCaptureKit surface 池。[官方 capture sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos)
4. `queueDepth` 默认 3，官方建议不超过 8；更多 surface 是用内存换抗抖动，不能视作越多越好。Apple 的解释：处理时长应小于 minimumFrameInterval，surface 持有时长必须控制在 interval × (queueDepth−1) 内，避免池耗尽后掉帧。[同上；WWDC22](https://developer.apple.com/videos/play/wwdc2022/10155/)
5. VideoToolbox 创建时可用 `RequireHardwareAcceleratedVideoEncoder` 表达严格硬编策略，失败后再决定显式软件降级；也可启用硬编并通过 `UsingHardwareAcceleratedVideoEncoder` 查询实际结果。接口允许硬编不意味着一定选中。[强制硬编](https://developer.apple.com/documentation/videotoolbox/kvtvideoencoderspecification_requirehardwareacceleratedvideoencoder)、[实际硬编状态](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_usinghardwareacceleratedvideoencoder)
6. `RealTime=true` 是实时性建议，不是固定延迟保证。适量 frame delay/reorder 应结合本地录制质量与暂停/停止排空需求选择，而非盲目照搬直播低延迟预设。[RealTime](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_realtime)
7. 编码结果交给 AVAssetWriter 写 MP4/MOV；如果使用已压缩的 VideoToolbox 输出，不再让 writer 二次编码。业务层负责 pause 去除的时间、audio mixing、PTS/DTS、finish/drain、磁盘满和原子交付。[AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter)

### 单窗口、动态尺寸与音频

Apple 的 ScreenCaptureKit 单窗口捕获支持被遮挡、离屏、跨显示器的窗口内容；最小化时该窗口流会暂停，恢复后继续。官方明确不建议随着源窗口频繁 resize 而不断改变输出尺寸，这会增加内存分配。因此建议录制画布固定，源内容缩放/letterbox；用户主动修改输出分辨率可创建新片段，而不是改变同一视频轨尺寸。[WWDC22](https://developer.apple.com/videos/play/wwdc2022/10155/)

音频过滤以应用为级别：单个窗口 filter 会捕获所属应用的其它窗口音频，不保证与视频窗口一一对应。macOS 15 的 SCStream 可以分别返回 screen、system audio、microphone；是否混音、单独轨道和格式必须由选用的写出路径明确约束。[WWDC22](https://developer.apple.com/videos/play/wwdc2022/10155/)、[WWDC24](https://developer.apple.com/videos/play/wwdc2024/10088/)

### SCRecordingOutput 的取舍

优点：macOS 15 起，向 stream 添加 recording output 即可直接写文件，配置输出 URL、file type、video codec，系统负责 asset writing；有开始、错误、结束 delegate，只有结束通知后才能把文件视作完成。[WWDC24](https://developer.apple.com/videos/play/wwdc2024/10088/)

局限：当前公开属性列出 `availableOutputFileTypes`、`availableVideoCodecTypes`、`outputFileType`、`outputURL`、`videoCodecType`，以及较新的 `mixesAudioWithMicrophone`，但没有完整跨平台码率/QP/暂停时钟/自定义 GPU 覆盖层控制。必须对每个可选属性再做版本检测，不能因为 class 是 macOS 15 就推断所有后加属性也在 15 中存在。[SCRecordingOutputConfiguration](https://developer.apple.com/documentation/screencapturekit/screcordingoutputconfiguration)

**未证实项：** 本轮公开文档不足以完整确认所有 macOS 小版本、Intel/Apple Silicon 对 HDR 直接录制与 SCRecordingOutput 的组合限制。不把“可以捕获 HDR”等同于“任何机器都可通过 SCRecordingOutput 保存 HDR”；发布前应以 SDK headers、运行时能力与真实 HDR 文件检查确认。

### 权限与分发

ScreenCaptureKit 官方示例首次启动请求 Screen Recording 权限，授权后示例要求重启应用；工程需把拒绝、尚未决定、授权后重新连接区分开。麦克风另有权限与 usage description；沙盒、签名、公证、最低系统版本、Apple Silicon/Intel 切片是 macOS 分发工作，而不是添加一个录制 crate 就自动完成。[官方 sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos)

系统框架不需要像 FFmpeg 一样随包携带整套 codec 二进制。可将 Objective-C/Swift 桥接控制在很小的 ABI 边界，业务状态与时间戳契约继续使用 Rust。此处为工程建议；不能据此宣称增量为零，桥接代码、符号、Universal Binary 两个 slice 均有体积成本。

## 跨平台统一契约建议

公共层应保存录制目标、质量意图、暂停状态、输出策略与观测数据，平台层应持有原生 GPU frame handle，不把所有平台的 frame 统一为 `Vec<u8>`。

- `VideoFrame`：原生 surface handle、尺寸、pixel format、color primaries/transfer/matrix/range、monotonic PTS、可选 dirty regions、release lease。
- `AudioPacket`：采样格式、采样率、声道布局、sample count、源时钟位置、PTS、silence/discontinuity。
- `EncoderCapabilities`：实际 codec/profile/bitdepth/input format/maximum tested mode、硬编是否使用、GPU ID、rate-control 选项。
- `EncodedPacket`：PTS、DTS、duration、keyframe、codec extradata、payload。
- `RecordingMetrics`：采集/编码/写入 FPS、丢帧和原因、队列峰值、CPU/RSS/显存、结束排空耗时、audio drift、输出字节数。

上层“高质量”是意图而不是统一数值。不同编码器把预设映射到自己的参数，测试结果决定默认值。录制路径只在启动录制后加载 GPU/编码资源，普通截图和托盘空闲不创建录制线程或纹理池。

## 必须验证，不能仅靠文档下结论

1. Windows Intel/AMD/NVIDIA 各至少一机，单 GPU/混合显卡、4K/多屏/旋转屏、HDR→SDR、窗口最小化、锁屏/解锁、device lost。
2. Windows 系统音和麦克风的 30 分钟同步；静音系统、设备切换、采样率不一致、蓝牙切换、音频包 discontinuity。
3. macOS 13/14/15+ 分层测试；Apple Silicon 与 Intel（若声明支持）；Screen Recording 拒绝/授予、系统 picker 与自选内容 filter、retina scaling、跨显示器和最小化。
4. 1080p30/60、1440p60、4K30/60 以实际编码成功和画质决定可用，不以 API 枚举名称判断。
5. 静止 IDE 文本、快速滚屏、视频播放、游戏/动画分别测 CPU 总和、RSS+辅助进程、GPU video engine、power、Dropped frames、文字清晰度。跨 API 公平对比必须相同输出分辨率/FPS/codec/码率或目标质量、相同内容和时长。
6. 强杀编码进程、磁盘写满、暂停/恢复 100 次、停止时仍有在途帧、多次开始/停止；验证文件可播、时间轴无跳变、资源回收。
7. 用平台 profiler 证明没有整帧 readback 和隐藏软件转换；仅观察低 CPU 或显示“硬件加速”不足以证明。

## 复现命令与资料限制

已执行 `smart-search context7-library 'Microsoft Media Foundation Windows Graphics Capture' 'D3D11 surface hardware video encoding screen recording' --format json`，再运行 `context7-docs /microsoft/media-foundation ...`。该仓库检索更偏向解码示例，**没有**拿解码的 DXVA 路径当作编码证明；最终编码结论来自精确官方 API 页面。

官方页面通过 `smart-search exa-search ... --include-domains learn.microsoft.com --include-text` 发现，并用 `smart-search fetch URL --format markdown` 抓取。Apple JS 文档若空正文，使用其 `.md` 官方表示补充，如 UsingHardwareAcceleratedVideoEncoder。

综合搜索 provider 的 TLS 错误属于检索服务问题；Exa、Context7 与 Tavily fetch 正常。本轮没有改服务配置，没有使用二手搜索摘要作为关键结论依据，没有测量平台录制性能。
