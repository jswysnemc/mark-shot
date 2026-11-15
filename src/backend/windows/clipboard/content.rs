//! 【Windows剪贴板】【文件及HTML】有界读取复制文件和原生 HTML 格式

use super::{Opened, memory};
use std::path::PathBuf;
use windows_sys::Win32::{
    System::DataExchange::{GetClipboardData, RegisterClipboardFormatW},
    UI::Shell::DragQueryFileW,
};

/// 【Windows剪贴板】【复制文件】无参数，通过 CF_HDROP 读取至多二百五十六个 Unicode 文件路径，返回本地路径列表
pub fn read_files() -> crate::AppResult<Vec<PathBuf>> {
    // 1. 【Windows剪贴板】【文件校验】确认原生格式及有界内存，拒绝超量文件
    let _clipboard = Opened::read()?;
    let bytes = memory::read(15, 1024 * 1024)?;
    if bytes.len() < 20 {
        return Err("invalid clipboard file list".into());
    }
    let handle = unsafe { GetClipboardData(15) };
    let count = unsafe { DragQueryFileW(handle.cast(), u32::MAX, std::ptr::null_mut(), 0) };
    if count > 256 {
        return Err("clipboard contains too many files".into());
    }
    let mut files = Vec::new();
    // 2. 【Windows剪贴板】【路径读取】逐项读取完整 Unicode 路径，确认长度未发生变化
    for index in 0..count {
        let size = unsafe { DragQueryFileW(handle.cast(), index, std::ptr::null_mut(), 0) };
        if size == 0 || size > 32768 {
            return Err("invalid clipboard path length".into());
        }
        let mut path = vec![0_u16; size as usize + 1];
        let written = unsafe { DragQueryFileW(handle.cast(), index, path.as_mut_ptr(), size + 1) };
        if written != size {
            return Err("clipboard file list changed".into());
        }
        files.push(PathBuf::from(String::from_utf16(&path[..size as usize])?));
    }
    Ok(files)
}

/// 【Windows剪贴板】【HTML读取】无参数，返回最多六十四 KiB 的 UTF-8 HTML 正文，拒绝无效编码
pub fn read_html() -> crate::AppResult<String> {
    let _clipboard = Opened::read()?;
    let name: Vec<_> = "HTML Format".encode_utf16().chain([0]).collect();
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    if format == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let bytes = memory::read(format, 64 * 1024)?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    Ok(std::str::from_utf8(&bytes[..end])?.into())
}
