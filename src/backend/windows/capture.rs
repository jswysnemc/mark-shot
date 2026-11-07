//! 【Windows截图】【业务适配】原生BGRA按原始网格转为共享RGBA帧，持续截图绑定同一输出

use super::native;
use crate::{
    backend::{CaptureMethod, capture_session},
    types::{Capture, CaptureResult, MonitorFrame, Output, StreamInfo},
};
use async_trait::async_trait;

pub struct Method {
    pub include_cursor: bool,
}

#[async_trait]
impl CaptureMethod for Method {
    /// 【Windows截图】【显示器捕获】按outputs捕获完整物理像素，返回紧密RGBA帧，输出序号保持请求顺序
    async fn capture_frame(&self, outputs: &[Output]) -> crate::AppResult<CaptureResult> {
        let mut frames = Vec::with_capacity(outputs.len());
        for (output, monitor) in outputs.iter().enumerate() {
            let bounds = monitor.native.bounds;
            let mut pixels = native::capture(bounds, self.include_cursor)?;
            crate::utils::to_rgba(&mut pixels, true, false);
            frames.push(MonitorFrame {
                output,
                pixels,
                pw_width: bounds.width,
                pw_height: bounds.height,
                pw_stride: bounds.width * 4,
                info: StreamInfo {
                    node_id: 0,
                    size: Some((bounds.width as i32, bounds.height as i32)),
                    position: Some((bounds.left, bounds.top)),
                },
            });
        }
        Ok(CaptureResult { frames })
    }

    /// 【Windows截图】【持续会话】按outputs建立固定原生目标，返回复用捕获器的持续会话，不创建周期线程
    async fn open_session(
        &self,
        outputs: &[Output],
    ) -> capture_session::Result<Box<dyn capture_session::CaptureSession>> {
        let first = self
            .capture_frame(outputs)
            .await
            .map_err(|error| error.to_string())?;
        Ok(capture_session::seeded(
            capture_session::repeated(
                Box::new(Self {
                    include_cursor: self.include_cursor,
                }),
                outputs,
            ),
            first,
        ))
    }

    /// 【Windows截图】【活动窗口】无参数，捕获前台窗口可见物理范围，返回原始像素或明确系统错误
    async fn capture_active_window(&self) -> crate::AppResult<Capture> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
        let hwnd = unsafe { GetForegroundWindow() };
        let mut rect = unsafe { std::mem::zeroed() };
        if hwnd.is_null() || unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let bounds = native::Bounds {
            left: rect.left,
            top: rect.top,
            width: (rect.right - rect.left).max(1) as u32,
            height: (rect.bottom - rect.top).max(1) as u32,
        };
        let mut pixels = native::capture(bounds, self.include_cursor)?;
        crate::utils::to_rgba(&mut pixels, true, false);
        let pixmap = tiny_skia::Pixmap::from_vec(
            pixels,
            tiny_skia::IntSize::from_wh(bounds.width, bounds.height)
                .ok_or("invalid window capture dimensions")?,
        )
        .ok_or("invalid window capture frame")?;
        Ok(Capture {
            pixmap: crate::types::frozen_pixmap::FrozenPixmap::cached(pixmap),
            scale: 1.0,
        })
    }
}
