# Mark Shot 录制恢复：包体积、编码画质、封装及依赖选型调研

调研日期：2026-11-11。只读源码和在线资料，未修改仓库、未编译录制引擎、未测量录制性能。以下“建议”是设计判断，“已核验”来自一手文档或源文件。网页原始证据保存在 `/tmp/mark-shot-recording-evidence/packaging/`。

## 1. 核心结论

建议采用**轻量主程序 + 按平台适配的录制引擎 + 可选兼容/导出组件**，避免把完整 FFmpeg/GStreamer/OBS 功能集无条件带入主程序。Windows/macOS 优先复用操作系统采集与硬件编码接口；Linux 以原生采集到 GPU 表面、VA-API/NVENC 等编码为高性能路径，具体适配交由平台报告。共用会话状态、时间轴、音频策略、能力探测和封装契约，不强求共用同一种像素缓冲。

“主安装包小”“首次录制无需下载”“所有平台/老硬件都支持”“统一依赖最少”无法同时无代价实现。使用系统库减少分发字节，不代表用户设备上没有这些依赖；按需组件减少首包，不代表完整安装总量更小；硬件编码降低 CPU，不代表 GPU、显存或总功耗一定最低。

对 Mark Shot 的具体建议：

- 默认交付保持现有截图程序轻量。录制器运行时按需初始化，停止后释放采集会话、编码器、音频和缓冲池。
- Linux 自带录制组件只编入需要的编码、复用与颜色/音频转换部分；优先固定自建版本并维护对应源码/构建清单。发行版包可依赖系统媒体库，便携包采用私有组件；两种分发分别测试。
- 单独进程适合隔离编码驱动异常、回收录制资源和按需组件生命周期，但跨进程不应经 stdin 每帧发送 BGRA；进程边界应传控制消息和系统可共享的 GPU/内存资源句柄，或让录制进程自行采集。
- H.264 + AAC + 可恢复写入的 MP4/MKV 作为兼容基线；HEVC/AV1 是明确探测硬件和目标播放器后的可选能力，不是启动即强制的“最佳”编码。
- GIF 放在停止录制后的导出任务，采用缩放、降低帧率、调色板和抖动选项，不在实时采集线程里做高成本 GIF 优化。

## 2. 依赖方案比较

| 方案 | 已核验证据 | 适合 Mark Shot 的判断 | 主要代价 |
| --- | --- | --- | --- |
| 系统原生媒体 API | `windows-capture` 当前源码经 `MediaStreamSample::CreateFromDirect3D11Surface` 交付表面，并设置 `MediaTranscoder` 硬件加速；无需外部 FFmpeg [S11] | Windows/macOS 的体积优先主路径；仍必须验证实际硬件编码器而非只看加速开关 | 平台代码、版本差异、编码器控制能力差异 |
| 最小化 libav* | FFmpeg 可关闭全部编码器/封装器再逐项启用，硬件封装仍可用 [S1][S2] | Linux 及需要统一高级封装/后处理时可用；组件清单按录制需求确定 | C ABI、版本固定、供应链和许可维护 |
| FFmpeg CLI 副进程 | `ffmpeg-sidecar` 是已有 FFmpeg 二进制的命令包装，不自带一个免成本编码器 [S10] | 兼容模式、导出 GIF、转封装、快速验证可用 | 原始像素管道复制、时间戳/背压、进程生命周期、完整二进制体积 |
| GStreamer | 官方支持共享、私有动态和私有静态部署；`gst-full-*` 可筛选插件与元素 [S6][S7] | 如果音视频图、设备协商、时钟/音频处理开发成本更关键，可作为整个录制引擎备选；不能简单贴“性能差”标签 | 引入 GLib/GObject 和插件体系；裁剪、扫描、跨平台打包与问题诊断成本 |
| libobs | `libobs/obs.h` 明确 GPL v2 or later [S8] | 可做实现与性能对照，不建议为单一录制功能直接嵌入 | 图形/混音/插件整体体系与 GPL 结合问题；不符合当前最小化方向 |
| scap | README 使用原生采集 API；当前 Cargo 标记 `0.1.0-beta.1`，frame API 暴露 `Vec` 像素及 CPU 转换 [S9] | 可参考平台权限和目标枚举，不可仅因“Rust + native”就认定满足无 CPU 回读目标 | 抽象边界是 CPU 图像；需要扩展原生表面交付，Linux/Windows依赖版本与当前项目也不同 |
| xcap | 维护者 README 明示视频录制 WIP、窗口录像待开发、Wayland 有特殊场景限制 [S12] | 截图/基础采集可评估；不能当作完整、跨平台等价的生产录制器替换方案 | 编码、音频、同步、恢复、硬件帧生命周期仍要自行解决 |
| windows-capture | 2.0.1 源码含 WGC/DXGI 和系统转码；`send_frame` 可以直接传表面 [S11] | 有价值的 Windows 平台候选或源码参考 | 当前编码器内部 `std::sync::mpsc::channel` 是无界队列；“立即返回”不等于有界低占用，慢编码/慢磁盘场景需审计 |

