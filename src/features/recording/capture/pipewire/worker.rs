//! 【录制流】【原生循环】持续接收视频和音频，回调只处理有界缓冲，状态通过条件变量交付

use super::{Command, Input, Shared};
use pipewire::{self as pw, properties::properties, spa};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

struct Format {
    video: spa::param::video::VideoInfoRaw,
}

/// 【录制流】【像素读取】按 data 和协商 format 复制可见帧到 shared，失败保留错误，不读取 DMA-BUF 为线性内存
fn video(
    data: &mut pw::spa::buffer::Data,
    format: &spa::param::video::VideoInfoRaw,
    shared: &Shared,
) -> Result<(), String> {
    use spa::param::video::VideoFormat;
    let (width, height) = (format.size().width, format.size().height);
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 33554432 {
        return Err("PipeWire frame exceeds budget".into());
    }
    let bgr = match format.format() {
        VideoFormat::BGRx | VideoFormat::BGRA => true,
        VideoFormat::RGBx | VideoFormat::RGBA => false,
        _ => return Err("Unsupported PipeWire video format".into()),
    };
    let offset = data.chunk().offset() as i64;
    let stride = i64::from(data.chunk().stride());
    if stride.unsigned_abs() < u64::from(width) * 4 {
        return Err("Invalid PipeWire stride".into());
    }
    let span = u64::from(height - 1) * stride.unsigned_abs() + u64::from(width) * 4;
    if span > u64::from(data.chunk().size()) {
        return Err("PipeWire video chunk is shorter than visible rows".into());
    }
    let bytes = data.data().ok_or("PipeWire shared buffer is unavailable")?;
    let mut target = shared.0.lock().unwrap();
    if target.size != (0, 0) && target.size != (width, height) {
        return Err("PipeWire source dimensions changed".into());
    }
    target
        .pixels
        .resize(width as usize * height as usize * 4, 255);
    for row in 0..height as usize {
        let start = usize::try_from(offset + row as i64 * stride)
            .map_err(|_| "Invalid negative PipeWire row")?;
        let line = bytes
            .get(start..start + width as usize * 4)
            .ok_or("PipeWire row exceeds mapping")?;
        let output = &mut target.pixels[row * width as usize * 4..(row + 1) * width as usize * 4];
        output.copy_from_slice(line);
        for pixel in output.as_chunks_mut::<4>().0 {
            if bgr {
                pixel.swap(0, 2);
            }
            pixel[3] = 255;
        }
    }
    target.size = (width, height);
    target.ready = true;
    shared.1.notify_one();
    Ok(())
}

/// 【录制流】【音频格式】无参数，返回固定 48kHz 双声道浮点格式的序列化协商数据
fn audio_format() -> Vec<u8> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(48000);
    info.set_channels(2);
    let object = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    };
    let mut bytes = Vec::new();
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(&mut bytes),
        &spa::pod::Value::Object(object),
    )
    .unwrap();
    bytes
}

