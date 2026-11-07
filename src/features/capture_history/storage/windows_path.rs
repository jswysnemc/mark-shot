//! 【Windows截图历史】【句柄路径】固定真实目录的可访问卷路径，支持没有DOS挂载名称的卷

use std::{
    fs::File,
    io,
    os::windows::{ffi::OsStringExt, io::AsRawHandle},
    path::PathBuf,
};
use windows_sys::Win32::Storage::FileSystem::{
    GetFinalPathNameByHandleW, VOLUME_NAME_DOS, VOLUME_NAME_GUID, VOLUME_NAME_NT,
};

/// 【Windows截图历史】【卷解析】按file依次取得DOS、卷GUID或NT路径，返回可访问目录；全部失败时返回系统错误，不退回相对路径
pub(super) fn resolve(file: &File) -> io::Result<PathBuf> {
    let mut failure = io::Error::other("cannot resolve the history directory handle");
    for volume in [VOLUME_NAME_DOS, VOLUME_NAME_GUID, VOLUME_NAME_NT] {
        let mut buffer = vec![0_u16; 32768];
        let count = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                volume,
            )
        } as usize;
        if count == 0 {
            failure = io::Error::last_os_error();
            continue;
        }
        if count >= buffer.len() {
            failure = io::Error::other("history directory path exceeds the system limit");
            continue;
        }
        // 1. 【Windows截图历史】【NT命名空间】NT设备路径需要GLOBALROOT前缀才能交给标准文件API
        let path = if volume == VOLUME_NAME_NT {
            let mut prefix = "\\\\?\\GLOBALROOT".encode_utf16().collect::<Vec<_>>();
            prefix.extend_from_slice(&buffer[..count]);
            PathBuf::from(std::ffi::OsString::from_wide(&prefix))
        } else {
            PathBuf::from(std::ffi::OsString::from_wide(&buffer[..count]))
        };
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_dir() => return Ok(path),
            Ok(_) => failure = io::Error::other("resolved history path is not a directory"),
            Err(error) => failure = error,
        }
    }
    Err(failure)
}
