//! 【媒体库】【进程内会话】延迟加载精简媒体库，编码与导出不创建外部进程

#[cfg(target_os = "linux")]
mod dmabuf;
mod ffi;
mod recording;
#[cfg(target_os = "linux")]
pub use dmabuf::Surface;
#[cfg(target_os = "linux")]
mod pulse;
#[cfg(target_os = "linux")]
pub use pulse::Pulse;
pub use recording::{Encoder, Settings};

use std::{
    ffi::{CStr, CString, c_char, c_void},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Clone)]
pub struct Media(Rc<ffi::Api>);

impl Media {
    /// 【媒体库】【按需初始化】无参数，加载可执行文件旁的媒体库，开发环境可显式指定绝对路径
    pub fn load() -> Result<Self, String> {
        let path = if let Some(path) = std::env::var_os("MARK_SHOT_MEDIA_LIBRARY") {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err("MARK_SHOT_MEDIA_LIBRARY must be absolute".into());
            }
            path
        } else {
            let executable = std::env::current_exe().map_err(|e| e.to_string())?;
            executable
                .parent()
                .ok_or("executable directory unavailable")?
                .join("media")
                .join(if cfg!(windows) {
                    "markshot-media.dll"
                } else {
                    "libmarkshot-media.so"
                })
        };
        ffi::Api::load(&path).map(|api| Self(Rc::new(api)))
    }

    /// 【媒体库】【导出转换】将 source 写到 destination，gif_fps 为空表示 MKV，cancelled 可取消，返回媒体错误
    pub fn export(
        &self,
        source: &Path,
        destination: &Path,
        gif_fps: Option<u32>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(), String> {
        if gif_fps.is_some_and(|fps| !(1..=30).contains(&fps)) {
            return Err("Invalid GIF frame rate".into());
        }
        let source = path_string(source)?;
        let destination = path_string(destination)?;
        let mut error = [0 as c_char; 512];
        let mut callback: &mut dyn FnMut() -> bool = &mut cancelled;
        let result = unsafe {
            (self.0.export)(
                source.as_ptr(),
                destination.as_ptr(),
                gif_fps.unwrap_or(0) as i32,
                cancel,
                (&mut callback as *mut &mut dyn FnMut() -> bool).cast(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if result < 0 {
            Err(message(error.as_ptr()))
        } else {
            Ok(())
        }
    }
}

/// 【媒体库】【取消桥接】按 context 调用当前导出闭包，捕获异常防止越过 C 调用栈，返回取消状态
unsafe extern "C" fn cancel(context: *mut c_void) -> i32 {
    let callback = unsafe { &mut *context.cast::<&mut dyn FnMut() -> bool>() };
    i32::from(std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback)).unwrap_or(true))
}

/// 【媒体库】【路径编码】按 path 返回 UTF-8 路径，拒绝无法表示的路径和内嵌零字节
fn path_string(path: &Path) -> Result<CString, String> {
    CString::new(path.to_str().ok_or("media path is not UTF-8")?).map_err(|e| e.to_string())
}

/// 【媒体库】【错误文本】按库返回的零结尾字符串复制文本，不保留动态库内存引用
fn message(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return "media library returned no error detail".into();
    }
    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}
