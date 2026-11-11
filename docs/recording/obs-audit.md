# OBS Studio 实际源码核查：Mark Shot 应借鉴的录制链路

日期：2026-11-11。已读取真实下载的上游源码，不是仅依赖README或泛泛介绍。源码目录 `/tmp/mark-shot-recording-sources/obs-studio`；远端 `https://github.com/obsproject/obs-studio.git`；固定提交 `6e22cced38142703d5fe3bbb80f3f97ab50d4a08`（提交日期2026-11-10）。读取范围：libobs、plugins、frontend、cmake、libobs-winrt、libobs-d3d11、libobs-opengl。没有编译或运行OBS，没有录制用户桌面，没有修改Mark Shot代码。

## 结论

OBS 低CPU录制的关键不是“应用大所以必然重”，而是保留GPU纹理、GPU合成/转换、直接向支持纹理的硬件编码器交付，避免完整视频帧持续读回CPU。它仍有GPU拷贝、编码器表面池和每帧合成；不能称为端到端零复制。OBS的通用场景、多源混合、预览、音频混音、插件、直播及回放缓冲范围比单屏录制器广，因此包体积/默认开销不能直接与wf-recorder或wl-screenrec做倍数比较。

Mark Shot 应借鉴原生纹理及同步契约、能力探测后回退、有界在途帧、时钟和资源生命周期；不建议为补齐录屏直接嵌入完整libobs。后者会扩大依赖和功能边界，且源码为GPLv2-or-later，需单独处理许可证结合问题。

## 1. Linux PipeWire：真的导入DMA-BUF，且处理modifier与显式同步

源码事实：

