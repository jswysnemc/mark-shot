//! 【Windows输入法】【候选读取】有界读取UTF16候选与确认，候选位置按客户区物理光标同步

use std::io;
use windows_sys::Win32::{Foundation::HWND, UI::Input::Ime::*};

pub(super) struct Composition {
    pub preedit: String,
    pub cursor: usize,
    pub commit: String,
}

/// 【Windows输入法】【字符串读取】按context与kind读取至多六十四KiB的UTF16，返回完整字符，不接受奇数字节或无效代理对
fn string(context: HIMC, kind: u32) -> io::Result<Vec<u16>> {
    let count = unsafe { ImmGetCompositionStringW(context, kind, std::ptr::null_mut(), 0) };
    if !(0..=64 * 1024).contains(&count) || count % 2 != 0 {
        return Err(io::Error::other("invalid Windows composition length"));
    }
    let mut units = vec![0_u16; count as usize / 2];
    if count != 0
        && unsafe {
            ImmGetCompositionStringW(context, kind, units.as_mut_ptr().cast(), count as u32)
        } != count
    {
        return Err(io::Error::other(
            "Windows composition changed while reading",
        ));
    }
    Ok(units)
}

/// 【Windows输入法】【确认分离】按hwnd与flags返回候选、UTF8光标与确认正文，句柄只在本次调用期间借用
pub(super) fn read(hwnd: HWND, flags: isize) -> io::Result<Composition> {
    let context = unsafe { ImmGetContext(hwnd) };
    if context.is_null() {
        return Ok(Composition {
            preedit: String::new(),
            cursor: 0,
            commit: String::new(),
        });
    }
    let result = (|| {
        let preedit = if flags & GCS_COMPSTR as isize != 0 {
            string(context, GCS_COMPSTR)?
        } else {
            Vec::new()
        };
        let offset =
            unsafe { ImmGetCompositionStringW(context, GCS_CURSORPOS, std::ptr::null_mut(), 0) }
                .max(0) as usize;
        let mut end = offset.min(preedit.len());
        if end > 0 && (0xd800..=0xdbff).contains(&preedit[end - 1]) {
            end -= 1;
        }
        let cursor = String::from_utf16(&preedit[..end])
            .map_err(io::Error::other)?
            .len();
        let commit = if flags & GCS_RESULTSTR as isize != 0 {
            String::from_utf16(&string(context, GCS_RESULTSTR)?).map_err(io::Error::other)?
        } else {
            String::new()
        };
        Ok(Composition {
            preedit: String::from_utf16(&preedit).map_err(io::Error::other)?,
            cursor,
            commit,
        })
    })();
    unsafe {
        ImmReleaseContext(hwnd, context);
    }
    result
}

/// 【Windows输入法】【目标同步】按hwnd、saved、previous与target切换候选资格，rect为物理光标范围，返回系统结果，不跨线程使用输入上下文
pub(super) fn sync(
    hwnd: HWND,
    saved: HIMC,
    previous: u64,
    target: u64,
    rect: Option<(i32, i32, i32, i32)>,
) -> io::Result<()> {
    if previous != target {
        if previous != 0 {
            let context = unsafe { ImmGetContext(hwnd) };
            if !context.is_null() {
                unsafe {
                    ImmNotifyIME(context, NI_COMPOSITIONSTR, CPS_CANCEL, 0);
                    ImmReleaseContext(hwnd, context);
                }
            }
        }
        unsafe {
            ImmAssociateContext(
                hwnd,
                if target == 0 {
                    std::ptr::null_mut()
                } else {
                    saved
                },
            );
        }
    }
    let Some((x, y, width, height)) = rect.filter(|_| target != 0) else {
        return Ok(());
    };
    let context = unsafe { ImmGetContext(hwnd) };
    if context.is_null() {
        return Ok(());
    }
    let mut composition: COMPOSITIONFORM = unsafe { std::mem::zeroed() };
    composition.dwStyle = CFS_POINT;
    composition.ptCurrentPos.x = x;
    composition.ptCurrentPos.y = y.saturating_add(height);
    let mut candidate: CANDIDATEFORM = unsafe { std::mem::zeroed() };
    candidate.dwStyle = CFS_EXCLUDE;
    candidate.ptCurrentPos = composition.ptCurrentPos;
    candidate.rcArea = windows_sys::Win32::Foundation::RECT {
        left: x,
        top: y,
        right: x.saturating_add(width),
        bottom: y.saturating_add(height),
    };
    unsafe {
        ImmSetCompositionWindow(context, &composition);
        ImmSetCandidateWindow(context, &candidate);
        ImmReleaseContext(hwnd, context);
    }
    Ok(())
}
