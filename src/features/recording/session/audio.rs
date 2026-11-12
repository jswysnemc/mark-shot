//! 【录制音频】【来源兼容】优先 PipeWire，连接不可用时切换原生 PulseAudio，禁止音频空轨静默成功

use super::clock::Clock;
use crate::features::recording::capture::pipewire::{Input, Stream};
use shot_media::{Encoder, Media, Pulse};
use std::time::{Duration, Instant};

enum Source {
    PipeWire(Stream),
    Pulse(Pulse),
}
pub(super) struct Audio {
    source: Source,
    media: Media,
    microphone: bool,
    received: bool,
    last: Instant,
}
impl Audio {
    /// 【录制音频】【来源创建】按 media、microphone 与 fps 建立首选流，返回异步采集状态
    pub fn new(media: &Media, microphone: bool, fps: u32) -> Result<Self, String> {
        Ok(Self {
            source: Source::PipeWire(Stream::new(Input::Audio { microphone }, fps)?),
            media: media.clone(),
            microphone,
            received: false,
            last: Instant::now(),
        })
    }
    /// 【录制音频】【控制转发】按 paused 切换当前来源，恢复后重置样本超时，无返回样本
    pub fn pause(&mut self, paused: bool) -> Result<(), String> {
        self.last = Instant::now();
        match &mut self.source {
            Source::PipeWire(stream) => stream.pause(paused),
            Source::Pulse(stream) => stream.pause(paused),
        }
    }
    /// 【录制音频】【有效样本】无参数，返回是否确实收到过采集样本，静音样本也是有效输入
    pub fn received(&self) -> bool {
        self.received
    }
    /// 【录制音频】【有界消费】按 encoder 与 clock 消费最多半秒音频，首次连接失败允许兼容回退，中途失败明确报告
    pub fn drain(&mut self, encoder: &mut Encoder, clock: &Clock) -> Result<(), String> {
        let result = match &mut self.source {
            Source::PipeWire(stream) => {
                let mut data = stream.data.0.lock().unwrap();
                if let Some(error) = &data.failure {
                    Err(error.clone())
                } else {
                    for (at, samples) in std::mem::take(&mut data.audio) {
                        if submit(encoder, clock, at, &samples)? {
                            self.received = true;
                            self.last = Instant::now();
                        }
                    }
                    Ok(())
                }
            }
            Source::Pulse(stream) => {
                for _ in 0..50 {
                    let Some((at, samples)) = stream.read()? else {
                        break;
                    };
                    if submit(encoder, clock, at, samples)? {
                        self.received = true;
                        self.last = Instant::now();
                    }
                }
                Ok(())
            }
        };
        let timeout = !clock.paused() && self.last.elapsed() > Duration::from_secs(3);
        if (result.is_err() || timeout)
            && !self.received
            && matches!(self.source, Source::PipeWire(_))
        {
            log::info!("【录制音频】【原生兼容】PipeWire 未提供样本，尝试 PulseAudio");
            self.source = Source::Pulse(Pulse::new(&self.media, self.microphone)?);
            self.last = Instant::now();
            return Ok(());
        }
        result?;
        if timeout {
            return Err("录音来源长时间无响应 / Audio source stopped delivering samples".into());
        }
        Ok(())
    }
}

/// 【录制音频】【边界裁剪】按 clock 去除 at、samples 中启动或恢复以前的部分，仅成功交付实际样本才返回真
fn submit(
    encoder: &mut Encoder,
    clock: &Clock,
    at: Instant,
    samples: &[f32],
) -> Result<bool, String> {
    if clock.paused() {
        return Ok(false);
    }
    let skip = clock
        .active_since()
        .checked_duration_since(at)
        .map_or(0, |elapsed| {
            (elapsed.as_nanos() * 48000).div_ceil(1_000_000_000)
        });
    if skip >= samples.len() as u128 / 2 {
        return Ok(false);
    }
    let position = at + Duration::from_nanos((skip * 1_000_000_000 / 48000) as u64);
    let Some(pts) = clock.pts(position) else {
        return Ok(false);
    };
    encoder.audio(&samples[skip as usize * 2..], pts)?;
    Ok(true)
}
