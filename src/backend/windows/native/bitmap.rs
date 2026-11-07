//! 【Windows位图】【资源所有权】固定尺寸顶向位图只持有一个原生缓冲，释放前恢复设备上下文原对象

use std::{io, ptr::NonNull};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject,
};

pub struct Bitmap {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: NonNull<u8>,
    size: (u32, u32),
    bytes: usize,
}

impl Bitmap {
    /// 【Windows位图】【尺寸分配】按size创建一个BGRA顶向位图，返回独占资源，拒绝溢出或超过512MiB的分配
    pub fn new(size: (u32, u32)) -> io::Result<Self> {
        let bytes = (size.0 as usize)
            .checked_mul(size.1 as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes > 0 && *bytes <= 512 * 1024 * 1024)
            .ok_or_else(|| io::Error::other("invalid Windows bitmap dimensions"))?;
        if size.0 > i32::MAX as u32 || size.1 > i32::MAX as u32 {
            return Err(io::Error::other(
                "Windows bitmap dimensions exceed GDI limits",
            ));
        }
        let dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        if dc.is_null() {
            return Err(super::last_error());
        }
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size.0 as i32,
            biHeight: -(size.1 as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..unsafe { std::mem::zeroed() }
        };
        let mut pointer = std::ptr::null_mut();
        let bitmap = unsafe {
            CreateDIBSection(
                dc,
                &info,
                DIB_RGB_COLORS,
                &mut pointer,
                std::ptr::null_mut(),
                0,
            )
        };
        if bitmap.is_null() || pointer.is_null() {
            let error = super::last_error();
            unsafe {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                DeleteDC(dc);
            }
            return Err(error);
        }
        let previous = unsafe { SelectObject(dc, bitmap) };
        if previous.is_null() || previous as isize == -1 {
            let error = super::last_error();
            unsafe {
                DeleteObject(bitmap);
                DeleteDC(dc);
            }
            return Err(error);
        }
        Ok(Self {
            dc,
            bitmap,
            previous,
            pixels: NonNull::new(pointer.cast()).unwrap(),
            size,
            bytes,
        })
    }

    /// 【Windows位图】【尺寸读取】无参数，返回位图物理像素尺寸，不复制内容
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// 【Windows位图】【设备引用】无参数，返回当前位图设备上下文，调用方不得关闭或替换其中对象
    pub(super) fn dc(&self) -> HDC {
        self.dc
    }

    /// 【Windows位图】【像素借用】无参数，返回拥有期内的BGRA字节，系统保证四字节行对齐
    pub fn pixels(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.pixels.as_ptr(), self.bytes) }
    }

    /// 【Windows位图】【像素修改】无参数，返回独占BGRA字节，借用结束后才允许调用提交或系统绘制
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.pixels.as_ptr(), self.bytes) }
    }

    /// 【Windows位图】【界面复制】把同尺寸rgba转换为原生BGRA，保留预乘透明度，返回尺寸校验结果，不产生完整临时副本
    pub fn write_rgba(&mut self, rgba: &[u8]) -> io::Result<()> {
        self.write_rgba_region(rgba, None)
    }

    /// 【Windows位图】【局部复制】把rgba中damage范围转为BGRA，空范围表示完整帧，返回尺寸校验结果并保留范围外内容
    pub fn write_rgba_region(
        &mut self,
        rgba: &[u8],
        damage: Option<(u32, u32, u32, u32)>,
    ) -> io::Result<()> {
        if rgba.len() != self.bytes {
            return Err(io::Error::other(
                "Windows frame size changed without resizing its bitmap",
            ));
        }
        let (width, height) = self.size;
        let (x, y, w, h) = damage.unwrap_or((0, 0, width, height));
        let right = x.saturating_add(w).min(width);
        let bottom = y.saturating_add(h).min(height);
        if x >= right || y >= bottom {
            return Ok(());
        }
        let stride = width as usize * 4;
        for row in y..bottom {
            let start = row as usize * stride + x as usize * 4;
            let end = row as usize * stride + right as usize * 4;
            for (target, source) in self.pixels_mut()[start..end]
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(rgba[start..end].as_chunks::<4>().0)
            {
                target.copy_from_slice(&[source[2], source[1], source[0], source[3]]);
            }
        }
        Ok(())
    }
}

impl Drop for Bitmap {
    /// 【Windows位图】【完整释放】无参数，先恢复原选中对象，再销毁位图和设备上下文，无返回值
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【Windows位图测试】【通道与行序】无参数，实际DIB保持顶向行序、预乘透明度和BGRA通道，完整缓冲没有重复副本，无返回值
    #[test]
    fn native_bitmap_preserves_rows_channels_and_alpha() {
        let mut bitmap = Bitmap::new((2, 2)).unwrap();
        bitmap
            .write_rgba(&[1, 2, 3, 255, 20, 30, 40, 128, 80, 90, 100, 255, 0, 0, 0, 0])
            .unwrap();
        assert_eq!(
            bitmap.pixels(),
            [3, 2, 1, 255, 40, 30, 20, 128, 100, 90, 80, 255, 0, 0, 0, 0]
        );
        assert!(bitmap.write_rgba(&[1, 2, 3]).is_err());
        assert_eq!(bitmap.size(), (2, 2));
    }

    /// 【Windows位图测试】【尺寸边界】无参数，错误尺寸在调用系统分配前失败，防止溢出和不受控大图，无返回值
    #[test]
    fn invalid_native_bitmap_sizes_are_rejected() {
        for size in [(0, 1), (1, 0), (u32::MAX, 1), (u32::MAX, u32::MAX)] {
            assert!(Bitmap::new(size).is_err());
        }
    }
}
