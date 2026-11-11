//! 【Windows录制】【软件兼容】无可用 GPU 视频处理器时使用有界 GDI 与系统编码器，明确显示兼容路径

use super::{audio::Audio, encoder::Encoder, qpc};
use crate::{
    backend::windows::native,
    config::recording::Recording,
    features::recording::{
        Command, Target,
        backend::{Shared, publish},
    },
};
use std::{
    path::Path,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};
use windows::{Win32::Media::MediaFoundation::*, core::Result as WinResult};

/// 【Windows录制】【CPU样本】按 bytes、pts、duration 创建独立媒体样本，返回有界内存缓冲
pub(super) fn sample(bytes: &[u8], pts: i64, duration: i64) -> WinResult<IMFSample> {
    unsafe {
        let sample = MFCreateSample()?;
        let buffer = MFCreateMemoryBuffer(bytes.len() as u32)?;
        let mut destination = std::ptr::null_mut();
        buffer.Lock(&mut destination, None, None)?;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len());
        buffer.Unlock()?;
        buffer.SetCurrentLength(bytes.len() as u32)?;
        sample.AddBuffer(&buffer)?;
        sample.SetSampleTime(pts)?;
        sample.SetSampleDuration(duration)?;
        Ok(sample)
    }
}

/// 【Windows录制】【兼容执行】按配置与区域直接采集 BGRA，控制和音频沿用统一时钟，返回实际录制错误
pub(super) fn run(
    config: &Recording,
    target: &Target,
    path: &Path,
    commands: &Receiver<Command>,
    status: &Shared,
    reason: &str,
) -> std::result::Result<(), String> {
    let monitors = native::monitors().map_err(|error| error.to_string())?;
    let monitor = if target.display.is_empty() {
        monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or(monitors.first())
    } else {
        monitors
            .iter()
            .find(|monitor| monitor.name == target.display)
    }
    .ok_or("录制显示器已移除 / Display removed")?;
    let [x, y, width, height] =
        target
            .crop
            .unwrap_or([0, 0, monitor.bounds.width & !1, monitor.bounds.height & !1]);
    if width < 2
        || height < 2
        || width % 2 != 0
        || height % 2 != 0
        || x.checked_add(width)
            .is_none_or(|end| end > monitor.bounds.width)
        || y.checked_add(height)
            .is_none_or(|end| end > monitor.bounds.height)
    {
        return Err("录制区域无效 / Invalid recording region".into());
    }
    let bounds = native::Bounds {
        left: monitor.bounds.left + x as i32,
        top: monitor.bounds.top + y as i32,
        width,
        height,
    };
    let encoder = Encoder::new(None, path, (width, height), config, false)
        .map_err(|error| format!("Software H.264: {error}"))?;
    let mut audio = if config.audio != "off" && config.format != "gif" {
        Some(Audio::new(config.audio == "microphone").map_err(|error| format!("WASAPI: {error}"))?)
    } else {
        None
    };
    let detail = format!("GDI · Software H.264\n{reason}");
    let epoch = qpc()?;
    let mut paused_at = None;
    let mut pause_offset = 0;
    let mut last_video = 0;
    let interval = Duration::from_secs_f64(1.0 / f64::from(config.frame_rate()));
    let duration = 10_000_000 / i64::from(config.frame_rate());
    let mut next = Instant::now();
    loop {
        let now = Instant::now();
        match commands.recv_timeout(
            next.saturating_duration_since(now)
                .min(Duration::from_millis(10)),
        ) {
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(Command::Pause) if paused_at.is_none() => paused_at = Some(qpc()?),
            Ok(Command::Resume) if paused_at.is_some() => {
                pause_offset += qpc()? - paused_at.take().unwrap();
                next = Instant::now();
            }
            _ => {}
        }
        if let Some(audio) = &mut audio {
            audio
                .drain(epoch, pause_offset, paused_at.is_none(), |sample| {
                    encoder.audio(sample)
                })
                .map_err(|error| format!("Audio: {error}"))?;
        }
        if paused_at.is_some() {
            next = Instant::now() + Duration::from_millis(50);
            publish(status, "paused", &detail, last_video);
            continue;
        }
        if Instant::now() < next {
            continue;
        }
        let pixels =
            native::capture(bounds, config.cursor).map_err(|error| format!("GDI: {error}"))?;
        let pts = qpc()? - epoch - pause_offset;
        let sample = sample(&pixels, pts, duration).map_err(|error| error.to_string())?;
        encoder
            .video(&sample)
            .map_err(|error| format!("Software encode: {error}"))?;
        last_video = (pts / 10000) as u64;
        publish(status, "recording", &detail, last_video);
        next = (next + interval).max(Instant::now());
    }
    drop(audio);
    publish(status, "saving", &detail, last_video);
    encoder
        .finish()
        .map_err(|error| format!("MP4 finalize: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【录制集成测试】【系统编码文件】无参数，实际写入运动色块和正弦音轨，产物交给 CI 解码验证
    #[test]
    #[ignore = "requires Windows Media Foundation runtime"]
    fn media_foundation_encodes_video_and_audio() {
        let directory =
            std::env::var_os("MARK_SHOT_RECORDING_EVIDENCE").expect("private evidence directory");
        std::fs::create_dir_all(&directory).unwrap();
        let path = Path::new(&directory).join("native-encoding.mp4");
        let _runtime = super::super::device::Runtime::new().unwrap();
        let config = Recording {
            audio: "system".into(),
            fps: 30,
            ..Default::default()
        };
        let encoder = Encoder::new(None, &path, (320, 240), &config, false).unwrap();
        for frame in 0..60 {
            let mut pixels = vec![0u8; 320 * 240 * 4];
            for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                pixel.copy_from_slice(&[
                    if index % 320 < 160 { 255 } else { 0 },
                    (frame * 4) as u8,
                    if index % 320 >= 160 { 255 } else { 0 },
                    255,
                ]);
            }
            encoder
                .video(&sample(&pixels, frame * 10_000_000 / 30, 10_000_000 / 30).unwrap())
                .unwrap();
            let mut pcm = Vec::with_capacity(1600 * 8);
            for index in 0..1600 {
                let value =
                    ((frame * 1600 + index) as f32 * std::f32::consts::TAU * 440.0 / 48000.0).sin()
                        * 0.15;
                pcm.extend_from_slice(&value.to_le_bytes());
                pcm.extend_from_slice(&value.to_le_bytes());
            }
            encoder
                .audio(&sample(&pcm, frame * 10_000_000 / 30, 10_000_000 / 30).unwrap())
                .unwrap();
        }
        encoder.finish().unwrap();
        assert!(std::fs::metadata(path).unwrap().len() > 1000);
    }
}
