# Mark Shot 原版 Qt 录制与 Rust 恢复接口核查

核查日期：2026-11-11。Qt 参考目录 `/home/snemc/workspace/mark-shot/qt`，对应仓库 HEAD `934d8f3`；Rust 当前目录 `/home/snemc/workspace/mark-shot-little`，HEAD `e8c8966`。本次只读源码和既有文档，没有执行录制性能实验，也没有修改仓库文件。

## 主要结论

1. Qt 录制并非简单启动 ffmpeg 命令：已经有按需加载的 libav 后端、平台采集流、有限帧队列、提前限帧、背压、暂停时钟、音频、GIF 调色板缓存、独立控制条和单实例命令。恢复应参考这些行为和测试契约，而不是只补一个录制按钮。
2. Qt 的主要性能余地在数据路径：Windows WGC 虽以 GPU 纹理采集，随后仍读回 CPU；PipeWire DMA-BUF 仍 `glReadPixels`；VAAPI/QSV 又将 CPU NV12 上传 GPU。将 C++ 包装改成 Rust 绑定不会消除这些搬运。
3. 原版已有深度评估和优化记录，不能把已经落地的有界队列、限帧、PAL8 重用再当作新增收益。所有历史数字必须注明日期、构建和局部测试边界。
4. Rust 当前持续捕获主要为长截图服务：一次明确请求取一帧，返回紧密 RGBA `Vec<u8>`，Portal 每次交付后停流。这个接口适合复用授权和目标管理，不宜直接作为高帧率 GPU 录制接口。
5. 当前 Rust 已有 Linux Wayland、X11、Windows 后端。旧 `docs/architecture.md`/`rewrite-status.md` 中“无 X11”是过期描述，必须以源码为准。macOS 仍没有平台工厂；Qt 中也没有确认完整 ScreenCaptureKit/VideoToolbox/CoreAudio 实现。
6. 小体积不等于不使用 FFmpeg：原版已提供裁剪脚本，历史文档有“六个 FFmpeg 运行库合计约 5.57 MiB”的实测，但明确不含 x264、Qt、驱动和完整包。该数不能直接作为 Rust 新版本增量承诺。

## 原版功能与配置对照