没有证据支持“用纯 Rust crate 自动比 C 媒体库小/快”，也没有本轮同构建数据支持给上述方案排序具体 MB、CPU 百分比或帧率。

## 3. FFmpeg 裁剪边界

**已核验：**FFmpeg 编码文档允许 `--disable-encoders` 后 `--enable-encoder=...`，封装同样支持 `--disable-muxers` / `--enable-muxer=...`；源码存在 `--disable-everything` 的统一组件开关 [S1][S2]。这证明能够裁剪，不证明任意“最小命令”能在各系统正确编码。

建议将能力分成三份，而非构建一个无所不包二进制：

1. 录制主路径：需要的 H.264/HEVC/AV1 硬件包装器、AAC（若操作系统不提供）、选定封装器、必要 bitstream filter；仅在确有用途时保留 libswresample / libswscale / libavfilter。
2. 软件兼容路径：单独可选软件编码组件，限制默认分辨率/帧率，明确显示已切换软件编码。不可在硬件失败后无提示切换极慢高质量软件 AV1/HEVC。
3. 导出路径：GIF 解码、缩放、帧率变换、palettegen/paletteuse、GIF 编码器和转封装功能，按需调用。

发布清单应记录：固定上游版本/commit、完整 configure/Meson 配置、所有动态依赖及最低系统版本、启用 encoder/muxer/filter 列表、对应许可证、源码和补丁、归档摘要、组件 ABI/协议版本。避免自动检测到开发机额外库后意外增加发布依赖。

不要把图形驱动、CUDA 工具链和所有 vendor SDK redistributable 当成默认运行时一起打包。硬件 API 的头文件/构建工具与真正的运行时依赖应分别列出；对驱动缺失、编码接口不支持、API 版本不匹配给出能力诊断。

**进程边界建议：**录制进程自身拥有采集源和编码器。主 UI 发送区域、帧率、音源、开始/暂停/停止等小消息，接收统计/错误。CPU 原始帧管道只用于明确的兼容路径。否则副进程虽然缩小了主程序常驻资源，仍可能因大规模 CPU 复制损害录制效率。

## 4. 许可与分发

Mark Shot 当前 `Cargo.toml` 标注 MIT OR Apache-2.0。FFmpeg 官方说明基本库为 LGPL 2.1+；启用 GPL 部分则该 FFmpeg 构建适用 GPL。官方 LGPL 清单建议动态链接、提供与二进制完全对应的源码、补丁和编译配置，并明确提及不能把 libx264 当作非 GPL 组件 [S3]。

FFmpeg 8.0 许可清单列出 libx264 和 libx265 的 GPL 要求；`--enable-nonfree` 组合可能产生不可再分发二进制 [S4]。本轮建议的默认可分发组件应避免这类不清晰组合，不能复制网上“all codecs”打包命令。

- 动态链接不是唯一合法方式，但静态链接的重新链接材料等义务更容易遗漏。GStreamer 官方私有静态部署文档也明确要求提供可重新链接所需文件 [S6]。
- 副进程隔离不应宣称“自动规避 GPL”，自动下载也不应宣称“无需履行组件分发义务”；应按实际分发、组合方式和依赖许可审核。
- “开源许可兼容”与 H.264/HEVC 等格式的专利/发行授权是不同问题。硬件或系统编码接口不等于项目获得了所有地域和商业场景的法律保证。此处是依赖工程约束，不作法律结论。
- libobs 明确 GPL，因此以其实现为参考和将其源码复制/链接到主程序是不同决策 [S8]。

## 5. 画质取舍：先确定输出目的

建议界面采用少量语义配置，后端映射为各编码器参数，不把 NVENC 的数值机械套到 VA-API、Media Foundation 或 VideoToolbox。

