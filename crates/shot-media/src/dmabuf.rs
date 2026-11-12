//! 【显存帧】【所有权边界】GBM 对象和导出 FD 一起持有，提交同步消费，不把借用输入留给编码器

use crate::{Encoder, Media};
use std::{
    ffi::c_void,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    path::{Path, PathBuf},
    ptr::NonNull,
};

#[repr(C)]
#[derive(Default)]
pub(crate) struct Descriptor {
    width: i32,
    height: i32,
    format: u32,
    planes: i32,
    modifier: u64,
    fd: [i32; 4],
    size: [u32; 4],
    offset: [u32; 4],
    stride: [u32; 4],
}
pub(crate) type Open =
    unsafe extern "C" fn(i32, i32, i32, u32, *const u64, i32, *mut Descriptor) -> *mut c_void;
pub(crate) type Submit = unsafe extern "C" fn(*mut c_void, *const Descriptor, i32, i32, i64) -> i32;

pub struct Surface {
    media: Media,
    handle: NonNull<c_void>,
    descriptor: Descriptor,
    fds: Vec<OwnedFd>,
    device: PathBuf,
}

impl Surface {
    /// 【显存帧】【缓冲分配】按 media、device、size、format 与 modifiers 创建兼容缓冲，返回拥有资源的句柄
    pub fn new(
        media: &Media,
        device: &Path,
        size: (u32, u32),
        format: u32,
        modifiers: &[u64],
    ) -> Result<Self, String> {
        if size.0 > 16384 || size.1 > 16384 || modifiers.is_empty() || modifiers.len() > 256 {
            return Err("Invalid DMA-BUF constraints".into());
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(device)
            .map_err(|e| e.to_string())?;
        let mut descriptor = Descriptor::default();
        let handle = NonNull::new(unsafe {
            (media.0.surface_open)(
                file.as_raw_fd(),
                size.0 as i32,
                size.1 as i32,
                format,
                modifiers.as_ptr(),
                modifiers.len() as i32,
                &mut descriptor,
            )
        })
        .ok_or("DMA-BUF allocation unavailable")?;
        let fds = descriptor.fd[..descriptor.planes as usize]
            .iter()
            .map(|fd| unsafe { OwnedFd::from_raw_fd(*fd) })
            .collect();
        Ok(Self {
            media: media.clone(),
            handle,
            descriptor,
            fds,
            device: device.to_owned(),
        })
    }
    /// 【显存帧】【平面借用】无参数，返回有效期内的 FD、偏移和行距，调用方不得关闭或提前归还缓冲
    pub fn planes(&self) -> impl Iterator<Item = (BorrowedFd<'_>, u32, u32)> {
        self.fds.iter().enumerate().map(|(index, fd)| {
            (
                fd.as_fd(),
                self.descriptor.offset[index],
                self.descriptor.stride[index],
            )
        })
    }
    /// 【显存帧】【修饰符读取】无参数，返回实际分配布局，不能把非线性布局当作普通内存读取
    pub fn modifier(&self) -> u64 {
        self.descriptor.modifier
    }
    /// 【显存帧】【格式读取】无参数，返回实际 DRM 四字符格式
    pub fn format(&self) -> u32 {
        self.descriptor.format
    }
    /// 【显存帧】【设备读取】无参数，返回分配设备路径，编码端使用同一设备导入
    pub fn device(&self) -> &Path {
        &self.device
    }
    /// 【显存帧】【尺寸读取】无参数，返回原始缓冲物理尺寸
    pub fn size(&self) -> (u32, u32) {
        (self.descriptor.width as u32, self.descriptor.height as u32)
    }
}
impl Encoder {
    /// 【显存帧】【同步编码】按 surface、origin 和 pts_us 完成裁剪及编码，返回前释放输入租约，不发生 CPU 像素回读
    pub fn dmabuf(
        &mut self,
        surface: &Surface,
        origin: (u32, u32),
        pts_us: i64,
    ) -> Result<(), String> {
        if origin.0 > 16384 || origin.1 > 16384 || pts_us < 0 {
            return Err("Invalid DMA-BUF crop or timestamp".into());
        }
        self.check(unsafe {
            (self.media.0.dmabuf)(
                self.handle.as_ptr(),
                &surface.descriptor,
                origin.0 as i32,
                origin.1 as i32,
                pts_us,
            )
        })
    }
}
impl Drop for Surface {
    /// 【显存帧】【资源销毁】无参数，先销毁原生缓冲，字段析构再关闭导出 FD 与媒体库
    fn drop(&mut self) {
        unsafe { (self.media.0.surface_destroy)(self.handle.as_ptr()) };
    }
}
