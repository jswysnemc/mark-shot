//! 【Windows录制】【GPU循环】WGC 采集、NV12 转换与时间轴推进，首帧提交失败交由入口决定回退

use super::super::{Shared, publish};
use super::{audio, capture, converter, device, encoder, qpc, software};
use crate::{
    config::recording::Recording,
    features::recording::{Command, Target},
};
use std::{
    path::Path,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

/// 【Windows录制】【执行入口】按 config、target、path 建立 GPU 管线，commands 控制录制，status 记录状态；retry_first_frame 返回首帧提交是否失败
pub(super) fn run(
    config: &Recording,
    target: &Target,
    path: &Path,
    commands: &Receiver<Command>,
    status: &Shared,
    retry_first_frame: &mut bool,
) -> Result<(), String> {
    let prepared = (|| -> Result<_, String> {
        let device = device::Device::new().map_err(|error| format!("D3D11: {error}"))?;
        let capture = capture::Capture::new(&device, target, config.cursor)?;
        let crop = target
            .crop
            .unwrap_or([0, 0, capture.size.0 & !1, capture.size.1 & !1]);
        let [x, y, width, height] = crop;
        if width < 2
            || height < 2
            || width % 2 != 0
            || height % 2 != 0
            || x.checked_add(width).is_none_or(|end| end > capture.size.0)
            || y.checked_add(height).is_none_or(|end| end > capture.size.1)
        {
            return Err("录制区域无效 / Invalid recording region".into());
        }
        let converter = converter::Converter::new(&device, capture.size, crop, config.frame_rate())
            .map_err(|error| format!("GPU NV12: {error}"))?;
        let encoder = match encoder::Encoder::new(
            Some(&device),
            path,
            (width, height),
            config,
            config.encoder != "software",
        ) {
            Ok(encoder) => encoder,
            Err(error) if config.encoder == "auto" => {
                log::info!("【Windows录制】【软件回退】{error}");
                encoder::Encoder::new(Some(&device), path, (width, height), config, false)
                    .map_err(|fallback| format!("Hardware: {error}; software: {fallback}"))?
            }
            Err(error) => return Err(format!("H.264: {error}")),
        };
        if config.encoder == "hardware" && !encoder.hardware {
            return Err("系统未确认硬件编码器，请使用自动模式 / No verified hardware encoder; use automatic mode".into());
        }
        Ok((device, capture, converter, encoder))
    })();
    let (_device, mut capture, converter, encoder) = match prepared {
        Ok(pipeline) => pipeline,
        Err(error) if config.encoder == "auto" => {
            log::info!("【Windows录制】【兼容回退】{error}");
            return software::run(config, target, path, commands, status, &error);
        }
        Err(error) => return Err(error),
    };
    let mut audio = if config.audio != "off" && config.format != "gif" {
        Some(
            audio::Audio::new(config.audio == "microphone")
                .map_err(|error| format!("WASAPI: {error}"))?,
        )
    } else {
        None
    };
    let detail = if encoder.hardware {
        "WGC · GPU NV12 · H.264 hardware"
    } else {
        "WGC · GPU NV12 · H.264 system encoder (hardware unverified)"
    };
    let mut epoch = 0;
    let mut last_video = -1;
    let mut frames = 0u64;
    let mut paused_at = None;
    let mut pause_offset = 0i64;
    let mut next_video = Instant::now();
    let start_timeout = Instant::now();
    let frame_duration = 10_000_000 / i64::from(config.frame_rate());
    let interval = Duration::from_secs_f64(1.0 / f64::from(config.frame_rate()));
    loop {
        let wait = if paused_at.is_some() {
            Duration::from_millis(50)
        } else {
            next_video
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(10))
        };
        match commands.recv_timeout(wait) {
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(Command::Pause) if paused_at.is_none() => paused_at = Some(qpc()?),
            Ok(Command::Resume) if paused_at.is_some() => {
                pause_offset += qpc()? - paused_at.take().unwrap();
                next_video = Instant::now();
            }
            _ => {}
        }
        let texture = capture.texture()?;
        if let Some(audio) = &mut audio {
            audio
                .drain(epoch, pause_offset, paused_at.is_none(), |sample| {
                    encoder.audio(sample)
                })
                .map_err(|error| format!("Audio: {error}"))?;
        }
        let now = Instant::now();
        if epoch == 0 && now.duration_since(start_timeout) > Duration::from_secs(15) {
            return Err("WGC 未交付视频帧 / Capture frame timeout".into());
        }
        if paused_at.is_some() {
            publish(status, "paused", detail, (last_video.max(0) / 10000) as u64);
            continue;
        }
        if now < next_video {
            continue;
        }
        let Some((texture, timestamp)) = texture else {
            next_video = Instant::now() + Duration::from_millis(10);
            continue;
        };
        if epoch == 0 {
            epoch = timestamp;
        }
        let pts = (qpc()? - epoch - pause_offset).max(0);
        if pts <= last_video {
            continue;
        }
        match converter.convert(&texture, pts, frame_duration) {
            Ok(sample) => {
                if let Err(error) = encoder.video(&sample) {
                    *retry_first_frame = frames == 0;
                    let crop =
                        target
                            .crop
                            .unwrap_or([0, 0, capture.size.0 & !1, capture.size.1 & !1]);
                    let bytes = unsafe { sample.GetTotalLength() }.ok();
                    return Err(format!(
                        "Video: {error}; frames={frames}, crop={crop:?}, pts={pts}, duration={frame_duration}, bytes={bytes:?}, pipeline={detail}"
                    ));
                }
                frames += 1;
                last_video = pts;
            }
            Err(error)
                if error.code()
                    == windows::Win32::Media::MediaFoundation::MF_E_SAMPLEALLOCATOR_EMPTY => {}
            Err(error) => return Err(format!("GPU conversion: {error}")),
        }
        next_video = (next_video + interval).max(now);
        publish(
            status,
            "recording",
            detail,
            (last_video.max(0) / 10000) as u64,
        );
    }
    drop(audio);
    drop(capture);
    publish(status, "saving", detail, (last_video.max(0) / 10000) as u64);
    encoder
        .finish()
        .map_err(|error| format!("MP4 finalize: {error}"))?;
    if frames == 0 {
        return Err("录制没有生成视频帧 / No video frames recorded".into());
    }
    Ok(())
}