/// 【录制流】【持续执行】按 input、fps、commands 与 shared 运行原生连接，返回协议失败
pub(super) fn run(
    input: Input,
    _fps: u32,
    commands: pw::channel::Receiver<Command>,
    shared: Shared,
) -> Result<(), String> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None).map_err(|e| e.to_string())?;
    let context = pw::context::ContextRc::new(&mainloop, None).map_err(|e| e.to_string())?;
    let is_video = matches!(input, Input::Video { .. });
    let (core, node, properties) = match input {
        Input::Video { node, remote } => (
            context
                .connect_fd_rc(remote, None)
                .map_err(|e| e.to_string())?,
            Some(node),
            properties! { *pw::keys::MEDIA_TYPE => "Video", *pw::keys::MEDIA_CATEGORY => "Capture", *pw::keys::MEDIA_ROLE => "Screen" },
        ),
        Input::Audio { microphone } => {
            let properties = properties! { *pw::keys::MEDIA_TYPE => "Audio", *pw::keys::MEDIA_CATEGORY => "Capture", *pw::keys::MEDIA_ROLE => "Production",
            "stream.capture.sink" => if microphone { "false" } else { "true" }, "node.latency" => "480/48000" };
            (
                context.connect_rc(None).map_err(|e| e.to_string())?,
                None,
                properties,
            )
        }
    };
    let stream = pw::stream::StreamRc::new(core.clone(), "Mark Shot recording", properties)
        .map_err(|e| e.to_string())?;
    let paused = Rc::new(Cell::new(false));
    let _errors = core
        .add_listener_local()
        .error({
            let shared = shared.clone();
            let mainloop = mainloop.clone();
            move |_, _, _, message| {
                shared.0.lock().unwrap().failure = Some(message.into());
                shared.1.notify_all();
                mainloop.quit();
            }
        })
        .register();
    let _listener = stream
        .add_local_listener_with_user_data(Format {
            video: Default::default(),
        })
        .state_changed({
            let shared = shared.clone();
            let mainloop = mainloop.clone();
            move |_, _, _, new| {
                if let pw::stream::StreamState::Error(error) = new {
                    shared.0.lock().unwrap().failure = Some(error);
                    shared.1.notify_all();
                    mainloop.quit();
                }
            }
        })
        .param_changed({
            let shared = shared.clone();
            move |stream, format, id, parameter| {
                if is_video
                    && id == spa::param::ParamType::Format.as_raw()
                    && let Some(parameter) = parameter
                {
                    let result = (|| -> Result<(), String> {
                        format.video.parse(parameter).map_err(|e| e.to_string())?;
                        let bytes = super::buffers::shared((
                            format.video.size().width,
                            format.video.size().height,
                        ))?;
                        stream
                            .update_params(&mut [spa::pod::Pod::from_bytes(&bytes)
                                .ok_or("Invalid PipeWire buffer parameters")?])
                            .map_err(|e| e.to_string())
                    })();
                    if let Err(error) = result {
                        shared.0.lock().unwrap().failure = Some(error);
                        shared.1.notify_all();
                    }
                }
            }
        })
        .process({
            let shared = shared.clone();
            let paused = paused.clone();
            move |stream, format| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                if paused.get() {
                    return;
                }
                let Some(data) = buffer.datas_mut().first_mut() else {
                    return;
                };
                if data.chunk().size() == 0 {
                    return;
                }
                let result = if is_video {
                    video(data, &format.video, &shared)
                } else {
                    let offset = data.chunk().offset() as usize;
                    let length = data.chunk().size() as usize;
                    (|| -> Result<(), String> {
                        if length > 48000 * 8 || !length.is_multiple_of(8) {
                            return Err("Invalid PipeWire audio size".into());
                        }
                        let bytes = data
                            .data()
                            .and_then(|bytes| bytes.get(offset..offset + length))
                            .ok_or("Invalid PipeWire audio mapping")?;
                        let samples: Vec<_> = bytes
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .map(|sample| f32::from_le_bytes(*sample))
                            .collect();
                        let mut target = shared.0.lock().unwrap();
                        if target
                            .audio
                            .iter()
                            .map(|(_, block)| block.len())
                            .sum::<usize>()
                            + samples.len()
                            > 48000
                        {
                            return Err("Audio encoder cannot keep up with capture".into());
                        }
                        target.last_audio = Some(Instant::now());
                        target.audio_received = true;
                        target
                            .audio
                            .push_back((audio_timestamp(stream, samples.len() / 2)?, samples));
                        Ok(())
                    })()
                };
                if let Err(error) = result {
                    shared.0.lock().unwrap().failure = Some(error);
                    shared.1.notify_all();
                }
            }
        })
        .register()
        .map_err(|e| e.to_string())?;
    let _control = commands.attach(mainloop.loop_(), {
        let stream = stream.clone();
        let mainloop = mainloop.clone();
        let shared = shared.clone();
        move |command| match command {
            Command::Stop => mainloop.quit(),
            Command::Pause(value, acknowledge) => {
                paused.set(value);
                while let Some(buffer) = stream.dequeue_buffer() {
                    drop(buffer);
                }
                if let Err(error) = stream.set_active(!value) {
                    shared.0.lock().unwrap().failure = Some(error.to_string());
                    shared.1.notify_all();
                }
                let mut data = shared.0.lock().unwrap();
                data.ready = false;
                data.audio.clear();
                data.last_audio = Some(Instant::now());
                let _ = acknowledge.send(());
            }
        }
    });
    let mut bytes = Vec::new();
    let pod = if is_video {
        crate::backend::wayland::capture::stream::format::build_format_pod(&mut bytes)
    } else {
        bytes = audio_format();
        spa::pod::Pod::from_bytes(&bytes).unwrap()
    };
    stream
        .connect(
            spa::utils::Direction::Input,
            node,
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut [pod],
        )
        .map_err(|e| e.to_string())?;
    mainloop.run();
    Ok(())
}

/// 【录制流】【设备时间】按 stream 时钟和 frames 计算首个样本的单调时刻，扣除图延迟与内部队列，不使用回调到达时间计数
fn audio_timestamp(stream: &pw::stream::Stream, frames: usize) -> Result<Instant, String> {
    let mut report = std::mem::MaybeUninit::<pw::sys::pw_time>::zeroed();
    let result = unsafe {
        pw::sys::pw_stream_get_time_n(
            stream.as_raw_ptr(),
            report.as_mut_ptr(),
            std::mem::size_of::<pw::sys::pw_time>(),
        )
    };
    if result < 0 {
        return Err("PipeWire audio clock unavailable".into());
    }
    let report = unsafe { report.assume_init() };
    if report.rate.denom == 0 || report.now <= 0 {
        return Err("Invalid PipeWire audio clock".into());
    }
    let now = Instant::now();
    let monotonic = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    let elapsed = (i128::from(monotonic.tv_sec) * 1_000_000_000 + i128::from(monotonic.tv_nsec)
        - i128::from(report.now))
    .max(0);
    let graph = i128::from(report.delay.max(0)) * i128::from(report.rate.num) * 1_000_000_000
        / i128::from(report.rate.denom);
    let queued = (i128::from(report.queued) + i128::from(report.buffered) + frames as i128)
        * 1_000_000_000
        / 48000;
    let delay =
        u64::try_from(elapsed + graph + queued).map_err(|_| "PipeWire audio clock overflow")?;
    if delay > 10_000_000_000 {
        return Err("PipeWire audio delay exceeds budget".into());
    }
    now.checked_sub(Duration::from_nanos(delay))
        .ok_or_else(|| "PipeWire audio timestamp underflow".into())
}
