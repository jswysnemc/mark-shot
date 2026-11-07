//! 【Windows显示器】【原生查询】读取屏幕位置、设备名称与每显示器DPI，不创建窗口或读取像素

use std::{io, sync::OnceLock};
use windows_sys::Win32::{
    Foundation::{ERROR_ACCESS_DENIED, GetLastError, LPARAM, RECT},
    Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    },
    UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, GetDpiForSystem,
        MDT_EFFECTIVE_DPI, SetProcessDpiAwarenessContext,
    },
    UI::WindowsAndMessaging::MONITORINFOF_PRIMARY,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct Monitor {
    pub handle: isize,
    pub name: String,
    pub bounds: Bounds,
    pub dpi: u32,
    pub primary: bool,
}

/// 【Windows显示器】【DPI启用】无参数，在创建任何原生窗口前启用每显示器DPI，返回系统结果，既有感知上下文保持有效
pub fn enable_dpi() -> io::Result<()> {
    static RESULT: OnceLock<Result<(), u32>> = OnceLock::new();
    match RESULT.get_or_init(|| {
        let result =
            unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let error = unsafe { GetLastError() };
        if result != 0 || error == ERROR_ACCESS_DENIED {
            Ok(())
        } else {
            Err(error)
        }
    }) {
        Ok(()) => Ok(()),
        Err(error) => Err(io::Error::from_raw_os_error(*error as i32)),
    }
}

/// 【Windows显示器】【枚举回调】把 handle 对应信息追加到 data 指向的本次结果，其他参数来自系统；返回是否继续枚举
unsafe extern "system" fn collect(
    handle: HMONITOR,
    _dc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> i32 {
    let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if unsafe {
        GetMonitorInfoW(
            handle,
            (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
        )
    } == 0
    {
        return 1;
    }
    let mut dpi_x = 96;
    let mut dpi_y = 96;
    if unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } < 0 {
        dpi_x = unsafe { GetDpiForSystem() }.max(96);
    }
    let rect = info.monitorInfo.rcMonitor;
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    if width <= 0 || height <= 0 || width > i64::from(i32::MAX) || height > i64::from(i32::MAX) {
        return 1;
    }
    let length = info
        .szDevice
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(info.szDevice.len());
    // 1. 【Windows显示器】【回调借用】枚举在当前线程同步执行，data只指向本次调用持有的结果容器
    let result = unsafe { &mut *(data as *mut Vec<Monitor>) };
    result.push(Monitor {
        handle: handle as isize,
        name: String::from_utf16_lossy(&info.szDevice[..length]),
        bounds: Bounds {
            left: rect.left,
            top: rect.top,
            width: width as u32,
            height: height as u32,
        },
        dpi: dpi_x.clamp(24, 768),
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    });
    1
}

/// 【Windows显示器】【完整枚举】无参数，返回当前显示器原生位置及DPI，不创建图像副本，没有屏幕时返回错误
pub fn monitors() -> io::Result<Vec<Monitor>> {
    enable_dpi()?;
    let mut result = Vec::<Monitor>::new();
    if unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(collect),
            (&mut result as *mut Vec<Monitor>) as LPARAM,
        )
    } == 0
    {
        return Err(super::last_error());
    }
    if result.is_empty() {
        return Err(io::Error::other("Windows did not report any displays"));
    }
    Ok(result)
}
