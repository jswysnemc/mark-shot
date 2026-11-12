//! 【媒体库】【固定接口】仅声明自有 C ABI，不跨动态库暴露编解码内部类型

use std::ffi::{c_char, c_int, c_void};

pub type Open = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    *mut c_char,
    usize,
) -> *mut c_void;
pub type Text = unsafe extern "C" fn(*mut c_void) -> *const c_char;
pub type Video = unsafe extern "C" fn(*mut c_void, *const u8, c_int, i64) -> c_int;
pub type Audio = unsafe extern "C" fn(*mut c_void, *const f32, c_int, i64) -> c_int;
pub type Finish = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type Destroy = unsafe extern "C" fn(*mut c_void);
pub type Cancel = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type Export = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    c_int,
    Cancel,
    *mut c_void,
    *mut c_char,
    usize,
) -> c_int;

pub struct Api {
    #[cfg(target_os = "linux")]
    pub pulse: super::pulse::Api,
    #[cfg(target_os = "linux")]
    pub surface_open: super::dmabuf::Open,
    #[cfg(target_os = "linux")]
    pub surface_destroy: Destroy,
    #[cfg(target_os = "linux")]
    pub dmabuf: super::dmabuf::Submit,
    pub open: Open,
    pub error: Text,
    pub name: Text,
    pub video: Video,
    pub audio: Audio,
    pub finish: Finish,
    pub destroy: Destroy,
    pub export: Export,
    pub _library: libloading::Library,
}

impl Api {
    /// 【媒体库】【受控加载】按 path 加载随包媒体库并核对 ABI，返回持有库生命周期的函数表
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        unsafe {
            let library =
                libloading::Library::new(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let version = library
                .get::<unsafe extern "C" fn() -> u32>(b"ms_media_abi\0")
                .map_err(|e| e.to_string())?;
            if version() != 2 {
                return Err("Unsupported media library ABI".into());
            }
            Ok(Self {
                #[cfg(target_os = "linux")]
                pulse: super::pulse::Api::load(&library)?,
                #[cfg(target_os = "linux")]
                surface_open: *library
                    .get(b"ms_surface_open\0")
                    .map_err(|e| e.to_string())?,
                #[cfg(target_os = "linux")]
                surface_destroy: *library
                    .get(b"ms_surface_destroy\0")
                    .map_err(|e| e.to_string())?,
                #[cfg(target_os = "linux")]
                dmabuf: *library
                    .get(b"ms_encoder_dmabuf\0")
                    .map_err(|e| e.to_string())?,
                open: *library
                    .get(b"ms_encoder_open\0")
                    .map_err(|e| e.to_string())?,
                error: *library
                    .get(b"ms_encoder_error\0")
                    .map_err(|e| e.to_string())?,
                name: *library
                    .get(b"ms_encoder_name\0")
                    .map_err(|e| e.to_string())?,
                video: *library
                    .get(b"ms_encoder_video\0")
                    .map_err(|e| e.to_string())?,
                audio: *library
                    .get(b"ms_encoder_audio\0")
                    .map_err(|e| e.to_string())?,
                finish: *library
                    .get(b"ms_encoder_finish\0")
                    .map_err(|e| e.to_string())?,
                destroy: *library
                    .get(b"ms_encoder_destroy\0")
                    .map_err(|e| e.to_string())?,
                export: *library.get(b"ms_export\0").map_err(|e| e.to_string())?,
                _library: library,
            })
        }
    }
}
