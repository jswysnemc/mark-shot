//! 【Windows窗口识别】【可见快照】保留真实前后顺序，过滤隐藏与穿透窗口，优先使用可见边框

use crate::backend::window_detection::{Window, geometry};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    UI::WindowsAndMessaging::*,
};

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmGetWindowAttribute(
        hwnd: HWND,
        attribute: u32,
        value: *mut std::ffi::c_void,
        size: u32,
    ) -> i32;
}

/// 【Windows窗口识别】【可见边界】按hwnd返回排除隐形缩放边框的物理范围，DWM不可用时回退Win32矩形
fn bounds(hwnd: HWND) -> Option<tiny_skia::Rect> {
    let mut rect = RECT::default();
    let extended = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            9,
            (&mut rect as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        )
    };
    if (extended < 0 || rect.right <= rect.left || rect.bottom <= rect.top)
        && unsafe { GetWindowRect(hwnd, &mut rect) } == 0
    {
        return None;
    }
    geometry::rect(
        rect.left.into(),
        rect.top.into(),
        f64::from(rect.right) - f64::from(rect.left),
        f64::from(rect.bottom) - f64::from(rect.top),
    )
}

/// 【Windows窗口识别】【快照回调】按hwnd和data收集可选择窗口，返回继续枚举，句柄仅在当前截图会话使用
unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> i32 {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid == std::process::id()
        || unsafe { IsWindowVisible(hwnd) } == 0
        || unsafe { IsIconic(hwnd) } != 0
    {
        return 1;
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    if style & (WS_EX_LAYERED | WS_EX_TRANSPARENT) == WS_EX_LAYERED | WS_EX_TRANSPARENT {
        return 1;
    }
    let Some(rect) = bounds(hwnd) else {
        return 1;
    };
    let mut cloaked = 0u32;
    if unsafe {
        DwmGetWindowAttribute(
            hwnd,
            14,
            (&mut cloaked as *mut u32).cast(),
            size_of::<u32>() as u32,
        )
    } >= 0
        && cloaked != 0
    {
        return 1;
    }
    let windows = unsafe { &mut *(data as *mut Vec<Window>) };
    if windows.len() < 4096 {
        windows.push(Window {
            rect,
            z_order: Some(-(windows.len() as i32)),
            native_id: hwnd as usize,
        });
    }
    1
}

/// 【Windows窗口识别】【冻结枚举】无参数，返回从前到后的物理窗口矩形，不读取其他进程控件或等待UIA
pub(crate) fn snapshot() -> Vec<Window> {
    let mut windows = Vec::new();
    if unsafe { EnumWindows(Some(collect), (&mut windows as *mut Vec<Window>) as LPARAM) } == 0 {
        log::error!(
            "【Windows窗口识别】【冻结枚举】{}",
            std::io::Error::last_os_error()
        );
    }
    windows
}
