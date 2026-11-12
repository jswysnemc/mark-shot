# Windows 视频提交失败分析

日期：2026-11-11。报告版本基于 `43534bd`，错误为 `Video: 参数错误。 (0x80070057)`。

## 已确定的范围

基线 `backend/windows/mod.rs` 在 GPU 录制循环调用 `Encoder::video` 时添加 `Video:` 前缀；
当前该循环已拆到 `backend/windows/gpu.rs`。
该方法直接调用 `IMFSinkWriter::WriteSample`。因此错误发生前，WGC、D3D11 视频转换器、
H.264 输入类型协商和 `BeginWriting` 已成功，至少有一次 NV12 转换返回了媒体样本。
现有消息没有帧计数、尺寸、实际编码转换器名称或样本属性，无法判断是否为首帧。

微软的 `WriteSample` 文档没有把 `E_INVALIDARG` 对应到唯一原因，不能仅凭错误码断言
某个纹理标志、buffer 长度、驱动或编码尺寸就是根因。

## 基线中确认的处理缺口

- 自动模式在设备和编码管线初始化失败时回退 GDI；进入循环后的 `WriteSample` 错误直接退出，没有运行期回退
- `MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS` 只用于允许硬件转换器，程序可能在硬件属性未确认的情况下提交 GPU NV12 样本
- 视频写入失败会提前返回，跳过循环末尾的 `Finalize`；控制器仍保留 `.Recording_*.recording/capture.mp4`，但目录存在不代表 MP4 可恢复
- Windows CI 的 `media_foundation_encodes_video_and_audio` 使用 `Encoder::new(None, ...)` 和 CPU BGRA 样本，没有覆盖 WGC、GPU NV12 与 DXGI 样本写入

## 已完成的处理修复

- 将平台运行时、回退调度与 GPU 循环拆开。自动模式的首个 `WriteSample` 失败时，
  等 GPU 调用返回、资源释放后，使用 GDI + CPU BGRA 重新启动软件录制，只回退一次
- 硬件强制模式不回退；已有成功视频帧时不重新打开同一路径，避免覆盖前段录制
- 编码器提前析构时尝试 `Finalize`，错误记录到日志；正常收尾已经尝试过时不重复调用
- 错误信息补充成功帧数、裁剪尺寸、PTS、样本时长、样本总长度和管线信息
- 添加正常 CPU 样本后的提前退出测试，以及真实 D3D11 视频设备上的 NV12 样本提交测试。
  Windows CI 新增提前退出文件的完整解码验证；GPU 测试需要实际设备，默认忽略

本地 Windows GNU 全测试目标交叉 Clippy 已通过。当前环境无法运行 Windows Media
Foundation 或用户显卡驱动；上述处理修复尚未通过 Windows 实机验收，不能据此认定
`0x80070057` 的底层原因已解决。异常 `Finalize` 也可能失败，恢复目录不保证可播放。

在 Windows 测试环境中，可先设置 `MARK_SHOT_RECORDING_EVIDENCE` 为独立产物目录，再运行：

```powershell
cargo test --lib --locked media_foundation_finalizes_on_early_exit -- --ignored --nocapture
cargo test --lib --locked media_foundation_encodes_gpu_nv12_samples -- --ignored --nocapture
```

GPU 样本测试覆盖转换器、分配器与编码器，不包含真实 WGC 桌面采集，也不模拟故障驱动。
首帧回退和故障后收尾仍需用原始失败场景验收。

## 差异验证

在设置的“屏幕录制”中选择“软件编码”，录制一个 1280×720、30 帧、关闭音频的短片。
软件模式走 GDI + CPU BGRA + 系统 H.264，可与当前 GPU 路径比较。成功只能将问题范围
缩小到两条路径的差异，仍不足以证明是哪项 GPU 属性不兼容。

继续复现需要：Windows 版本、显卡与驱动版本、实际录制分辨率和帧率、自动或硬件模式、
是否启动即失败，以及软件模式结果。当前 Linux 环境不能执行对应 Windows 显卡路径。
保留原恢复目录；先检查其中 `capture.mp4` 的大小与可播放性，不对原文件直接覆盖或修复。

## 官方依据

通过 `smart-search fetch <URL> --format markdown` 核对以下正文：

- [IMFSinkWriter::WriteSample](https://learn.microsoft.com/en-us/windows/win32/api/mfreadwrite/nf-mfreadwrite-imfsinkwriter-writesample)
- [IMFVideoSampleAllocatorEx::InitializeSampleAllocatorEx](https://learn.microsoft.com/en-us/windows/win32/api/mfidl/nf-mfidl-imfvideosampleallocatorex-initializesampleallocatorex)
