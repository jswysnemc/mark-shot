use crate::utils::to_rgba;
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::param::video::VideoFormat;
use spa::pod::Pod;
use std::os::fd::{BorrowedFd, OwnedFd};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::time::Duration;

struct UserData {
    format: spa::param::video::VideoInfoRaw,
}

pub struct PipewireFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

const FRAME_TIMEOUT: Duration = Duration::from_secs(5);

type FrameResult = Result<PipewireFrame, String>;

pub fn capture_frame(
    node_id: u32,
    fd: BorrowedFd<'_>,
) -> Result<PipewireFrame, Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::sync_channel::<FrameResult>(1);

    let owned_fd: OwnedFd = fd
        .try_clone_to_owned()
        .map_err(|e| format!("Failed to clone file descriptor: {}", e))?;

    std::thread::spawn(move || {
        if let Err(e) = run_stream(node_id, owned_fd, tx.clone()) {
            let _ = tx.try_send(Err(e));
        }
    });

    match rx.recv_timeout(FRAME_TIMEOUT) {
        Ok(frame) => frame.map_err(Into::into),
        Err(RecvTimeoutError::Timeout) => Err("PipeWire didn't deliver a frame in time".into()),
        Err(RecvTimeoutError::Disconnected) => {
            Err("PipeWire capture thread exited without a frame".into())
        }
    }
}

fn run_stream(node_id: u32, fd: OwnedFd, tx: SyncSender<FrameResult>) -> Result<(), String> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|e| format!("Failed to create main loop: {e}"))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| format!("Failed to create context: {e}"))?;

    let core = context
        .connect_fd_rc(fd, None)
        .map_err(|e| format!("Failed to connect via FD: {e}"))?;

    let data = UserData {
        format: Default::default(),
    };

    let stream = pw::stream::StreamRc::new(
        core.clone(),
        "mark-shot",
        properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )
    .map_err(|e| format!("Failed to create stream: {e}"))?;

    let mainloop_clone = mainloop.clone();
    let tx_clone = tx.clone();
    let mainloop_state = mainloop.clone();

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .state_changed(move |_, _, _, new| {
            if let pw::stream::StreamState::Error(msg) = new {
                let _ = tx.try_send(Err(format!("PipeWire stream error: {msg}")));
                mainloop_state.quit();
            }
        })
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else { return };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }

            let (media_type, media_subtype) = match spa::param::format_utils::parse_format(param) {
                Ok(v) => v,
                Err(_) => return,
            };

            if media_type != spa::param::format::MediaType::Video
                || media_subtype != spa::param::format::MediaSubtype::Raw
            {
                return;
            }

            let _ = user_data.format.parse(param);
        })
        .process(move |stream, user_data| {
            if let Some(mut buffer) = stream.dequeue_buffer() {
                let datas = buffer.datas_mut();
                if let Some(data) = datas.first_mut() {
                    let (offset, size, chunk_stride) = {
                        let chunk = data.chunk();
                        (
                            chunk.offset() as usize,
                            chunk.size() as usize,
                            chunk.stride(),
                        )
                    };
                    if size == 0 {
                        return;
                    }

                    let Some(bytes) = data.data() else {
                        let _ = tx_clone.try_send(Err(
                            "PipeWire sent a buffer that can't be mapped (DMA-BUF?)".to_string(),
                        ));
                        mainloop_clone.quit();
                        return;
                    };
                    let width = user_data.format.size().width;
                    let height = user_data.format.size().height;
                    if width == 0 || height == 0 {
                        return;
                    }
                    let Some(pixels) = bytes.get(offset..offset + size) else {
                        return;
                    };
                    let bgr = match user_data.format.format() {
                        VideoFormat::BGRA | VideoFormat::BGRx => true,
                        VideoFormat::RGBA | VideoFormat::RGBx => false,
                        other => {
                            let _ = tx_clone.try_send(Err(format!(
                                "PipeWire picked unsupported format {other:?}"
                            )));
                            mainloop_clone.quit();
                            return;
                        }
                    };

                    let default_stride = width.saturating_mul(4);
                    let stride = match chunk_stride {
                        s if s > 0 => s as u32,
                        _ => default_stride,
                    };

                    let mut pixels = pixels.to_vec();
                    to_rgba(&mut pixels, bgr, false);

                    let frame_data = PipewireFrame {
                        pixels,
                        width,
                        height,
                        stride,
                    };
                    let _ = tx_clone.try_send(Ok(frame_data));
                    mainloop_clone.quit();
                }
            }
        })
        .register()
        .map_err(|e| format!("Failed to register stream listener: {e}"))?;

    let mut params_buf = Vec::new();
    let pod = build_format_pod(&mut params_buf);

    stream
        .connect(
            spa::utils::Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut [pod],
        )
        .map_err(|e| format!("Failed to connect stream: {e}"))?;

    mainloop.run();
    Ok(())
}

fn build_format_pod(buffer: &mut Vec<u8>) -> &Pod {
    use spa::pod::serialize::PodSerializer;
    use spa::pod::{ChoiceValue, Object, Property, PropertyFlags, Value};
    use spa::sys::*;
    use spa::utils::{Choice, ChoiceEnum, ChoiceFlags, Id};

    PodSerializer::serialize(
        std::io::Cursor::new(&mut *buffer),
        &Value::Object(Object {
            type_: SPA_TYPE_OBJECT_Format,
            id: SPA_PARAM_EnumFormat,
            properties: vec![
                Property {
                    key: SPA_FORMAT_mediaType,
                    flags: PropertyFlags::empty(),
                    value: Value::Id(Id(SPA_MEDIA_TYPE_video)),
                },
                Property {
                    key: SPA_FORMAT_mediaSubtype,
                    flags: PropertyFlags::empty(),
                    value: Value::Id(Id(SPA_MEDIA_SUBTYPE_raw)),
                },
                Property {
                    // previously it supported only BGRA
                    // added some of the others since cosmic didn't return BGRA
                    key: SPA_FORMAT_VIDEO_format,
                    flags: PropertyFlags::empty(),
                    value: Value::Choice(ChoiceValue::Id(Choice(
                        ChoiceFlags::empty(),
                        ChoiceEnum::Enum {
                            default: Id(VideoFormat::BGRx.as_raw()),
                            alternatives: [
                                VideoFormat::BGRx,
                                VideoFormat::BGRA,
                                VideoFormat::RGBx,
                                VideoFormat::RGBA,
                            ]
                            .iter()
                            .map(|f| Id(f.as_raw()))
                            .collect(),
                        },
                    ))),
                },
            ],
        }),
    )
    .unwrap();

    Pod::from_bytes(buffer).unwrap()
}
