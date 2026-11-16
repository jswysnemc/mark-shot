//! 【开机启动】【Windows注册】仅修改当前用户 Run 启动项，使用宽字符保留中文程序及配置路径

use std::{io, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
        RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
    },
};

/// 【开机启动】【注册表更新】按 executable、config、enabled 创建或删除 MarkShot 用户启动值，返回系统错误
pub(super) fn update(executable: &Path, config: &Path, enabled: bool) -> io::Result<()> {
    let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let name: Vec<u16> = "MarkShot\0".encode_utf16().collect();
    let command = enabled.then(|| command(executable, config));
    write(&key, &name, command.as_deref())
}

/// 【开机启动】【注册表写入】按 key、name 和 command 更新用户字符串值，空命令删除单个值，返回系统错误
fn write(key: &[u16], name: &[u16], command: Option<&[u16]>) -> io::Result<()> {
    let enabled = command.is_some();
    let mut handle = std::ptr::null_mut();
    // 1. 【开机启动】【用户权限】只打开当前用户键，关闭时不创建缺失的注册表路径
    let status = unsafe {
        if enabled {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut handle,
                std::ptr::null_mut(),
            )
        } else {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut handle,
            )
        }
    };
    if !enabled && status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    // 2. 【开机启动】【命令写入】程序及配置均独立加引号，Run 值按 REG_SZ 写入且包含终止字符
    let status = unsafe {
        if let Some(command) = command {
            RegSetValueExW(
                handle,
                name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                size_of_val(command) as u32,
            )
        } else {
            RegDeleteValueW(handle, name.as_ptr())
        }
    };
    unsafe { RegCloseKey(handle) };
    if status == ERROR_SUCCESS || !enabled && status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

/// 【开机启动】【宽字符命令】按 executable、config 返回含终止字符的后台启动命令，避免路径依赖当前目录
fn command(executable: &Path, config: &Path) -> Vec<u16> {
    let mut value: Vec<u16> = "\"".encode_utf16().collect();
    value.extend(executable.as_os_str().encode_wide());
    value.extend("\" --background --config \"".encode_utf16());
    value.extend(config.as_os_str().encode_wide());
    value.extend("\"\0".encode_utf16());
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Registry::{KEY_QUERY_VALUE, RegDeleteKeyW, RegQueryValueExW};

    /// 【开机启动回归】【系统读取】按 key、name 返回注册表字符串与类型，供独立读取接口核对真实写入
    fn read(key: &[u16], name: &[u16]) -> Option<(u32, Vec<u16>)> {
        let mut handle = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut handle,
            )
        };
        assert_eq!(status, ERROR_SUCCESS);
        let mut kind = 0;
        let mut bytes = [0u16; 512];
        let mut size = size_of_val(&bytes) as u32;
        let status = unsafe {
            RegQueryValueExW(
                handle,
                name.as_ptr(),
                std::ptr::null_mut(),
                &mut kind,
                bytes.as_mut_ptr().cast(),
                &mut size,
            )
        };
        unsafe { RegCloseKey(handle) };
        if status == ERROR_FILE_NOT_FOUND {
            return None;
        }
        assert_eq!(status, ERROR_SUCCESS);
        Some((kind, bytes[..size as usize / size_of::<u16>()].to_vec()))
    }

    /// 【开机启动回归】【注册生命周期】无参数，在独立测试键验证真实宽字符注册、更新、幂等删除及相邻值保留
    #[test]
    fn registry_roundtrip_preserves_other_values() {
        let directory = tempfile::tempdir().unwrap();
        let key: Vec<u16> = format!(
            "Software\\MarkShotAutostartTest-{}\0",
            directory.path().file_name().unwrap().to_string_lossy()
        )
        .encode_utf16()
        .collect();
        let name: Vec<u16> = "MarkShot\0".encode_utf16().collect();
        let neighbor: Vec<u16> = "Other\0".encode_utf16().collect();
        let command = command(
            Path::new("C:\\应用 程序\\mark-shot.exe"),
            Path::new("C:\\用户\\配置.toml"),
        );
        write(&key, &name, None).unwrap();
        write(&key, &neighbor, Some(&command)).unwrap();
        write(&key, &name, Some(&command)).unwrap();
        assert_eq!(read(&key, &name), Some((REG_SZ, command.clone())));
        let moved: Vec<u16> = "\"C:\\New Location\\mark-shot.exe\" --background\0"
            .encode_utf16()
            .collect();
        write(&key, &name, Some(&moved)).unwrap();
        assert_eq!(read(&key, &name), Some((REG_SZ, moved)));
        write(&key, &name, None).unwrap();
        write(&key, &name, None).unwrap();
        assert_eq!(read(&key, &name), None);
        assert_eq!(read(&key, &neighbor), Some((REG_SZ, command)));
        assert_eq!(
            unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, key.as_ptr()) },
            ERROR_SUCCESS
        );
    }

    /// 【开机启动回归】【Windows路径】无参数，验证中文与空格路径完整保留，并以独立参数启动后台服务
    #[test]
    fn preserves_unicode_and_spaces_in_command() {
        let command = super::command(
            std::path::Path::new("C:\\应用 程序\\mark-shot.exe"),
            std::path::Path::new("C:\\用户\\配置 文件.toml"),
        );
        assert_eq!(
            String::from_utf16(&command).unwrap(),
            "\"C:\\应用 程序\\mark-shot.exe\" --background --config \"C:\\用户\\配置 文件.toml\"\0"
        );
    }
}
