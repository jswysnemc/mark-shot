// ── Wayland ext-image-copy-capture backend ─────────────────────────────────
//
// Bypasses xdg-desktop-portal entirely, uses standard Wayland which is faster
// and doesn't ask for monitor choice as Portal does
use async_trait::async_trait;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::{wl_buffer, wl_output};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::{ExtImageCopyCaptureManagerV1, Options},
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use super::shm_copy::{self, Protocol, State};
use crate::backend::CaptureMethod;
use crate::types::{CaptureResult, Output};

pub struct ImageCopyMethod {
    conn: Connection,
    include_cursor: bool,
}

impl ImageCopyMethod {
    pub fn new(conn: Connection) -> Self {
        Self::with_cursor(conn, false)
    }

    /// 【Wayland】【图像捕获】创建后端；conn 为连接，include_cursor 控制截图是否包含指针
    pub fn with_cursor(conn: Connection, include_cursor: bool) -> Self {
        Self {
            conn,
            include_cursor,
        }
    }
}

pub fn supported(conn: &Connection) -> bool {
    shm_copy::supported(
        conn,
        &[
            "ext_output_image_capture_source_manager_v1",
            "ext_image_copy_capture_manager_v1",
        ],
    )
}

#[async_trait]
impl CaptureMethod for ImageCopyMethod {
    async fn capture_frame(
        &self,
        outputs: &[Output],
    ) -> Result<CaptureResult, Box<dyn std::error::Error>> {
        shm_copy::capture::<ImageCopy>(&self.conn, outputs, self.include_cursor).await
    }
}

struct ImageCopy {
    sources_mgr: ExtOutputImageCaptureSourceManagerV1,
    copy_mgr: ExtImageCopyCaptureManagerV1,
    sessions: Vec<(ExtImageCaptureSourceV1, ExtImageCopyCaptureSessionV1)>,
    frames: Vec<ExtImageCopyCaptureFrameV1>,
}

impl Protocol for ImageCopy {
    fn bind(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, String> {
        Ok(Self {
            sources_mgr: globals
                .bind(qh, 1..=1, ())
                .map_err(|e| format!("ext_output_image_capture_source_manager_v1: {e}"))?,
            copy_mgr: globals
                .bind(qh, 1..=1, ())
                .map_err(|e| format!("ext_image_copy_capture_manager_v1: {e}"))?,
            sessions: Vec::new(),
            frames: Vec::new(),
        })
    }

    fn request(
        &mut self,
        output: &wl_output::WlOutput,
        idx: usize,
        qh: &QueueHandle<State>,
        include_cursor: bool,
    ) {
        let source = self.sources_mgr.create_source(output, qh, ());
        let session = self.copy_mgr.create_session(
            &source,
            if include_cursor {
                Options::PaintCursors
            } else {
                Options::empty()
            },
            qh,
            idx,
        );
        self.sessions.push((source, session));
    }

    fn copy(
        &mut self,
        idx: usize,
        buffer: &wl_buffer::WlBuffer,
        (w, h): (u32, u32),
        qh: &QueueHandle<State>,
    ) {
        let frame = self.sessions[idx].1.create_frame(qh, idx);
        frame.attach_buffer(buffer);
        frame.damage_buffer(0, 0, w as i32, h as i32);
        frame.capture();
        self.frames.push(frame);
    }

    fn destroy(self) {
        for frame in self.frames {
            frame.destroy();
        }
        for (source, session) in self.sessions {
            session.destroy();
            source.destroy();
        }
        self.copy_mgr.destroy();
        self.sources_mgr.destroy();
    }
}

wayland_client::delegate_noop!(State: ExtOutputImageCaptureSourceManagerV1);
wayland_client::delegate_noop!(State: ExtImageCaptureSourceV1);
wayland_client::delegate_noop!(State: ExtImageCopyCaptureManagerV1);

impl Dispatch<ExtImageCopyCaptureSessionV1, usize> for State {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        &idx: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_session_v1::Event;
        let slot = &mut state.slots[idx];
        match event {
            Event::BufferSize { width, height } => {
                slot.size = Some((width, height));
                slot.stride = width * 4;
            }
            Event::ShmFormat {
                format: WEnum::Value(format),
            } => slot.formats.push(format),
            Event::Done => slot.constraints_done = true,
            Event::Stopped => {
                slot.result
                    .get_or_insert(Err("capture session stopped".into()));
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, usize> for State {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        &idx: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_frame_v1::Event;
        let slot = &mut state.slots[idx];
        match event {
            Event::Transform {
                transform: WEnum::Value(t),
            } => slot.transform = Some(t),
            Event::Ready => slot.result = Some(Ok(())),
            Event::Failed { reason } => {
                slot.result = Some(Err(format!("capture failed: {reason:?}")))
            }
            _ => {}
        }
    }
}
