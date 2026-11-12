//! 【Wayland录制】【事件处理】将协议事件更新为有界会话状态，过期帧不会交付像素

use super::State;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
    globals::GlobalListContents,
    protocol::{wl_registry, wl_shm},
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1 as image, ext_image_copy_capture_session_v1 as session,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1 as screen;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    /// 【Wayland录制】【注册表】按事件更新全局列表，不创建重复绑定，无返回值
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<session::ExtImageCopyCaptureSessionV1, ()> for State {
    /// 【Wayland录制】【会话约束】按 event 保存尺寸、格式和停止状态，无返回值
    fn event(
        state: &mut Self,
        _: &session::ExtImageCopyCaptureSessionV1,
        event: session::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.constraint(event);
    }
}

impl State {
    /// 【Wayland录制】【约束批次】按 event 合并任意顺序的格式和尺寸，新批次首次约束清空旧格式，Done 后允许分配
    fn constraint(&mut self, event: session::Event) {
        let state = self;
        if matches!(
            event,
            session::Event::BufferSize { .. }
                | session::Event::ShmFormat { .. }
                | session::Event::DmabufDevice { .. }
                | session::Event::DmabufFormat { .. }
        ) && state.constraints
        {
            state.constraints = false;
            state.dma_formats.clear();
            state.formats.clear();
        }
        match event {
            session::Event::BufferSize { width, height } => {
                state.size = Some((width, height));
                state.stride = width.saturating_mul(4);
            }
            session::Event::ShmFormat {
                format: WEnum::Value(format),
            } => state.formats.push(format),
            session::Event::Done => state.constraints = true,
            session::Event::DmabufDevice { device } => {
                state.device = super::dmabuf::device(&device)
            }
            session::Event::DmabufFormat { format, modifiers } => {
                let (values, remaining) = modifiers.as_chunks::<8>();
                if remaining.is_empty() && values.len() <= 256 && state.dma_formats.len() < 64 {
                    state.dma_formats.push((
                        format,
                        values
                            .iter()
                            .map(|bytes| u64::from_ne_bytes(*bytes))
                            .collect(),
                    ));
                }
            }
            session::Event::Stopped => {
                state.failure = Some("Wayland capture source stopped".into())
            }
            _ => {}
        }
    }
}

impl Dispatch<image::ExtImageCopyCaptureFrameV1, ()> for State {
    /// 【Wayland录制】【标准帧完成】按 event 保存方向和结果，读取共享缓冲必须等待 Ready
    fn event(
        state: &mut Self,
        _: &image::ExtImageCopyCaptureFrameV1,
        event: image::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            image::Event::Transform {
                transform: WEnum::Value(transform),
            } => state.transform = transform,
            image::Event::Ready => state.ready = true,
            image::Event::Failed { reason } => {
                state.failure = Some(format!("Wayland image-copy failed: {reason:?}"))
            }
            _ => {}
        }
    }
}

impl Dispatch<screen::ZwlrScreencopyFrameV1, ()> for State {
    /// 【Wayland录制】【兼容帧完成】按 event 保存行距、颜色格式与结果，旧协议首个 Buffer 即完成约束
    fn event(
        state: &mut Self,
        frame: &screen::ZwlrScreencopyFrameV1,
        event: screen::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            screen::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                state.size = Some((width, height));
                state.stride = stride;
                if let WEnum::Value(format) = format {
                    state.formats.push(format);
                }
                if frame.version() < 3 {
                    state.constraints = true;
                }
            }
            screen::Event::BufferDone => state.constraints = true,
            screen::Event::LinuxDmabuf { format, .. } => state.dma_formats.push((format, vec![0])),
            screen::Event::Flags {
                flags: WEnum::Value(flags),
            } => state.inverted = flags.contains(screen::Flags::YInvert),
            screen::Event::Ready { .. } => state.ready = true,
            screen::Event::Failed => state.failure = Some("Wayland screencopy failed".into()),
            _ => {}
        }
    }
}

wayland_client::delegate_noop!(State: ignore wl_shm::WlShm);
wayland_client::delegate_noop!(State: ignore wayland_client::protocol::wl_shm_pool::WlShmPool);
wayland_client::delegate_noop!(State: ignore wayland_client::protocol::wl_buffer::WlBuffer);
wayland_client::delegate_noop!(State: ignore wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1);
wayland_client::delegate_noop!(State: ignore wayland_protocols::ext::image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1);
wayland_client::delegate_noop!(State: ignore wayland_protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1);
wayland_client::delegate_noop!(State: ignore wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1);

#[cfg(test)]
mod tests {
    use super::*;
    /// 【Wayland录制回归】【任意事件顺序】无参数，尺寸晚于格式不会清空当前批次，下一批替换旧格式
    #[test]
    fn format_before_size_survives_constraint_batches() {
        let mut state = State::default();
        state.constraint(session::Event::ShmFormat {
            format: WEnum::Value(wl_shm::Format::Xrgb8888),
        });
        state.constraint(session::Event::DmabufFormat {
            format: u32::from_le_bytes(*b"XR24"),
            modifiers: 0u64.to_ne_bytes().to_vec(),
        });
        state.constraint(session::Event::BufferSize {
            width: 1280,
            height: 720,
        });
        state.constraint(session::Event::Done);
        assert!(state.constraints);
        assert_eq!(state.formats, [wl_shm::Format::Xrgb8888]);
        assert_eq!(state.dma_formats.len(), 1);
        state.constraint(session::Event::ShmFormat {
            format: WEnum::Value(wl_shm::Format::Xbgr8888),
        });
        assert!(!state.constraints);
        state.constraint(session::Event::BufferSize {
            width: 640,
            height: 480,
        });
        state.constraint(session::Event::Done);
        assert_eq!(state.formats, [wl_shm::Format::Xbgr8888]);
        assert!(state.dma_formats.is_empty());
    }
}
