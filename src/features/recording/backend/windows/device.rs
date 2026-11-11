//! 【Windows录制】【设备生命周期】COM、Media Foundation 与 D3D11 在录制线程内建立和释放

use windows::{
    Win32::{
        Graphics::{Direct3D::D3D_DRIVER_TYPE_HARDWARE, Direct3D11::*, Dxgi::IDXGIDevice},
        Media::MediaFoundation::*,
        System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    },
    core::{Interface, Result},
};

pub(super) struct Runtime;
impl Runtime {
    /// 【Windows录制】【运行时启动】无参数，在当前线程初始化 COM 与媒体框架，返回资源错误
    pub fn new() -> Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
            if let Err(error) = MFStartup(MF_VERSION, MFSTARTUP_FULL) {
                RoUninitialize();
                return Err(error);
            }
        }
        Ok(Self)
    }
}
impl Drop for Runtime {
    /// 【Windows录制】【运行时释放】无参数，在所有媒体资源析构后关闭当前线程的运行时
    fn drop(&mut self) {
        unsafe {
            let _ = MFShutdown();
            RoUninitialize();
        }
    }
}

pub(super) struct Device {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub manager: IMFDXGIDeviceManager,
}
impl Device {
    /// 【Windows录制】【图形设备】无参数，创建支持视频处理且线程保护的 D3D11 设备和媒体管理器
    pub fn new() -> Result<Self> {
        unsafe {
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
            let device = device.unwrap();
            let protection: ID3D11Multithread = device.cast()?;
            let _ = protection.SetMultithreadProtected(true);
            let mut manager = None;
            let mut token = 0;
            MFCreateDXGIDeviceManager(&mut token, &mut manager)?;
            let manager = manager.unwrap();
            manager.ResetDevice(&device, token)?;
            Ok(Self {
                device,
                context: context.unwrap(),
                manager,
            })
        }
    }

    /// 【Windows录制】【WinRT桥接】无参数，返回同一 DXGI 设备的 WinRT 接口，不创建第二设备
    pub fn winrt(&self) -> Result<windows::Graphics::DirectX::Direct3D11::IDirect3DDevice> {
        let dxgi: IDXGIDevice = self.device.cast()?;
        unsafe {
            windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?
                .cast()
        }
    }
}
