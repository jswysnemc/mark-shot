//! 【Windows录制】【纹理采集】WGC 保留两帧池，只交付最新纹理，不读回 CPU 像素

use super::device::Device;
use crate::features::recording::Target;
use windows::{
    Graphics::{Capture::*, DirectX::DirectXPixelFormat},
    Win32::{
        Graphics::{Direct3D11::ID3D11Texture2D, Gdi::HMONITOR},
        System::WinRT::{
            Direct3D11::IDirect3DDxgiInterfaceAccess,
            Graphics::Capture::IGraphicsCaptureItemInterop,
        },
    },
    core::{Interface, Result},
};

pub(super) struct Capture {
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    pub size: (u32, u32),
    latest: Option<Direct3D11CaptureFrame>,
    item: GraphicsCaptureItem,
}

impl Capture {
    /// 【Windows录制】【启动捕获】按 device、target、cursor 创建指定显示器会话，返回真实权限或设备错误
    pub fn new(
        device: &Device,
        target: &Target,
        cursor: bool,
    ) -> std::result::Result<Self, String> {
        if !GraphicsCaptureSession::IsSupported().map_err(|error| error.to_string())? {
            return Err("Windows Graphics Capture 不可用 / WGC unavailable".into());
        }
        let monitors =
            crate::backend::windows::native::monitors().map_err(|error| error.to_string())?;
        let monitor = if target.display.is_empty() {
            monitors
                .iter()
                .find(|monitor| monitor.primary)
                .or(monitors.first())
        } else {
            monitors
                .iter()
                .find(|monitor| monitor.name == target.display)
        }
        .ok_or("录制显示器已移除 / Display removed")?;
        let create = || -> Result<Self> {
            let factory: IGraphicsCaptureItemInterop =
                windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem =
                unsafe { factory.CreateForMonitor(HMONITOR(monitor.handle as *mut _))? };
            let size = item.Size()?;
            let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &device.winrt()?,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                size,
            )?;
            let session = pool.CreateCaptureSession(&item)?;
            session.SetIsCursorCaptureEnabled(cursor)?;
            session.StartCapture()?;
            Ok(Self {
                pool,
                session,
                size: (size.Width as u32, size.Height as u32),
                latest: None,
                item,
            })
        };
        create().map_err(|error| format!("WGC: {error}"))
    }

    /// 【Windows录制】【最新帧】无参数，排空已到达帧并关闭旧租约，返回保留帧的 GPU 纹理及 QPC 时间
    pub fn texture(&mut self) -> std::result::Result<Option<(ID3D11Texture2D, i64)>, String> {
        while let Ok(frame) = self.pool.TryGetNextFrame() {
            let size = frame.ContentSize().map_err(|error| error.to_string())?;
            if (size.Width as u32, size.Height as u32) != self.size {
                return Err(
                    "录制显示器尺寸已变化，请重新开始 / Capture size changed; restart recording"
                        .into(),
                );
            }
            if let Some(old) = self.latest.replace(frame) {
                let _ = old.Close();
            }
        }
        let Some(frame) = &self.latest else {
            return Ok(None);
        };
        let get = || -> Result<_> {
            let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
            Ok((
                unsafe { access.GetInterface::<ID3D11Texture2D>()? },
                frame.SystemRelativeTime()?.Duration,
            ))
        };
        get().map(Some).map_err(|error| error.to_string())
    }
}
impl Drop for Capture {
    /// 【Windows录制】【捕获关闭】无参数，先关会话与保留帧，再关池，不遗留系统捕获状态
    fn drop(&mut self) {
        let _ = self.session.Close();
        if let Some(frame) = self.latest.take() {
            let _ = frame.Close();
        }
        let _ = self.pool.Close();
        let _ = &self.item;
    }
}