| 能力 | 已确认内容与边界 | 证据 |
|---|---|---|
| 模式 | Video 与 GIF；录制启动对话框可切换模式 | `qt/src/recording/recording_options.h:12`；`recording_config_layout.cpp:34` |
| 范围 | Region、Display；多屏时提供 All Displays。WGC 显式拒绝全输出录制，依赖回退，并不代表所有后端原生都可跨屏合成 | `recording_options.h:17`；`recording_display_source.cpp:50`；`recording_windows_wgc_capture_stream.cpp:44` |
| 帧率 | 视频 15/24/30/48/60，默认30；GIF 6/8/10/12/15/20/24/30，默认12。配置解析允许1–120，但下拉选项之外的值会回退默认，不能宣称UI完整提供120 | `recording_config_options.cpp:86`；`recording_dialog_config.cpp:108` |
| 倒计时 | UI Off/3/5 秒；配置读取夹紧0–10秒，UI只列三项 | `recording_config_options.cpp:169`；`recording_dialog_config.cpp:166` |
| 容器 | MP4、MKV；GIF单独路径。MKV文案表示中断可播放，实际不应承诺任意损坏都可恢复 | `recording_config_options.cpp:131`；`recording_container_format.cpp:5` |
| 质量 | Balanced、High、Efficient；CRF/CQ/QP 23/18/28，软件preset和硬件preset随48fps分界变化；不同编码器同数值不能视为同等质量 | `recording_quality_options.cpp:5`；`libav/libav_video_encoder_setup.cpp:69` |
| 视频编码 | Windows候选 H.264 NVENC→AMF→QSV→MF；Linux NVENC→VAAPI→QSV；软件候选x264→mpeg4。无HEVC/AV1候选 | `recording_video_encoder_options.cpp:65`、`:89` |
| 音频 | 可开关。Linux PulseAudio source枚举含monitor；Windows固定默认render端点 WASAPI loopback，忽略deviceName，未见Windows麦克风选择或麦克风+系统音混合 | `audio/audio_input_device_list.cpp:40`；`audio/wasapi_audio_capture_reader.cpp:320`、`:372`、`:428` |
| 音频格式 | 一个立体声AAC流；输入float PCM，经swresample转换；输入AudioCaptureSample只有pcm+sequence，没有时间戳字段 | `libav_audio_encoder.cpp:109`、`:237`；`audio/audio_capture_sample.h:9` |
| 暂停恢复 | 同步抑制视频采集、停止音频reader、暂停心跳，视频时间轴扣除暂停累计时长 | `recording_controller.cpp:93`；`recording_pause_state.cpp:48`；`libav_recording_process.cpp:412` |
| 保存与通知 |输出目录、扩展名规范化、时间戳文件名；启动、成功、失败通知；启动先预检编码后端 | `recording_file_naming.cpp:35`；`recording_session_manager.cpp:25` |
| 控制条 |区域录制显示轮廓和暂停/停止/时长；全屏没有安全放置空间时不显示控制条，使用托盘与快捷键；刷新500ms | `ui/recording_overlay_service.cpp:14`、`:115`；`ui/recording_control_bar.cpp:122` |
| 托盘 | Start Recording、Recording状态、Pause/Resume、Stop | `qt/src/windows_tray_controller.cpp:288` |
| 命令 | `--recording-status` JSON、`--stop-recording`、`--pause-recording`；启动工具GifRecorder/VideoRecorder接入区域选择 | `qt/src/main.cpp:117`；`qt/src/cli/recording_cli.cpp:39`；`qt/src/shot_window_recording.cpp:34` |
| 配置存储 | `recording.dialog`，兼容`recorder`及`startDialog/ui`；保存scope/backend/container/quality/videoFps/gifFps/countdownSeconds/includeAudio/audioDevice/outputDirectory/displayKey | `recording_dialog_config.cpp:141`、`:182` |
| 捕获后端 | Auto/wlroots/PipeWire/WindowsWgc/Polling；Auto顺序wlroots→PipeWire→WGC→Polling；采集首帧前失败可回退，首帧后失败结束会话 | `recording_capture_backend.cpp:88`；`recording_frame_grabber.cpp:199` |
| 运行时编码回退 | 打开失败逐候选尝试；初始写入失败只允许启动窗口内重建候选，并非整个会话任意时刻无损切换 | `video_recording_writer.cpp:19`、`:65`、`:143` |

上述相对 `recording/` 路径均相对 `/home/snemc/workspace/mark-shot/qt/src/recording/`，表中显式qt路径例外。

## Qt 数据路径与性能事实

### GPU读回及重新上传

- Windows `qt/src/platform/windows/windows_wgc_stream.cpp:88` 建立D3D11_STAGING，`:166` CopyResource完整纹理，`:169` Map，`:181`逐行memcpy到BGRA池。因此区域录制也存在完整纹理readback机会；先GPU裁剪可减少搬运，但收益需Windows实测。
- PipeWire `qt/src/pipewire/pipewire_dmabuf_importer.cpp:118`裁剪后glReadPixels到CPU，`:581`完整图路径也读回。不是DMA-BUF直接进入硬件编码器。
- wlroots已可把SHM映射借给下游，`RecordingRawBgraFrame`有mappedOwner/mappedData而非必须拷贝；不能说每条Qt链都额外拷贝整帧。`qt/src/recording/recording_frame_payload.h:10`。
- 统一CPU BGRA→swscale YUV420/NV12，`qt/src/recording/libav/libav_video_scaler.cpp:205`起；VAAPI/QSV `libav_hw_encoder_context.cpp:200`调用av_hwframe_transfer_data。NVENC/AMF/MF当前允许系统内存输入，仍不等于保留原始GPU表面。
- 当前硬件帧池配置 `initial_pool_size=24`（同文件`:142`），不是24张必然驻留，也不是可以任意降至3的证据。

### 已经做过的优化

