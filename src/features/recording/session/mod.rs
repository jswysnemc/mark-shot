//! 【录制会话】【进程内调度】采集与编码在单个工作线程串行消费，音频使用有界队列

mod audio;
mod clock;
#[cfg(test)]
mod tests;
use super::{
    Command, Target,
    backend::{Shared, publish},
    capture,
};
use crate::config::recording::Recording;
use audio::Audio;
use clock::Clock;
use shot_media::{Encoder, Media, Settings};
use std::{
    path::Path,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

/// 【录制会话】【运行入口】按 config、target、path 录制，commands 控制暂停停止，status 交付状态和实际路径
pub(super) fn run(
    config: &Recording,
    target: &Target,
    path: &Path,
    commands: &Receiver<Command>,
    status: &Shared,
) -> Result<(), String> {
    let media = Media::load()?;
    let mut source = capture::open(
        target,
        config.cursor,
        commands,
        &media,
        config.encoder != "software",
    )?;
    let mut encoder: Option<Encoder> = None;
    let mut audio = None;
    let mut clock = None;
    let mut detail = String::new();
    let mut next = Instant::now();
    let interval = Duration::from_secs_f64(1.0 / f64::from(config.frame_rate()));
    let mut frames = 0u64;
    let mut last_pts = -1;
    let mut dimensions = None;
    let mut paused = false;
    let mut software_retry = false;
    let mut pending = false;
    let started = Instant::now();
    loop {
        let wait = if paused {
            Duration::from_millis(50)
        } else if !pending && source.asynchronous() {
            Duration::ZERO
        } else {
            next.saturating_duration_since(Instant::now())
                .min(Duration::from_millis(10))
        };
        match commands.recv_timeout(wait) {
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(Command::Pause) if !paused => {
                paused = true;
                source.pause(true)?;
                pending = false;
                if let Some(audio) = &mut audio {
                    Audio::pause(audio, true)?;
                }
                if let Some(clock) = &mut clock {
                    Clock::pause(clock, Instant::now());
                }
            }
            Ok(Command::Resume) if paused => {
                paused = false;
                if let Some(clock) = &mut clock {
                    Clock::resume(clock, Instant::now());
                }
                source.pause(false)?;
                if let Some(audio) = &mut audio {
                    Audio::pause(audio, false)?;
                }
                next = Instant::now();
            }
            _ => {}
        }
        if let (Some(audio), Some(encoder), Some(clock)) = (&mut audio, &mut encoder, &clock) {
            audio.drain(encoder, clock)?;
        }
        if paused {
            let pts = clock
                .as_ref()
                .and_then(|clock| clock.pts(Instant::now()))
                .unwrap_or(0);
            publish(status, "paused", &detail, (pts / 1000) as u64);
            continue;
        }
        if Instant::now() < next && !source.asynchronous() {
            continue;
        }
        if !pending {
            pending = source.next(Duration::from_millis(10))?;
        }
        let now = Instant::now();
        if now < next {
            continue;
        }
        let updated = pending;
        if let Some(clock) = &clock {
            let pts = clock.pts(now).unwrap_or(0);
            publish(status, "recording", &detail, (pts / 1000) as u64);
            if !updated && pts - last_pts < 1_000_000 {
                next = now + interval;
                continue;
            }
        }
        let Some(frame) = source.frame() else {
            if frames == 0 && started.elapsed() > Duration::from_secs(15) {
                return Err("录制来源未提供视频帧 / No recording frames".into());
            }
            continue;
        };
        if dimensions.is_some_and(|size| size != frame.size) {
            return Err("录制尺寸已变化 / Recording dimensions changed".into());
        }
        dimensions = Some(frame.size);
        let crop = crop(target.crop, frame.size)?;
        if encoder.is_none() {
            let settings = Settings {
                device: frame.gpu.map(|surface| surface.device().to_owned()),
                width: crop[2],
                height: crop[3],
                fps: config.frame_rate(),
                quality: match config.quality.as_str() {
                    "high" => 2,
                    "efficient" => 0,
                    _ => 1,
                },
                mode: if software_retry {
                    2
                } else if frame.gpu.is_some() {
                    1
                } else {
                    match config.encoder.as_str() {
                        "hardware" => 1,
                        "software" => 2,
                        _ => 0,
                    }
                },
                audio: config.audio != "off" && config.format != "gif",
            };
            let opened = match Encoder::open(&media, path, &settings) {
                Ok(opened) => opened,
                Err(error) if frame.gpu.is_some() => {
                    log::info!("【录制会话】【显存回退】{error}");
                    source.prefer_cpu();
                    pending = false;
                    continue;
                }
                Err(error) => return Err(error),
            };
            detail = format!("{} · {} · in-process", source.name(), opened.name());
            if settings.audio {
                audio = Some(Audio::new(
                    &media,
                    config.audio == "microphone",
                    config.frame_rate(),
                )?);
            }
            encoder = Some(opened);
            clock = Some(Clock::new(Instant::now()));
        }
        let now = Instant::now();
        let pts = clock.as_ref().unwrap().pts(now).unwrap_or(0);
        if pts <= last_pts {
            continue;
        }
        if let Err(error) = write_frame(encoder.as_mut().unwrap(), &frame, crop, pts) {
            if frames == 0 && !software_retry && config.encoder == "auto" {
                log::info!("【录制会话】【首帧回退】{error}");
                drop(encoder.take());
                audio = None;
                clock = None;
                software_retry = true;
                source.prefer_cpu();
                pending = false;
                continue;
            }
            return Err(error);
        }
        frames += 1;
        pending = false;
        last_pts = pts;
        next = (next + interval).max(now);
        publish(status, "recording", &detail, (pts / 1000) as u64);
    }
    if let (Some(frame), Some(encoder), Some(clock)) = (source.frame(), &mut encoder, &clock) {
        let pts = clock.pts(Instant::now()).unwrap_or(last_pts);
        if pts > last_pts {
            let crop = crop(target.crop, frame.size)?;
            write_frame(encoder, &frame, crop, pts)?;
            last_pts = pts;
        }
    }
    source.pause(true)?;
    if let (Some(audio), Some(encoder), Some(clock)) = (&mut audio, &mut encoder, &clock) {
        audio.pause(true)?;
        audio.drain(encoder, clock)?;
        if !audio.received() {
            return Err("录音来源没有提供样本 / Audio source provided no samples".into());
        }
    }
    drop(audio);
    drop(source);
    publish(status, "saving", &detail, (last_pts.max(0) / 1000) as u64);
    if let Some(mut encoder) = encoder {
        encoder.finish()?;
    }
    if frames == 0 {
        return Err("录制没有视频帧 / No video frames recorded".into());
    }
    Ok(())
}

/// 【录制会话】【帧交付】按 frame、crop 和 pts 同步提交 CPU 或显存内容，返回前解除采集缓冲借用
fn write_frame(
    encoder: &mut Encoder,
    frame: &capture::Frame<'_>,
    crop: [u32; 4],
    pts: i64,
) -> Result<(), String> {
    if let Some(surface) = frame.gpu {
        return encoder.dmabuf(surface, (crop[0], crop[1]), pts);
    }
    let offset = (crop[1] as usize)
        .checked_mul(frame.stride)
        .and_then(|offset| offset.checked_add(crop[0] as usize * 4))
        .ok_or("Recording frame offset overflow")?;
    let pixels = frame
        .pixels
        .get(offset..)
        .ok_or("Recording crop exceeds frame storage")?;
    encoder.video(pixels, frame.stride, pts)
}

/// 【录制会话】【裁剪验证】按 requested 和 size 返回合法偶数物理区域，拒绝越界与空区域
fn crop(requested: Option<[u32; 4]>, size: (u32, u32)) -> Result<[u32; 4], String> {
    let [x, y, width, height] = requested.unwrap_or([0, 0, size.0 & !1, size.1 & !1]);
    if width < 2
        || height < 2
        || width % 2 != 0
        || height % 2 != 0
        || x.checked_add(width).is_none_or(|end| end > size.0)
        || y.checked_add(height).is_none_or(|end| end > size.1)
    {
        return Err("录制区域无效 / Invalid recording crop".into());
    }
    Ok([x, y, width, height])
}