- `init_format_info_sync`查询图形后端可导入的DRM格式与modifier，把交集用于PipeWire协商；不是仅看到DMA-BUF标志就当作可直接使用。[格式与modifier能力查询](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/linux-pipewire/pipewire.c#L442)
- `find_latest_buffer`排空已经到达的多个buffer，只保留最新一个，并及时归还旧buffer。这是一种低延迟丢旧帧策略，不能把采集队列长度转成无上限事件堆积。[最新buffer选择](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/linux-pipewire/pipewire.c#L583)
- 同步视频路径读取每plane的fd、offset、stride和modifier，使用`gs_texture_create_from_dmabuf`导入GPU纹理；导入失败移除该modifier并触发重新协商。[DMA-BUF导入与重协商](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/linux-pipewire/pipewire.c#L711)
- PipeWire 1.2编译能力下，读取SPA_META_SyncTimeline及syncobj fd；绘制前等待acquire点，绘制后导出release点，旧引用未绘制时也补发释放信号。[同步元数据](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/linux-pipewire/pipewire.c#L777)、[绘制时等待及释放](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/linux-pipewire/pipewire.c#L1324)
- 绘制支持crop、旋转/翻转和独立cursor纹理；非DMA-BUF仍保留内存纹理路径。GPU路径并非所有桌面和驱动必然命中。

对Mark Shot的含义：录制frame必须携带内存类型、plane、modifier、设备和同步责任；只传一个fd或把fd mmap成Vec不能等价替代。Portal会话管理可以复用，现有按需截图RGBA接口不能承接这些资源契约。

重要边界：同文件1341行仍有“按raw colorimetry选择纹理色彩空间”的FIXME。这证明成熟项目也有具体路径限制；不能把此实现直接说成完整HDR/色彩管理模板。[明确未完成的colorimetry项](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/linux-pipewire/pipewire.c#L1341)

## 2. Windows：WGC/DXGI捕获留在GPU，仍做GPU复制

- WGC回调从frame.Surface获得ID3D11Texture2D；根据需要创建OBS texture。客户区使用CopySubresourceRegion裁剪，完整目标使用CopyResource。尺寸变化Recreate帧池，关闭目标设置inactive；该代码段没有Map到CPU。[WGC帧处理](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs-winrt/winrt-capture.cpp#L128)
- WGC当前源码使用两帧pool；原始纹理复制到自有纹理后保留，避免异步下游依赖已归还的采集buffer。[WGC池与会话](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs-winrt/winrt-capture.cpp#L310)
- DXGI duplicator使用AcquireNextFrame(0)，获得纹理并CopyResource到自有纹理，处理ACCESS_LOST与WAIT_TIMEOUT。[DXGI帧复制与获取](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs-d3d11/d3d11-duplicator.cpp#L233)
- 显示器Auto后端并非永远优先WGC：通常DXGI，某显示器不在当前adapter或移动设备多GPU等情况转WGC。这表明后端选择需要硬件拓扑和实测，不宜机械指定单个API为所有设备最优。[后端选择](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/win-capture/duplicator-monitor-capture.c#L250)

对Mark Shot的差异：Qt版本是WGC→STAGING→Map→memcpy→CPU BGRA；OBS这部分保持GPU纹理。应借鉴数据路径而非仅移植WGC名称。

## 3. 合成、颜色转换与硬编码入口

`render_video`先绘制主场景，再按输出尺寸处理，GPU conversion开启时绘制转换纹理；GPU encoder活跃则交给GPU编码队列，只有raw_active才stage输出纹理，`output_frame`也只有raw_active时调用download_frame。[主渲染入口](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video.c#L542)、[CPU下载分支](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video.c#L871)

是否走GPU由encoder声明PASS_TEXTURE且core可生成NV12/P010纹理决定，不是“编码器名字里含硬件”即可。[GPU编码资格](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-encoder.c#L219)

### VAAPI路径

1. av_hwframe_get_buffer取得VAAPI编码surface。
2. vaExportSurfaceHandle导出DRM_PRIME_2 planes。
3. gs_texture_create_from_dmabuf把这些surface planes导入图形上下文。
4. `vaapi_encode_tex`用gs_copy_texture把core的已转换纹理复制到编码surface，再提交编码。[VAAPI surface导出](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/obs-ffmpeg/obs-ffmpeg-vaapi.c#L401)、[纹理编码](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/obs-ffmpeg/obs-ffmpeg-vaapi.c#L762)

创建时真实尝试surface导入，不可用则回退CPU输入编码器；CPU路径仍有av_hwframe_transfer_data。不能说OBS VAAPI永远无CPU回读。[可运行性探测和回退](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/obs-ffmpeg/obs-ffmpeg-vaapi.c#L548)

### NVENC D3D11路径

取得共享texture并AcquireSync，CopyResource到已注册的NVENC纹理，释放输入共享纹理的keyed mutex，再nvEncMapInputResource和编码。这里的“map”是NVENC注册资源映射，不是D3D CPU Map；输入共享纹理与编码器自有纹理的生命周期不同。[NVENC D3D11实际提交](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/obs-nvenc/nvenc-d3d11.c#L210)

结论：可争取“常态不读回完整CPU视频帧”，不应承诺“采集到文件完全零复制”。为缓冲归还、格式转换、异步编码安全保留GPU copy往往合理。

## 4. 有界纹理、调度和慢编码处理

- core定义NUM_TEXTURES=2、NUM_ENCODE_TEXTURES=10、等待1帧；分配NV12/P010共享纹理并进入avail queue。[常量](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-internal.h#L49)、[初始化](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video-gpu-encode.c#L237)
- queue_frame无可用texture时增加待编码帧count，重用已有图像；正常情况下直接交换texture，raw_active或初次重复时才GPU copy。[纹理队列策略](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video.c#L445)
- 独立GPU编码线程以信号量等待，提交encode_texture2；编码完成后归还texture，重复次数仍有剩余则放回队首。[编码线程](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video-gpu-encode.c#L24)
- video_sleep用单调时钟按帧间隔调度，迟到时计算时间跨过的帧数并计lagged_frames，而不是把每个采集callback都当成一个目标FPS帧。[时间格调度](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video.c#L808)

限制：这些事实证明的是core纹理资源数量和调度策略，不证明所有encoder内部surface、packet队列、audio缓存、驱动资源都严格同一上限。不能复制“10”到Mark Shot就宣称低内存；要按每个后端的实际in-flight需求和内存预算测量。

## 5. 音频时钟比Qt旧录制更加明确

WASAPI读取packet同时取得device position与timestamp；ProcessOutput使用ts×100纳秒；其他来源可用设备时间或os_gettime_ns减去本块时长。[WASAPI时间戳](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/win-wasapi/win-wasapi.cpp#L1026)

libobs `source_output_audio_data`判断时间戳是否已在系统时域，设置timing_adjust；以expected next timestamp平滑小抖动、处理大跳变，再对齐音频buffer位置。audio mix在缓存达到上限且来源迟到时丢弃过时样本或重启该source timing；GPU video编码开始前可等待paired audio首样本。[输入音频时钟与跳变](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-source.c#L1621)、[迟到音频处理](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-audio.c#L140)、[编码起点配对](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video-gpu-encode.c#L100)

应借鉴：保存设备时间戳、明确定义时钟映射、暂停规则、缺样本/跳变恢复和可观察指标；不要只按AAC累计样本计数就声称长期同步已解决。本核查不证明OBS所有设备无漂移，也未测它的长时误差。

## 6. OBS额外开销与公平比较

源码可确认每帧有scene/view render，可能多个mix；额外display/preview逐个render；编码器缩放可创建另一个匹配尺寸/色彩格式的mix。完整插件集合包含browser/websocket、采集卡、摄像头、滤镜、转场、音频接口、流媒体等。[场景合成](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video.c#L172)、[额外display](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-video.c#L94)、[GPU缩放mix](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs-encoder.c#L230)、[插件构建清单](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/CMakeLists.txt#L45)

推断：单输出单屏录制器不需要始终保留直播场景编辑和高频预览，可减少这部分工作及默认依赖；但OBS能选择性构建核心模块和关闭preview，也并非所有插件在所有会话都占满资源。没有公平实测，不得给“轻量工具比OBS快N倍/少N%”结论。

与wf-recorder的关键区别还包括帧调度。对照本地固定提交`c5de47440e8e81c92befb696cb69819cdb3bfe8a`，其`use_damage`默认开启，使用`copy_with_damage`等待画面更新；帮助文本明确默认输出为可变帧率，而`--no-damage`才持续请求画面。`-r`则向FFmpeg过滤链加入`fps`转换。这与OBS以固定帧间隔进行场景输出不同：静态桌面下，两个默认配置处理的帧数量就可能不同。较少采集和编码工作可以解释部分低占用，但不能据此推导活跃画面或相同帧率条件下的性能倍数。[wf-recorder默认damage开关](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L266)、[请求变化帧](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L313)、[帧率选项说明](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/main.cpp#L915)、[固定帧率过滤器](https://github.com/ammen99/wf-recorder/blob/c5de47440e8e81c92befb696cb69819cdb3bfe8a/src/frame-writer.cpp#L266)

公平基准至少固定：同采集API、同GPU/adapter、相同区域/分辨率/FPS、H.264 profile/preset/质量或等质量文件大小、相同音轨、cursor、HDR/SDR、预览开关；记录CPU总核秒、GPU各engine busy、RSS/PSS与GPU内存、power、render/encode dropped、文件码率及解码质量。静态文档、滚动文本、视频、3D游戏分别比较。默认配置截图不能支撑架构优劣结论。

## 7. 进程边界与打包启示

OBS FFmpeg mux路径确实创建外部pipe，但传的是已编码packet及时间元数据，不是持续BGRA原始整帧。因此“使用子进程必然高CPU”是错误概括；边界放在哪个阶段决定搬运量。[创建mux子进程](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/obs-ffmpeg/obs-ffmpeg-mux.c#L310)、[写入encoded packet](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/obs-ffmpeg/obs-ffmpeg-mux.c#L626)

对Mark Shot可选：录制worker自采集/自编码，控制消息通过IPC；或只让mux helper接收编码packet。不要把每帧CPU RGBA穿过JSON、PNG或stdin rawvideo后称为GPU直连。

libobs与本次读取多数文件头为GPLv2或后续版本；当前Mark Shot Rust许可证MIT OR Apache-2.0。学习算法与接口设计、独立实现平台API，与直接复制/链接GPL源码是不同选择。本文不提供具体法律裁定，但建议默认不嵌完整libobs，保持源码来源与依赖许可核查。[libobs许可头](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/libobs/obs.h#L1)

构建方面，PipeWire插件依赖PipeWire、Gio、libdrm及图形层；frontend还引入FFmpeg/CURL及界面组件，browser/websocket为子模块。源码下载只做针对性sparse checkout，未拉取所有子模块，未进行完整编译，不能报告OBS完整可分发大小。[PipeWire插件依赖](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/plugins/linux-pipewire/CMakeLists.txt#L8)、[frontend依赖](https://github.com/obsproject/obs-studio/blob/6e22cced38142703d5fe3bbb80f3f97ab50d4a08/frontend/CMakeLists.txt#L12)

## 8. 证据与未确认事项

已经通过实际源码确认的数据路径：PipeWire modifier协商/DMABUF导入/显式同步、WGC和DXGI GPU复制、core GPU转换与CPU下载条件、NVENC/VAAPI纹理编码、纹理队列与重复帧、音频时间戳与调整、mux子进程packet边界、许可证。

没有进行：实际OBS录制、同机wf-recorder/wl-screenrec性能对比、编译包大小、所有编码器全部回退、所有平台长时音画同步、强制结束后的文件恢复验证。上述未知不能用README或架构推断补成“已验证”。