- `recording_frame_handoff.cpp:71`合并通知并保留最新样本，拒绝乱序timestamp/sequence。
- 默认编码队列96MiB，容量按帧尺寸1–4限制，48fps以上最多3；`recording_memory_policy.cpp:19`。该预算不是整个进程RSS上限。
- 采集预算默认96MiB，同时保证最大帧两帧容量；`recording_capture_memory_policy.cpp:9`、`:22`。预算允许因极大帧超过96MiB。
- Windows背压在读回前，`recording_windows_wgc_capture_stream.cpp:125`；WGC reader也提前pacer过滤。wlroots有delayBeforeNext、copy_with_damage，`wlroots_screencopy_worker_frame.cpp:176`、`:252`。
- swscale有常驻线程池，默认按核数2–4切片；`libav_video_scaler.cpp:35`。不能把“避免逐帧创建线程”说成待开发优化。
- 补帧复用已转换视频像素、已上传硬件表面；`video_recording_writer.cpp:172`和`libav_recording_process.cpp:355`。
- GIF有正sequence标识的单张PAL8缓存，同序号跳过转换和palettegen/paletteuse；`libav_gif_recording_process.cpp:359`。固定RGB8调色板未启用此缓存。
- 视频时间基1/fps、GOP=fps、max_b_frames=0；NVENC tune=ull，AMF usage=ultralowlatency。此偏低延迟策略对本地高质量录屏是否合适应重新评估，而非原样抄成最优。
- Efficient使用ultrafast+较高量化参数，“Smaller file”是产品标签，不能仅凭ultrafast就推导压缩效率最佳。画质/体积/CPU需要同源对照。

### 时间轴/音频需重点升级

`AudioCaptureSample`没有设备时间戳，AAC `m_nextPts`按样本数推进；视频使用独立单调时间/pacer。代码有暂停基本处理，却未在已读路径看到跨设备长期漂移反馈补偿。不能因使用swresample就声称已解决长时A/V drift。恢复设计应明确共同时间基、音频设备时钟映射、缺帧/静音补偿、暂停后首样本、设备热插拔和长时验收。

### 封装可靠性

`qt/src/recording/libav/libav_muxer.cpp:75`调用avformat_write_header(..., nullptr)，未见fragmented MP4 movflags；正常结束调用av_write_trailer。由此可确认普通MP4收尾依赖，不能将faststart误当抗中断。恢复设计可选MKV工作文件+结束重封装MP4，或明确分段/fMP4，但须验证目标播放器兼容与失败保留策略。

## 历史实验：可引用，但不能当本轮实测

原始资料：
- `/home/snemc/workspace/mark-shot/docs/recording-rust-assessment.zh-CN.md`
- `/home/snemc/workspace/mark-shot/docs/recording-optimization.zh-CN.md`
- `/home/snemc/workspace/mark-shot/docs/recording-memory.md`
- `/home/snemc/workspace/mark-shot/docs/performance.md`

可用事实与正确限定：

| 历史记录 | 正确解释 |
|---|---|
| 4K、30帧交接压力约966.51→80.08MiB | 有界通知消除排队持有帧；压力测试，不是完整桌面录制，不是Rust收益 |
| 4K GIF每四帧变化，24帧3801→1573ms，文件相同 | 跳过重复量化的收益；不能外推持续变化画面 |
| 软件4K/30均衡峰值约802MiB，自动线程约2255MiB | 编码器并发与preset主导大量内存；输入是合成编码压力，不含真实采集/音频/界面 |
| VAAPI pool24→4，持有一帧GTT相同；持有8帧时配置4在第5张失败 | 不应无依据将surface pool改为3/4，驱动分配与在途引用要测 |
| 裁剪FFmpeg8.0.1六个库合计5,837,584字节≈5.57MiB | 不含x264、Qt、驱动、ONNX和应用资源，非完整安装包；不能承诺新平台同大小 |

历史优化文档明确没有验证Windows真实WGC、GNOME真实Portal、完整A/V同步和长时录制。应保留这些边界，不能用“27项录制CTest通过”替代整平台真实录制验收。

## 打包与许可证边界

