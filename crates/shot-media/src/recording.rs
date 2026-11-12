//! 【媒体库】【编码资源】单线程拥有编码器，Rust 输入边界保护 C 库的行距和数组读取

use super::{Media, message, path_string};
use std::{
    ffi::{c_char, c_void},
    path::Path,
    ptr::NonNull,
};

pub struct Settings {
    pub device: Option<std::path::PathBuf>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// 0 为较小文件，1 为均衡，2 为高质量
    pub quality: u32,
    /// 0 自动，1 强制硬件，2 软件
    pub mode: u32,
    pub audio: bool,
}

pub struct Encoder {
    pub(crate) media: Media,
    pub(crate) handle: NonNull<c_void>,
    size: (u32, u32),
}

impl Encoder {
    /// 【媒体库】【编码建立】按 media、path 和 settings 建立独占编码器，返回启动失败原因
    pub fn open(media: &Media, path: &Path, settings: &Settings) -> Result<Self, String> {
        if settings.width > 32768
            || settings.height > 32768
            || settings.fps > 60
            || settings.mode > 2
            || settings.quality > 2
        {
            return Err("Invalid recording dimensions or settings".into());
        }
        let path = path_string(path)?;
        let device = settings.device.as_deref().map(path_string).transpose()?;
        let mut error = [0 as c_char; 512];
        let handle = unsafe {
            (media.0.open)(
                path.as_ptr(),
                device
                    .as_ref()
                    .map_or(std::ptr::null(), |device| device.as_ptr()),
                settings.width as i32,
                settings.height as i32,
                settings.fps as i32,
                settings.quality as i32,
                settings.mode as i32,
                i32::from(settings.audio),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        Ok(Self {
            media: media.clone(),
            handle: NonNull::new(handle).ok_or_else(|| message(error.as_ptr()))?,
            size: (settings.width, settings.height),
        })
    }

    /// 【媒体库】【实际编码器】无参数，返回本会话确认使用的编码器名称
    pub fn name(&self) -> String {
        message(unsafe { (self.media.0.name)(self.handle.as_ptr()) })
    }

    /// 【媒体库】【视频提交】按 rgba、stride、pts_us 同步提交帧，返回完成消费或错误，不借用到调用之后
    pub fn video(&mut self, rgba: &[u8], stride: usize, pts_us: i64) -> Result<(), String> {
        let needed = stride
            .checked_mul(self.size.1.saturating_sub(1) as usize)
            .and_then(|v| v.checked_add(self.size.0 as usize * 4));
        if stride < self.size.0 as usize * 4
            || stride > i32::MAX as usize
            || needed.is_none_or(|v| v > rgba.len())
            || pts_us < 0
        {
            return Err("Invalid RGBA frame storage".into());
        }
        self.check(unsafe {
            (self.media.0.video)(self.handle.as_ptr(), rgba.as_ptr(), stride as i32, pts_us)
        })
    }

    /// 【媒体库】【音频提交】按双声道交错 samples 和 pts_us 交付 48kHz 浮点样本，返回编码结果
    pub fn audio(&mut self, samples: &[f32], pts_us: i64) -> Result<(), String> {
        if !samples.len().is_multiple_of(2) || samples.len() > 96000 || pts_us < 0 {
            return Err("Invalid stereo audio block".into());
        }
        self.check(unsafe {
            (self.media.0.audio)(
                self.handle.as_ptr(),
                samples.as_ptr(),
                (samples.len() / 2) as i32,
                pts_us,
            )
        })
    }

    /// 【媒体库】【结束封装】无参数，排空编码器并写尾部，失败仍保持对象用于诊断
    pub fn finish(&mut self) -> Result<(), String> {
        self.check(unsafe { (self.media.0.finish)(self.handle.as_ptr()) })
    }

    /// 【媒体库】【错误转换】按 code 返回成功或编码器持有的诊断，不覆盖原始错误
    pub(crate) fn check(&self, code: i32) -> Result<(), String> {
        if code < 0 {
            Err(message(unsafe {
                (self.media.0.error)(self.handle.as_ptr())
            }))
        } else {
            Ok(())
        }
    }
}

impl Drop for Encoder {
    /// 【媒体库】【资源回收】无参数，销毁编码器后才允许卸载动态库
    fn drop(&mut self) {
        unsafe { (self.media.0.destroy)(self.handle.as_ptr()) };
    }
}