| 模式 | 建议行为 | 原因及限制 |
| --- | --- | --- |
| 兼容/通用分享 | H.264，8-bit 4:2:0，AAC，SDR；30 fps 默认候选，60 fps 显式选择 | 覆盖常见播放器；这是设计起点，需要跨系统导入/播放验证 |
| 省空间 | 硬件 HEVC 或 AV1；能力与播放端兼容通过后启用 | 不存在所有 GPU/驱动相同画质比；应以同质量文件大小比较，不能只比同码率 |
| 清晰文字/教程 | 原生像素分辨率，避免重复缩放；文字/线条场景选择更高质量，必要时提供 4:4:4 选项 | 4:2:0 对色度降采样，仅加码率不能恢复已经丢失的色度样本；4:4:4 编码/解码/编辑支持必须逐平台探测 |
| 长时间低占用 | 有界表面池、适中 preset、减少 lookahead/多遍；优先 30 fps；静止区域减少工作 | 低 CPU、低显存、低功耗不同维度，需同时测量 |
| HDR | 明确的 10-bit/P010、色彩元数据和转 SDR 策略，能力不足时提示或按已选策略转换 | “抓到 HDR 桌面”不代表最终编码颜色正确；第一期可以明确只提供 SDR |

NVENC 一手资料给出的可核验约束 [S5]：

- VBR target quality 在 `maxBitRate` 限制下维持目标画质；限制太低会达不到目标。不能同时保证任意内容的固定文件大小和固定质量。
- 相同条件下，两遍帧编码性能低于单遍；应按质量/性能测试选择。
- lookahead 缓冲未来帧以提高码率分配准确性，增加管线缓冲；不能在追求极低显存和低延迟时无条件开启最长 lookahead。
- Spatial AQ 可能改善主观质量但降低 PSNR，因此不能只用一个客观分数选择设置。
- Temporal AQ、部分色彩转换/质量增强功能可能使用 CUDA；“硬件编码”不意味着完全不占 GPU 计算资源。
- B 帧要求额外输入/输出缓冲；编码完成时必须 drain 排队帧，不能直接杀进程当作正常停止。

NVIDIA 官方说明 4:2:0 保存全亮度而色度保留原始色度内容的 25%，4:4:4 保留完整色度 [S13]。彩色细文字更容易暴露色度降采样，这是由采样结构得出的工程推论，不是本轮对 Mark Shot 的实测结论。

## 6. 质量验证不能只录一个视频

建议固定可重复输入矩阵：黑底白字终端、彩色 IDE、小字号中文、细线表格、网页滚动、60 fps 动画、全屏高运动视频、鼠标移动与点击、高 DPI 多屏。用无损原始参考和帧时间戳核对输出。

需记录：

- 视觉：原生像素观看、边缘和彩色文字色度误差、灰阶/渐变色带、截图裁切的细线完整性；SSIM/PSNR/VMAF 只能作补充，不能代替屏幕内容可读性检查。
- 时间：实际输入帧数、输出帧数、捕获/编码/封装各阶段丢帧、呈现时间分布、重复帧、音视频偏移及长时漂移。
- 成本：同一设备/驱动/场景下 CPU 使用时间、GPU encode/compute/copy 引擎、功耗、RSS/PSS、显存、编码队列深度、磁盘写入量与文件大小。
- 对比必须同时给“相同画质下的资源/体积”和“相同码率下的画质”，不同编码器 QP/quality 数值不能当成相同质量。

不应在没有真实硬件数据时承诺“4K60 只占 2% CPU”“录制模块固定 5 MB”“AV1 一定比 HEVC 小多少”。本轮没有运行这些基准。

## 7. 内存与体积的预算方法

原始帧大小属于可计算量，不是跑分：

- 3840×2160 BGRA：33,177,600 字节，即 31.64 MiB/帧；仅 3 个完整帧约 94.92 MiB。
- 同尺寸 NV12 4:2:0 8-bit：12,441,600 字节，即 11.87 MiB/帧；仅 3 个完整帧约 35.60 MiB。
- 3840×2160×4×60 = 1,990,656,000 字节/秒，约 1.85 GiB/s，仅代表一遍完整 BGRA 数据流的字节量；读写两端、颜色转换、中间缓冲、总线访问会增加真实流量。

因此“几十 MiB 总内存录 4K”应先解释表面数、内存归属和采样口径。共享 GPU 表面仍占显存/统一内存；不计主进程 RSS 不等于没有成本。用户侧统计要把录制子进程、媒体框架进程及 GPU 内存一起考虑。