- `qt/cmake/recording_backend.cmake:12`建立MODULE，链接QtCore/Gui和FFmpeg库，开始录制时加载。
- `qt/src/recording/backend/recording_backend_loader.cpp:69` Windows使用PreventUnloadHint，原因是宽泛FFmpeg依赖ggml重复静态初始化问题；Linux最后租约释放后卸载。独立helper进程可提供更可靠的整体释放边界，但帧不要跨进程反复复制。
- `qt/scripts/build-minimal-ffmpeg.sh:77`用 `--disable-autodetect --disable-everything --disable-programs --disable-network --disable-avdevice`，只启用录制编码器、mp4/matroska/gif muxer、file协议、少量GIF滤镜；保留NASM/SIMD。
- 该脚本`:80`显式 `--enable-gpl --enable-version3 --enable-libx264`，`:144`保存GPLv3文本。当前Rust crate声明MIT OR Apache-2.0；不能直接沿用该二进制配置并把发行物称作纯MIT/Apache。具体分发义务需主调研查官方来源；本笔记仅陈述源码配置，不作法律结论。
- 可按平台重用系统编码框架或裁剪LGPL FFmpeg，软件fallback需单独决定。主程序小、首次下载小、安装总量小、启动RSS低是四个不同指标。

## Rust 恢复入口与缺口

| 现有模块 | 可复用 | 不应误用 |
|---|---|---|
| `src/backend/capture_session.rs:11` CaptureSession |固定输出、授权生命周期、关闭幂等、首帧验证 |next_frame返回CPU CaptureResult，无时间戳/GPU租约；不是现成实时录制API |
| `src/backend/wayland/capture/portal/session.rs` |CreateSession/SelectSources/Start/OpenPipeWireRemote会话、输出匹配 |只接受输出截图语义，录制窗口源/权限元数据还需扩展 |
| `src/backend/wayland/capture/stream.rs:15` |PipeWire连接、可取消worker |PipewireFrame仅Vec+尺寸+stride；worker在`:144`每次交付停流，下一请求`:172`才重新激活 |
| `src/backend/wayland/capture/stream/format.rs:6` |格式协商与错误处理经验 |仅BGRx/BGRA/RGBx/RGBA；无DMA-BUF modifier、plane、fence、native surface类型 |
| `src/backend/wayland/capture/stream/pixels.rs:9` |CPU fallback行距/负行距校验 |每帧新Vec并转换紧密RGBA；高fps录制不应固定强制经过这里 |
| `src/types/capture.rs:79` MonitorFrame |截图输出元数据与显示器映射 |无PTS、色域/范围/transfer/HDR、GPU/CPU联合载荷，不宜直接膨胀成万能帧 |
| `src/backend/windows/native/capture.rs:66` |GDI截图fallback |BitBlt路径不等于WGC/DXGI持续纹理流；需要新增录制专用后端 |
| `src/backend/platform/linux.rs:72` |实际有Wayland和X11平台选择 |旧架构文档排除X11已过时 |
| `src/backend/platform/mod.rs` |Linux/Windows隔离结构 |无macOS分支，macOS不仅是加一个encoder，还要窗口/托盘/权限/快捷键/构建签名 |
| `src/features/desktop/tray/menu.rs:26` |跨平台共享菜单模型 |当前明确非录像菜单，状态只含language/delayed，需录制状态机接入 |
| `src/features/desktop/actions.rs`、单实例IPC |已有动作分发和命令转发 |录制启动、暂停、停止、状态与capture互斥/并存策略要新增 |
| 软件UI、选区、窗口、配置、通知、保存路径 |可直接复用产品交互 |不能将截图编辑器完整像素缓存带进持续录制worker |

建议以独立 `features/recording` 状态机、录制帧与资源租约契约、各平台capture/convert/encode适配器、muxer/音频为边界。Rust共享业务层保留，GPU资源留在所属平台/设备线程，界面仅接收状态与降采样预览。该建议尚未实施。

## macOS 证据边界

Qt虽有 `.dylib` 文件后缀和loader分支，但当前采集工厂只列wlroots、PipeWire、WindowsWgc、Polling；encoder候选非Windows分支按Linux DRM/NVIDIA检测，音频工厂仅WASAPI/PulseAudio。未在这些职责模块发现ScreenCaptureKit、VideoToolbox或CoreAudio。因此“重新补齐macOS”应描述为新增平台能力，不是已确认原版功能的直译。

## 本轮未验证

没有重新编译Qt、运行真录制、测帧率/功耗/峰值内存、抓取网络资料、测精简FFmpeg最新包。所有源码结论对应当前本地版本；所有性能数值来自既有文档并按边界引用。
