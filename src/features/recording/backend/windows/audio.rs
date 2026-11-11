//! 【Windows录制】【声音采集】WASAPI 默认系统声或麦克风，设备 QPC 时间映射到视频时间轴

use windows::{
    Win32::{
        Media::{Audio::*, MediaFoundation::*},
        System::Com::*,
    },
    core::Result,
};

pub(super) struct Audio {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    last_end: i64,
}

impl Audio {
    /// 【Windows录制】【音频打开】按 microphone 选择默认输入或 loopback，统一输出 48kHz 双声道浮点 PCM
    pub fn new(microphone: bool) -> Result<Self> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator
                .GetDefaultAudioEndpoint(if microphone { eCapture } else { eRender }, eConsole)?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let format = WAVEFORMATEX {
                wFormatTag: 3,
                nChannels: 2,
                nSamplesPerSec: 48000,
                nAvgBytesPerSec: 384000,
                nBlockAlign: 8,
                wBitsPerSample: 32,
                cbSize: 0,
            };
            let flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY
                | if microphone {
                    0
                } else {
                    AUDCLNT_STREAMFLAGS_LOOPBACK
                };
            client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 1_000_000, 0, &format, None)?;
            let capture = client.GetService()?;
            client.Start()?;
            Ok(Self {
                client,
                capture,
                last_end: 0,
            })
        }
    }

    /// 【Windows录制】【音频排空】按 epoch、pause_offset 和 enabled 读取音频；submit 接收有时间戳样本，暂停时丢弃缓存
    pub fn drain(
        &mut self,
        epoch: i64,
        pause_offset: i64,
        enabled: bool,
        mut submit: impl FnMut(&IMFSample) -> Result<()>,
    ) -> Result<()> {
        unsafe {
            // 1. 【Windows录制】【有界读取】每次最多处理128块，保持控制命令及时响应
            for _ in 0..128 {
                if self.capture.GetNextPacketSize()? == 0 {
                    break;
                }
                let mut data = std::ptr::null_mut();
                let mut frames = 0;
                let mut flags = 0;
                let mut qpc = 0;
                self.capture
                    .GetBuffer(&mut data, &mut frames, &mut flags, None, Some(&mut qpc))?;
                let result = (|| -> Result<()> {
                    if !enabled || epoch == 0 {
                        return Ok(());
                    }
                    let duration = i64::from(frames) * 10_000_000 / 48000;
                    let pts = qpc as i64 - epoch - pause_offset;
                    if pts < 0 {
                        return Ok(());
                    }
                    let sample = MFCreateSample()?;
                    let bytes = frames * 8;
                    let buffer = MFCreateMemoryBuffer(bytes)?;
                    let mut destination = std::ptr::null_mut();
                    buffer.Lock(&mut destination, None, None)?;
                    if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
                        std::ptr::write_bytes(destination, 0, bytes as usize);
                    } else {
                        std::ptr::copy_nonoverlapping(data, destination, bytes as usize);
                    }
                    buffer.Unlock()?;
                    buffer.SetCurrentLength(bytes)?;
                    sample.AddBuffer(&buffer)?;
                    sample.SetSampleTime(pts.max(self.last_end))?;
                    sample.SetSampleDuration(duration)?;
                    self.last_end = pts.max(self.last_end) + duration;
                    submit(&sample)
                })();
                self.capture.ReleaseBuffer(frames)?;
                result?;
            }
            Ok(())
        }
    }
}
impl Drop for Audio {
    /// 【Windows录制】【音频停止】无参数，停止设备流并释放 COM 资源
    fn drop(&mut self) {
        unsafe {
            let _ = self.client.Stop();
        }
    }
}
