//! 【Windows剪贴板】【资源交接】打开范围自动关闭，系统内存只在成功交付后转移拥有权

use super::super::native;
use std::{
    io,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{GlobalFree, HGLOBAL},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
        },
        Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
    },
};

pub(super) struct Opened {
    _owner: Option<native::Window>,
}
impl Opened {
    /// 【Windows剪贴板】【范围打开】按owner打开剪贴板，返回自动关闭范围，短暂占用在有界期限内重试
    fn open(owner: Option<native::Window>) -> io::Result<Self> {
        let hwnd = owner
            .as_ref()
            .map_or(std::ptr::null_mut(), |window| window.handle() as _);
        let deadline = Instant::now() + Duration::from_millis(150);
        while unsafe { OpenClipboard(hwnd) } == 0 {
            if Instant::now() >= deadline {
                return Err(io::Error::last_os_error());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(Self { _owner: owner })
    }
    /// 【Windows剪贴板】【接收范围】无参数，返回只读打开范围，不创建窗口或修改已有数据
    pub(super) fn read() -> io::Result<Self> {
        Self::open(None)
    }
    /// 【Windows剪贴板】【拥有窗口】无参数，创建隐藏拥有窗口并打开剪贴板，返回范围，避免空拥有者导致交接失败
    pub(super) fn write() -> io::Result<Self> {
        let window = native::Window::new(
            native::WindowOptions {
                title: "Mark Shot clipboard owner".into(),
                bounds: native::Bounds {
                    left: 0,
                    top: 0,
                    width: 1,
                    height: 1,
                },
                minimum: (1, 1),
                topmost: false,
                activate: false,
                layered: false,
                owner: 0,
            },
            native::EventQueue::default(),
        )?;
        Self::open(Some(window))
    }
    /// 【Windows剪贴板】【拥有切换】无参数，清空旧数据并取得本次范围所有权，返回系统结果
    pub(super) fn clear(&self) -> io::Result<()> {
        if unsafe { EmptyClipboard() } == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
impl Drop for Opened {
    /// 【Windows剪贴板】【范围关闭】无参数，先关闭剪贴板再释放隐藏窗口，无返回值
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

pub(super) struct Buffer(HGLOBAL);
impl Buffer {
    /// 【Windows剪贴板】【私有分配】按bytes创建可移动系统内存，返回未交付缓冲，失败自动回收
    pub(super) fn new(bytes: &[u8]) -> io::Result<Self> {
        let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
        if memory.is_null() {
            return Err(io::Error::last_os_error());
        }
        let buffer = Self(memory);
        let pointer = unsafe { GlobalLock(memory) };
        if pointer.is_null() {
            return Err(io::Error::last_os_error());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
            GlobalUnlock(memory);
        }
        Ok(buffer)
    }
    /// 【Windows剪贴板】【所有权交付】按format交接self缓冲，返回系统结果；失败仍由本对象释放，成功由系统释放
    pub(super) fn give(mut self, format: u32) -> io::Result<()> {
        if unsafe { SetClipboardData(format, self.0.cast()) }.is_null() {
            return Err(io::Error::last_os_error());
        }
        self.0 = std::ptr::null_mut();
        Ok(())
    }
}
impl Drop for Buffer {
    /// 【Windows剪贴板】【未交付释放】无参数，只释放尚未属于系统的缓冲，无返回值
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                GlobalFree(self.0);
            }
        }
    }
}

/// 【Windows剪贴板】【有界复制】按format和limit复制系统数据，返回独立字节，系统句柄不由调用方释放
pub(super) fn read(format: u32, limit: usize) -> io::Result<Vec<u8>> {
    let handle = unsafe { GetClipboardData(format) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let count = unsafe { GlobalSize(handle.cast()) };
    if count == 0 || count > limit {
        return Err(io::Error::other("clipboard data exceeds limit"));
    }
    let pointer = unsafe { GlobalLock(handle.cast()) };
    if pointer.is_null() {
        return Err(io::Error::last_os_error());
    }
    let result = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), count) }.to_vec();
    unsafe {
        GlobalUnlock(handle.cast());
    }
    Ok(result)
}
