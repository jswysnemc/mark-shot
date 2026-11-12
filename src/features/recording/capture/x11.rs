//! 【X11录制】【共享像素】复用服务端共享段，远程服务器回退 GetImage，不依赖 OpenGL

use super::{Frame, Source};
use crate::{
    backend::x11::{connection::Display, pixels::Format},
    features::recording::Target,
};
use memmap2::MmapMut;
use std::time::Duration;
use x11rb::{
    connection::Connection,
    protocol::{shm::ConnectionExt as _, xfixes::ConnectionExt as _, xproto::*},
};

pub(super) struct Capture {
    display: Display,
    output_id: u32,
    origin: (i16, i16),
    size: (u16, u16),
    format: Format,
    shared: Option<(u32, MmapMut)>,
    pixels: Vec<u8>,
    cursor: bool,
}

impl Capture {
    /// 【X11录制】【来源建立】按 display、target 和 cursor 固定显示器并创建共享段，返回初始化错误
    pub fn new(display: Display, target: &Target, cursor: bool) -> Result<Self, String> {
        let setup = || -> crate::AppResult<Self> {
            let outputs = display.outputs()?;
            let output = super::choose(&outputs, target)?;
            let (width, height) = output
                .info
                .logical_size
                .ok_or("X11 output has no dimensions")?;
            let origin = output.info.logical_position.unwrap_or(output.info.location);
            let screen = display.screen();
            let format = Format::new(&display, screen.root_depth, screen.root_visual)?;
            let length = format
                .stride(width as usize)
                .checked_mul(height as usize)
                .ok_or("X11 frame size overflow")?;
            if length > 128 * 1024 * 1024 {
                return Err("X11 frame exceeds recording memory budget".into());
            }
            let shared = (|| -> crate::AppResult<_> {
                let version = display.connection.shm_query_version()?.reply()?;
                if (version.major_version, version.minor_version) < (1, 2) {
                    return Err("MIT-SHM FD transfer unavailable".into());
                }
                let segment = display.connection.generate_id()?;
                let reply = display
                    .connection
                    .shm_create_segment(segment, length as u32, false)?
                    .reply()?;
                let mapping = unsafe { MmapMut::map_mut(&reply.shm_fd) };
                match mapping {
                    Ok(mapping) => Ok((segment, mapping)),
                    Err(error) => {
                        let _ = display.connection.shm_detach(segment);
                        Err(error.into())
                    }
                }
            })()
            .ok();
            Ok(Self {
                display: display.clone(),
                output_id: output.info.id,
                origin: (origin.0.try_into()?, origin.1.try_into()?),
                size: (width.try_into()?, height.try_into()?),
                format,
                shared,
                pixels: Vec::new(),
                cursor,
            })
        };
        setup().map_err(|e| e.to_string())
    }

    /// 【X11录制】【光标合成】无参数，把 XFixes 图像绘制到当前 RGBA，不改变根窗口原始内容
    fn cursor(&mut self) -> crate::AppResult {
        let cursor = self.display.connection.xfixes_get_cursor_image()?.reply()?;
        for row in 0..i32::from(cursor.height) {
            for column in 0..i32::from(cursor.width) {
                let x = i32::from(cursor.x) - i32::from(cursor.xhot) + column
                    - i32::from(self.origin.0);
                let y =
                    i32::from(cursor.y) - i32::from(cursor.yhot) + row - i32::from(self.origin.1);
                if x < 0 || y < 0 || x >= i32::from(self.size.0) || y >= i32::from(self.size.1) {
                    continue;
                }
                let argb = cursor.cursor_image[(row * i32::from(cursor.width) + column) as usize];
                let alpha = argb >> 24;
                let offset = (y as usize * usize::from(self.size.0) + x as usize) * 4;
                for channel in 0..3 {
                    self.pixels[offset + channel] = (((argb >> (16 - channel * 8)) & 255)
                        + u32::from(self.pixels[offset + channel]) * (255 - alpha) / 255)
                        .min(255) as u8;
                }
            }
        }
        Ok(())
    }
}

impl Source for Capture {
    /// 【X11录制】【共享取帧】按 timeout 调用服务器读取，返回当前借用像素；服务器连接失败立即报告
    fn next(&mut self, _timeout: Duration) -> Result<bool, String> {
        let capture = |this: &mut Self| -> crate::AppResult {
            let outputs = this.display.outputs()?;
            if !outputs.iter().any(|o| {
                o.info.id == this.output_id
                    && o.info.logical_size == Some((i32::from(this.size.0), i32::from(this.size.1)))
                    && o.info.logical_position.unwrap_or(o.info.location)
                        == (i32::from(this.origin.0), i32::from(this.origin.1))
            }) {
                return Err("录制显示器已变化 / Recording display changed".into());
            }
            if let Some((segment, mapping)) = &this.shared {
                this.display
                    .connection
                    .shm_get_image(
                        this.display.screen().root,
                        this.origin.0,
                        this.origin.1,
                        this.size.0,
                        this.size.1,
                        u32::MAX,
                        u8::from(ImageFormat::Z_PIXMAP),
                        *segment,
                        0,
                    )?
                    .reply()?;
                this.format.decode_into(
                    mapping,
                    this.size.0.into(),
                    this.size.1.into(),
                    &mut this.pixels,
                )?;
            } else {
                let reply = this
                    .display
                    .connection
                    .get_image(
                        ImageFormat::Z_PIXMAP,
                        this.display.screen().root,
                        this.origin.0,
                        this.origin.1,
                        this.size.0,
                        this.size.1,
                        u32::MAX,
                    )?
                    .reply()?;
                this.format.decode_into(
                    &reply.data,
                    this.size.0.into(),
                    this.size.1.into(),
                    &mut this.pixels,
                )?;
            }
            if this.cursor {
                this.cursor()?;
            }
            Ok(())
        };
        capture(self).map_err(|e| e.to_string())?;
        Ok(true)
    }
    /// 【X11录制】【当前像素】无参数，借用最近一帧的复用缓冲
    fn frame(&self) -> Option<Frame<'_>> {
        if self.pixels.is_empty() {
            return None;
        }
        Some(Frame {
            gpu: None,
            pixels: &self.pixels,
            size: (self.size.0.into(), self.size.1.into()),
            stride: usize::from(self.size.0) * 4,
        })
    }
    /// 【X11录制】【暂停控制】按 paused 停止上层取帧即可，无持续抓取资源需要切换
    fn pause(&mut self, _paused: bool) -> Result<(), String> {
        Ok(())
    }
    /// 【X11录制】【实际路径】无参数，返回已成功建立的共享内存或协议回退方式
    fn name(&self) -> &'static str {
        if self.shared.is_some() {
            "X11 MIT-SHM"
        } else {
            "X11 GetImage"
        }
    }
}

impl Drop for Capture {
    /// 【X11录制】【共享段回收】无参数，先解除服务器共享段，再释放客户端映射与连接
    fn drop(&mut self) {
        if let Some((segment, _)) = &self.shared {
            let _ = self.display.connection.shm_detach(*segment);
            let _ = self.display.connection.flush();
        }
    }
}
