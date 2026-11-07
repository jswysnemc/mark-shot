//! 【Windows捕获】【原生像素】从屏幕复制目标区域，可选合成原生光标，临时设备上下文在本次调用结束时释放

use super::{Bitmap, Bounds};
use std::io;
use windows_sys::Win32::{
    Graphics::Gdi::{BitBlt, CAPTUREBLT, DeleteObject, GetDC, HDC, ReleaseDC, SRCCOPY},
    UI::WindowsAndMessaging::{
        CURSOR_SHOWING, CURSORINFO, DI_NORMAL, DrawIconEx, GetCursorInfo, GetIconInfo, ICONINFO,
    },
};

struct DesktopDc(HDC);
impl Drop for DesktopDc {
    /// 【Windows捕获】【桌面上下文释放】无参数，归还本次借用的桌面设备上下文，不销毁系统桌面，无返回值
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(std::ptr::null_mut(), self.0);
        }
    }
}

/// 【Windows捕获】【光标合成】按bounds将当前原生光标绘入bitmap，返回系统结果，不可见或没有光标时保持原图
fn cursor(bitmap: &Bitmap, bounds: Bounds) -> io::Result<()> {
    let mut cursor: CURSORINFO = unsafe { std::mem::zeroed() };
    cursor.cbSize = size_of::<CURSORINFO>() as u32;
    if unsafe { GetCursorInfo(&mut cursor) } == 0 {
        return Err(super::last_error());
    }
    if cursor.flags & CURSOR_SHOWING == 0 || cursor.hCursor.is_null() {
        return Ok(());
    }
    let mut icon: ICONINFO = unsafe { std::mem::zeroed() };
    if unsafe { GetIconInfo(cursor.hCursor, &mut icon) } == 0 {
        return Err(super::last_error());
    }
    let result = unsafe {
        DrawIconEx(
            bitmap.dc(),
            cursor.ptScreenPos.x - bounds.left - icon.xHotspot as i32,
            cursor.ptScreenPos.y - bounds.top - icon.yHotspot as i32,
            cursor.hCursor,
            0,
            0,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        )
    };
    // 1. 【Windows捕获】【光标副本回收】系统光标保持借用，GetIconInfo创建的颜色与遮罩副本分别销毁
    unsafe {
        if !icon.hbmColor.is_null() {
            DeleteObject(icon.hbmColor);
        }
        if !icon.hbmMask.is_null() {
            DeleteObject(icon.hbmMask);
        }
    }
    if result == 0 {
        Err(super::last_error())
    } else {
        Ok(())
    }
}

/// 【Windows捕获】【区域读取】按bounds和include_cursor返回紧密BGRA屏幕像素，所有背景像素不透明；本次位图和DC随后释放
pub fn capture(bounds: Bounds, include_cursor: bool) -> io::Result<Vec<u8>> {
    let mut bitmap = Bitmap::new((bounds.width, bounds.height))?;
    let desktop = DesktopDc(unsafe { GetDC(std::ptr::null_mut()) });
    if desktop.0.is_null() {
        return Err(super::last_error());
    }
    if unsafe {
        BitBlt(
            bitmap.dc(),
            0,
            0,
            bounds.width as i32,
            bounds.height as i32,
            desktop.0,
            bounds.left,
            bounds.top,
            SRCCOPY | CAPTUREBLT,
        )
    } == 0
    {
        return Err(super::last_error());
    }
    if include_cursor {
        cursor(&bitmap, bounds)?;
    }
    for pixel in bitmap.pixels_mut().as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    Ok(bitmap.pixels().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Graphics::Gdi::GdiFlush;

    /// 【Windows捕获测试】【私有像素对照】无参数，只在明确私有Wine/Xvfb中绘制原生样本，捕获完整区域必须逐字节相同，无返回值
    #[test]
    #[ignore = "requires the private Windows test runtime"]
    fn native_screen_capture_preserves_region_origin_rows_and_exact_pixels() {
        assert_eq!(
            std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref(),
            Ok("1")
        );
        let displays = super::super::monitors().unwrap();
        assert!(!displays.is_empty());
        let bounds = Bounds {
            left: displays[0].bounds.left + 64,
            top: displays[0].bounds.top + 96,
            width: 32,
            height: 24,
        };
        let mut source = Bitmap::new((bounds.width, bounds.height)).unwrap();
        for (index, pixel) in source
            .pixels_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .enumerate()
        {
            let x = index % bounds.width as usize;
            let y = index / bounds.width as usize;
            pixel.copy_from_slice(&[(x * 7) as u8, (y * 9) as u8, ((x + y) * 3) as u8, 255]);
        }
        let expected = source.pixels().to_vec();
        let desktop = DesktopDc(unsafe { GetDC(std::ptr::null_mut()) });
        assert!(!desktop.0.is_null());
        assert_ne!(
            unsafe {
                BitBlt(
                    desktop.0,
                    bounds.left,
                    bounds.top,
                    bounds.width as i32,
                    bounds.height as i32,
                    source.dc(),
                    0,
                    0,
                    SRCCOPY,
                )
            },
            0
        );
        unsafe {
            GdiFlush();
        }
        let actual = capture(bounds, false).unwrap();
        assert!(
            actual == expected,
            "native region capture changed the painted sample pixels"
        );
        assert_eq!(capture(bounds, true).unwrap().len(), expected.len());
    }
}