包体积也应分四栏记录：主程序未压缩大小、下载归档大小、录制组件安装大小、完整安装（OCR 和录制均启用）总量。系统依赖清单另列。

本地已存在的 `docs/binary-size-analysis.md` 记录 2026-11-08 一个明确构建指纹的主程序 27,703,328 字节；它是历史发布构建，不是当前 HEAD 的新测量，也不能在该数字上凭经验加一个录制组件 MB 得出承诺。该文档已经证明按需 OCR 运行库减少首包但未保证完整安装总量更小，这个口径可直接延续到录制组件。

## 8. 容器、崩溃恢复与停止流程

**已核验：**FFmpeg 官方说明常规 MOV/MP4 把所有包元数据集中保存，通常结束时写入；碎片化 MP4 将数据和对应元数据分段写入，因此中断后仍可能解码，长录制期间元数据内存也更少，但应用兼容性较弱 [S1]。`+faststart` 是移动最终索引的第二遍处理，不是录制期间的崩溃恢复方案。

OBS 的 Hybrid MP4/MOV 是先碎片化写入、结束时 soft-remux 为常规布局；文档说明章节等元数据在 finalization 阶段写入，崩溃后不能保证恢复它们 [S14]。这是可借鉴的设计，不意味着可以忽略 libobs 源码许可直接复制实现。

推荐决策：

- 第一版选择成熟的 MKV 录制后转封装，或已验证的 fragmented MP4；对兼容编辑软件的最终输出再生成普通 MP4。
- 转封装只搬运压缩包而不重新编码，不会再次损失画质，但需要额外 I/O，若生成第二个完整文件会有峰值磁盘空间要求。
- 正常停止依次停止产生新帧、drain 编码器、写尾部/索引、关闭并检查 I/O、成功后将 `.partial` 文件改为正式文件。失败保留可恢复文件并显示具体原因。
- 进程崩溃和断电不能混为一谈。容器可恢复性保护的是已经写出的完整数据；用户空间缓冲、内核缓存、磁盘缓存尚未持久化的数据仍可能丢失。需要明确 `flush`/`fsync` 策略与性能代价。
- 分段/关键帧间隔可作为候选 1–2 秒后测试，但这不是已验证的“最多丢 2 秒”承诺；缓冲与持久化边界可能更大。
- 覆盖异常测试：磁盘满、权限丢失、暂停/恢复多次、编码器设备丢失、音频源消失、进程被终止、部分片段截断、长录制 2 小时以上、重启恢复。

## 9. GIF 恢复策略

FFmpeg 的 `palettegen` / `paletteuse` 明确支持全局/差分/逐帧调色板、抖动、只重处理变化矩形；调色板最多 256 色（透明模式会保留一项）[S15]。这说明 GIF 不应被当成等价的视频存储格式。

建议先保存一份可恢复的视频录制，再由用户选择 GIF 导出范围、尺寸、帧率和抖动。短片导出在独立任务运行，可以取消；源视频保留到导出完成，后续是否删除由现有产品保存策略决定。这个模式增加临时磁盘/停止后处理时长，却避免量化与抖动占用实时录制线程，并能重新导出不同参数。

若要求“停止立刻得到 GIF”，需单列短时、低分辨率模式，明确限制与资源上限，不能沿用 4K60 视频档位。

## 10. 本轮未验证项与下一步门槛

- 未实测任何录制候选引擎的 CPU/RSS/显存/功耗/包体积；没有跨平台可执行录制样机。
- `windows-capture` 的硬件加速布尔值不是实际硬件编码成功凭证；队列、表面池和慢磁盘背压需要原型审计。
- scap 当前检索主分支的 CPU frame API 已核验，但不代表其维护者不会提供其他分支/后续 GPU API。
- FFmpeg 和 GStreamer 最小构建取决于选择的后端、编码器、许可证和链接器，需固定版本实际构建后才能给体积预算。
- 不推荐把“纯 Rust 自写完整 muxer/音频同步/编解码”作为第一期省体积捷径；未经容器兼容与异常测试的少依赖实现会转移成本到正确性。

建议第一阶段仅建立两条可比较的原型：平台原生/GPU 编码路径与有界 CPU + FFmpeg 兼容路径，以同一测试素材和同一输出规范记录数据。能证明资源与画质收益后，再决定是否引入 GStreamer 或进一步自建封装层。

