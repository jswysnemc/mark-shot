//! 【Windows窗口识别】【原生快照】枚举当前可见顶层窗口，排除本进程和不可见窗口

use super::{Window, geometry};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM},
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    },
};

/// 【Windows窗口识别】【枚举回调】按hwnd读取可见矩形追加到data，返回继续枚举，不保留系统句柄
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
    let mut bounds = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut bounds) } != 0
        && let Some(rect) = geometry::rect(
            f64::from(bounds.left),
            f64::from(bounds.top),
            f64::from(bounds.right - bounds.left),
            f64::from(bounds.bottom - bounds.top),
        )
    {
        let windows = unsafe { &mut *(data as *mut Vec<Window>) };
        geometry::append(windows, rect, Some(-(windows.len() as i32)));
    }
    1
}
/// 【Windows窗口识别】【快照查询】无参数，返回窗口层级顺序与物理矩形，枚举失败记录错误，不创建长期监听
pub(super) fn snapshot() -> Vec<Window> {
    let mut result = Vec::<Window>::new();
    if unsafe { EnumWindows(Some(collect), (&mut result as *mut Vec<Window>) as LPARAM) } == 0 {
        log::error!(
            "【Windows窗口识别】【快照查询】{}",
            std::io::Error::last_os_error()
        );
    }
    result
}
