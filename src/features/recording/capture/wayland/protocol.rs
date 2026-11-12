//! 【Wayland录制】【协议对象】分离持久会话与短生命周期帧对象

use super::State;
use crate::types::Output;
use wayland_client::{
    QueueHandle,
    globals::GlobalList,
    protocol::{wl_buffer, wl_output},
};
use wayland_protocols::ext::{
    image_capture_source::v1::client::{
        ext_image_capture_source_v1::ExtImageCaptureSourceV1,
        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
    },
    image_copy_capture::v1::client::{
        ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        ext_image_copy_capture_manager_v1::{ExtImageCopyCaptureManagerV1, Options},
        ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    },
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

pub(super) enum Protocol {
    Image {
        source: ExtImageCaptureSourceV1,
        session: ExtImageCopyCaptureSessionV1,
        frame: Option<ExtImageCopyCaptureFrameV1>,
    },
    Screen {
        manager: ZwlrScreencopyManagerV1,
        output: wl_output::WlOutput,
        cursor: bool,
        frame: Option<ZwlrScreencopyFrameV1>,
        copying: bool,
    },
}

impl Protocol {
    /// 【Wayland录制】【能力绑定】按 globals、qh、output 和 cursor 优先创建标准捕获会话，返回协议错误
    pub fn new(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
        output: &Output,
        cursor: bool,
    ) -> Result<Self, String> {
        let output = output
            .wl_output
            .as_ref()
            .ok_or("Wayland output unavailable")?;
        if let (Ok(source_manager), Ok(manager)) = (
            globals.bind::<ExtOutputImageCaptureSourceManagerV1, _, _>(qh, 1..=1, ()),
            globals.bind::<ExtImageCopyCaptureManagerV1, _, _>(qh, 1..=1, ()),
        ) {
            let source = source_manager.create_source(output, qh, ());
            let session = manager.create_session(
                &source,
                if cursor {
                    Options::PaintCursors
                } else {
                    Options::empty()
                },
                qh,
                (),
            );
            source_manager.destroy();
            manager.destroy();
            return Ok(Self::Image {
                source,
                session,
                frame: None,
            });
        }
        Ok(Self::Screen {
            manager: globals.bind(qh, 1..=3, ()).map_err(|e| e.to_string())?,
            output: output.clone(),
            cursor,
            frame: None,
            copying: false,
        })
    }
    /// 【Wayland录制】【帧请求】按 state 清除逐帧约束，qh 路由新帧事件，无返回值
    pub fn request(&mut self, state: &mut State, qh: &QueueHandle<State>) {
        if let Self::Screen {
            manager,
            output,
            cursor,
            frame,
            copying,
        } = self
        {
            state.constraints = false;
            state.formats.clear();
            state.dma_formats.clear();
            *copying = false;
            *frame = Some(manager.capture_output(i32::from(*cursor), output, qh, ()));
        }
    }
    /// 【Wayland录制】【提交缓冲】按 buffer、size 和 qh 提交一次采集，不创建无界请求
    pub fn copy(
        &mut self,
        buffer: &wl_buffer::WlBuffer,
        size: (u32, u32),
        qh: &QueueHandle<State>,
    ) {
        match self {
            Self::Image { session, frame, .. } => {
                let next = session.create_frame(qh, ());
                next.attach_buffer(buffer);
                next.damage_buffer(0, 0, size.0 as i32, size.1 as i32);
                next.capture();
                *frame = Some(next);
            }
            Self::Screen { frame, copying, .. } => {
                frame.as_ref().unwrap().copy(buffer);
                *copying = true;
            }
        }
    }
    /// 【Wayland录制】【请求状态】无参数，返回当前是否已经提交共享缓冲
    pub fn copying(&self) -> bool {
        match self {
            Self::Image { frame, .. } => frame.is_some(),
            Self::Screen { copying, .. } => *copying,
        }
    }
    /// 【Wayland录制】【帧释放】无参数，销毁完成或取消的短期帧对象，持久捕获会话继续存在
    pub fn complete(&mut self) {
        match self {
            Self::Image { frame, .. } => {
                if let Some(frame) = frame.take() {
                    frame.destroy();
                }
            }
            Self::Screen { frame, copying, .. } => {
                if let Some(frame) = frame.take() {
                    frame.destroy();
                }
                *copying = false;
            }
        }
    }
    /// 【Wayland录制】【路径名称】无参数，返回协商成功的协议和内存路径
    pub fn name(&self) -> &'static str {
        match self {
            Self::Image { .. } => "Wayland image-copy SHM",
            Self::Screen { .. } => "Wayland screencopy SHM",
        }
    }
}

impl Drop for Protocol {
    /// 【Wayland录制】【会话释放】无参数，依次销毁帧、捕获会话和来源，不保留后台连接
    fn drop(&mut self) {
        self.complete();
        match self {
            Self::Image {
                source, session, ..
            } => {
                session.destroy();
                source.destroy();
            }
            Self::Screen { manager, .. } => manager.destroy(),
        }
    }
}