## 来源索引与复现命令

已抓取正文，不将搜索摘要当作唯一证据：

- [S1] FFmpeg Formats，MOV/MP4 fragmentation、muxer 选择与 `faststart`：https://ffmpeg.org/ffmpeg-formats.html
- [S2] FFmpeg 文档与 configure 源码：https://ffmpeg.org/ffmpeg-all.html ；https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.0/configure
- [S3] FFmpeg 官方 License and Legal Considerations：https://ffmpeg.org/legal.html
- [S4] FFmpeg 8.0 License：https://ffmpeg.org/doxygen/8.0/md_LICENSE.html
- [S5] NVIDIA Video Codec SDK 13.0 NVENC Programming Guide（3.8 rate control / multipass、3.9 buffers、8.2 lookahead、8.6 AQ 等）：https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-video-encoder-api-prog-guide/index.html
- [S6] GStreamer Deployment：https://gstreamer.freedesktop.org/documentation/deploying/index.html
- [S7] GStreamer Meson options： https://raw.githubusercontent.com/GStreamer/gstreamer/main/meson.options
- [S8] libobs 授权头：https://raw.githubusercontent.com/obsproject/obs-studio/master/libobs/obs.h
- [S9] scap 当前 Cargo 与视频帧源码：https://raw.githubusercontent.com/CapSoftware/scap/main/Cargo.toml ；https://raw.githubusercontent.com/CapSoftware/scap/main/src/frame/video.rs
- [S10] ffmpeg-sidecar 维护者文档：https://docs.rs/crate/ffmpeg-sidecar/latest
- [S11] windows-capture 2.0.1 Cargo 与 encoder 源码：https://raw.githubusercontent.com/NiiightmareXD/windows-capture/main/Cargo.toml ；https://raw.githubusercontent.com/NiiightmareXD/windows-capture/main/src/encoder.rs
- [S12] xcap 维护者 README：https://raw.githubusercontent.com/nashaofu/xcap/master/README.md
- [S13] NVIDIA Video Codec SDK 13.0 / Blackwell 色度说明：https://developer.nvidia.com/blog/nvidia-video-codec-sdk-13-0-powered-by-nvidia-blackwell/
- [S14] OBS Hybrid MP4 / Hybrid MOV：https://obsproject.com/kb/hybrid-mp4
- [S15] FFmpeg palettegen / paletteuse：https://ffmpeg.org/ffmpeg-filters.html#palettegen-1 ；https://ffmpeg.org/ffmpeg-filters.html#paletteuse
- [S16] GStreamer Licensing Advisory：https://gstreamer.freedesktop.org/documentation/application-development/appendix/licensing.html
- [S17] OBS Audio/Video Formats Guide：https://obsproject.com/kb/audio-video-formats-guide
- [S18] 本地历史体积报告：`/home/snemc/workspace/mark-shot-little/docs/binary-size-analysis.md:3`

主要复现方式（输出路径见证据目录）：

```sh
smart-search deep 'Screen recording minimal FFmpeg build hardware encoder quality tradeoffs fMP4 crash recovery GStreamer libobs Rust libraries licenses and packaging primary sources' --budget deep --format json
smart-search context7-library ffmpeg 'configure disable everything selectively enable encoders muxers hardware acceleration license static dynamic linking' --format json
smart-search context7-docs '/websites/ffmpeg_documentation' 'Building small FFmpeg disable-everything enable specific encoder muxer fragmented mp4 movflags frag_keyframe empty_moov crash' --format json
smart-search exa-search 'NVIDIA video codec SDK NVENC screen content 4:4:4 lookahead multipass quality presets memory latency' --include-domains docs.nvidia.com developer.nvidia.com --num-results 4 --include-highlights --format json
smart-search fetch 'https://ffmpeg.org/legal.html' --format markdown
smart-search fetch 'https://ffmpeg.org/ffmpeg-formats.html' --format markdown
smart-search fetch 'https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-video-encoder-api-prog-guide/index.html' --format markdown
```

Context7 搜索曾返回许多无关项目，实际仅采用官方 FFmpeg 文档 ID；`scap/src/frame.rs` 返回 404，随后定位到实际的 `src/frame/mod.rs` 与 `src/frame/video.rs`，404 页面不作证据。综合搜索服务 TLS 失败，本轮全部采用已验证可用的 Context7/Exa/抓取通道。
