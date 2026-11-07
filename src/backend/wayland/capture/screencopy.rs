// ── Wayland wlr-screencopy backend ─────────────────────────────────────────
//
// for compsitors that doesn't use ext-image-copy-capture (like niri),
// to avoid using portal
use async_trait::async_trait;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::{wl_buffer, wl_output};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use super::shm_copy::{self, Protocol, State};
use crate::backend::CaptureMethod;
use crate::types::{CaptureResult, Output};

pub struct ScreencopyMethod {
    conn: Connection,
    include_cursor: bool,
}

impl ScreencopyMethod {
    pub fn new(conn: Connection) -> Self {
        Self::with_cursor(conn, false)
    }

    /// 【Wayland】【屏幕捕获】创建后端；conn 为连接，include_cursor 控制截图是否包含指针
    pub fn with_cursor(conn: Connection, include_cursor: bool) -> Self {
        Self {
            conn,
            include_cursor,
        }
    }
}

pub fn supported(conn: &Connection) -> bool {
    shm_copy::supported(conn, &["zwlr_screencopy_manager_v1"])
}

#[async_trait]
impl CaptureMethod for ScreencopyMethod {
    /// 【持续捕获】【原生屏幕会话】按 outputs 创建固定输出的无授权请求后端，返回会话
    async fn open_session(
        &self,
        outputs: &[Output],
    ) -> crate::backend::capture_session::Result<
        Box<dyn crate::backend::capture_session::CaptureSession>,
    > {
        super::isolation::open(outputs, |connection| {
            Box::new(Self::with_cursor(connection, self.include_cursor))
        })
        .await
    }
    async fn capture_frame(
        &self,
        outputs: &[Output],
    ) -> Result<CaptureResult, Box<dyn std::error::Error>> {
        shm_copy::capture::<Screencopy>(&self.conn, outputs, self.include_cursor).await
    }
}

struct Screencopy {
    manager: ZwlrScreencopyManagerV1,
    frames: Vec<ZwlrScreencopyFrameV1>,
}

impl Protocol for Screencopy {
    fn bind(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, String> {
        Ok(Self {
            manager: globals
                .bind(qh, 1..=3, ())
                .map_err(|e| format!("zwlr_screencopy_manager_v1: {e}"))?,
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
        self.frames.push(
            self.manager
                .capture_output(i32::from(include_cursor), output, qh, idx),
        );
    }

    fn copy(
        &mut self,
        idx: usize,
        buffer: &wl_buffer::WlBuffer,
        _: (u32, u32),
        _: &QueueHandle<State>,
    ) {
        self.frames[idx].copy(buffer);
    }

    fn destroy(self) {
        for frame in self.frames {
            frame.destroy();
        }
        self.manager.destroy();
    }
}

wayland_client::delegate_noop!(State: ZwlrScreencopyManagerV1);

impl Dispatch<ZwlrScreencopyFrameV1, usize> for State {
    fn event(
        state: &mut Self,
        frame: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        &idx: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use zwlr_screencopy_frame_v1::{Event, Flags};
        let slot = &mut state.slots[idx];
        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                slot.size = Some((width, height));
                slot.stride = stride;
                if let WEnum::Value(format) = format {
                    slot.formats.push(format);
                }
                if frame.version() < 3 {
                    slot.constraints_done = true;
                }
            }
            Event::BufferDone => slot.constraints_done = true,
            Event::Flags {
                flags: WEnum::Value(flags),
            } => slot.y_invert = flags.contains(Flags::YInvert),
            Event::Ready { .. } => slot.result = Some(Ok(())),
            Event::Failed => {
                slot.result = Some(Err("capture failed".into()));
            }
            _ => {}
        }
    }
}
