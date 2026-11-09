//! 【X11截图】【原生捕获】读取服务器物理像素，支持活动窗口和持续捕获

use super::{connection::Display, pixels::Format};
use crate::{
    backend::{CaptureMethod, capture_session},
    types::{Capture, CaptureResult, MonitorFrame, Output, StreamInfo},
};
use async_trait::async_trait;
use x11rb::protocol::{xfixes::ConnectionExt as _, xproto::*};

#[derive(Clone)]
pub struct Method {
    pub display: Display,
    pub include_cursor: bool,
}

impl Method {
    /// 【X11截图】【矩形读取】按position和size捕获根窗口可见像素，返回RGBA并按需合成鼠标
    fn rectangle(&self, position: (i32, i32), size: (i32, i32)) -> crate::AppResult<Vec<u8>> {
        let (width, height) = (u16::try_from(size.0)?, u16::try_from(size.1)?);
        if width == 0 || height == 0 {
            return Err("empty X11 capture rectangle".into());
        }
        let screen = self.display.screen();
        let cursor = if self.include_cursor {
            self.display
                .connection
                .xfixes_query_version(5, 0)?
                .reply()?;
            Some(self.display.connection.xfixes_get_cursor_image()?.reply()?)
        } else {
            None
        };
        let reply = self
            .display
            .connection
            .get_image(
                ImageFormat::Z_PIXMAP,
                screen.root,
                position.0.try_into()?,
                position.1.try_into()?,
                width,
                height,
                u32::MAX,
            )?
            .reply()?;
        let mut pixels = Format::new(&self.display, reply.depth, reply.visual)?.decode(
            &reply.data,
            width.into(),
            height.into(),
        )?;
        if let Some(cursor) = cursor {
            for row in 0..i32::from(cursor.height) {
                for column in 0..i32::from(cursor.width) {
                    let target_x =
                        i32::from(cursor.x) - i32::from(cursor.xhot) + column - position.0;
                    let target_y = i32::from(cursor.y) - i32::from(cursor.yhot) + row - position.1;
                    if target_x < 0 || target_y < 0 || target_x >= size.0 || target_y >= size.1 {
                        continue;
                    }
                    let argb =
                        cursor.cursor_image[(row * i32::from(cursor.width) + column) as usize];
                    let alpha = argb >> 24;
                    let offset = ((target_y * size.0 + target_x) * 4) as usize;
                    for channel in 0..3 {
                        pixels[offset + channel] = (((argb >> (16 - channel * 8)) & 255)
                            + u32::from(pixels[offset + channel]) * (255 - alpha) / 255)
                            .min(255) as u8;
                    }
                }
            }
        }
        Ok(pixels)
    }
}

#[async_trait]
impl CaptureMethod for Method {
    /// 【X11截图】【显示器捕获】按outputs顺序返回物理像素帧，保留各输出位置及尺寸
    async fn capture_frame(&self, outputs: &[Output]) -> crate::AppResult<CaptureResult> {
        log::info!(
            "【X11截图】【显示器捕获】输出数量={}，包含光标={}",
            outputs.len(),
            self.include_cursor
        );
        let mut frames = Vec::new();
        for (output, monitor) in outputs.iter().enumerate() {
            let size = monitor.info.logical_size.ok_or("X11 output has no size")?;
            let position = monitor
                .info
                .logical_position
                .unwrap_or(monitor.info.location);
            frames.push(MonitorFrame {
                output,
                pixels: self.rectangle(position, size)?,
                pw_width: size.0 as u32,
                pw_height: size.1 as u32,
                pw_stride: size.0 as u32 * 4,
                info: StreamInfo {
                    node_id: 0,
                    size: Some(size),
                    position: Some(position),
                },
            });
        }
        Ok(CaptureResult { frames })
    }

    /// 【X11截图】【持续会话】按outputs固定捕获目标，返回复用连接的会话，不申请门户权限
    async fn open_session(
        &self,
        outputs: &[Output],
    ) -> capture_session::Result<Box<dyn capture_session::CaptureSession>> {
        Ok(capture_session::repeated(Box::new(self.clone()), outputs))
    }

    /// 【X11截图】【活动窗口】无参数，读取前台客户区的可见像素，返回截图或窗口不可用错误
    async fn capture_active_window(&self) -> crate::AppResult<Capture> {
        let window = self.display.active_window()?;
        let geometry = self.display.connection.get_geometry(window)?.reply()?;
        let position = self
            .display
            .connection
            .translate_coordinates(window, self.display.screen().root, 0, 0)?
            .reply()?;
        let left = i32::from(position.dst_x).max(0);
        let top = i32::from(position.dst_y).max(0);
        let right = (i32::from(position.dst_x) + i32::from(geometry.width))
            .min(i32::from(self.display.screen().width_in_pixels));
        let bottom = (i32::from(position.dst_y) + i32::from(geometry.height))
            .min(i32::from(self.display.screen().height_in_pixels));
        let size = (right - left, bottom - top);
        let pixels = self.rectangle((left, top), size)?;
        let pixmap = tiny_skia::Pixmap::from_vec(
            pixels,
            tiny_skia::IntSize::from_wh(size.0 as u32, size.1 as u32)
                .ok_or("empty active window")?,
        )
        .ok_or("invalid X11 pixels")?;
        Ok(Capture {
            pixmap: crate::types::frozen_pixmap::FrozenPixmap::cached(pixmap),
            scale: 1.0,
        })
    }
}
