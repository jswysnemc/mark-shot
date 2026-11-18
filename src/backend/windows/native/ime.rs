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
        // 1. 【Windows输入法】【增量候选】光标、属性和分句消息只表示变化字段，读取当前完整候选避免误清空正文
        let preedit = if flags
            & (GCS_COMPSTR | GCS_CURSORPOS | GCS_COMPATTR | GCS_COMPCLAUSE) as isize
            != 0
        {
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

/// 【Windows输入法】【目标同步】按 hwnd、previous 和 target 切换默认输入上下文，rect 为物理光标范围，返回系统结果，不跨线程使用上下文
pub(super) fn sync(
    hwnd: HWND,
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
        if target == 0 {
            unsafe {
                ImmAssociateContext(hwnd, std::ptr::null_mut());
            }
        } else {
            // 2. 【Windows输入法】【默认恢复】与 Qt 保持一致，恢复线程默认上下文，不依赖创建时取得的句柄
            if unsafe { ImmAssociateContextEx(hwnd, std::ptr::null_mut(), IACE_DEFAULT) } == 0 {
                return Err(io::Error::other(
                    "cannot associate the default Windows input context",
                ));
            }
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
        // 3. 【Windows输入法】【中文兼容】部分中文输入法采用候选点定位，随后仍保留完整光标排除区域
        let language =
            windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout(0) as usize & 0x3ff;
        if language == 0x04 {
            let mut point = candidate;
            point.dwStyle = CFS_CANDIDATEPOS;
            ImmSetCandidateWindow(context, &point);
        }
        ImmSetCandidateWindow(context, &candidate);
        ImmReleaseContext(hwnd, context);
    }
    Ok(())
}

/// 【Windows输入法】【光标查询】按 hwnd、rect、geometry 和系统请求指针返回处理结果；组合字符使用真实几何，不猜测字宽
pub(super) unsafe fn query_position(
    hwnd: HWND,
    rect: Option<(i32, i32, i32, i32)>,
    geometry: Option<&crate::types::text_input::CompositionGeometry>,
    request: isize,
) -> isize {
    use windows_sys::Win32::{
        Foundation::POINT, Graphics::Gdi::ClientToScreen, UI::WindowsAndMessaging::GetClientRect,
    };
    if request == 0 {
        return 0;
    }
    // 1. 【Windows输入法】【系统请求】消息指针由 IMM 在窗口线程借用，先核对结构尺寸及可定位字符
    let query = unsafe { &mut *(request as *mut IMECHARPOSITION) };
    if query.dwSize < std::mem::size_of::<IMECHARPOSITION>() as u32 {
        return 0;
    }
    let position = match geometry {
        Some(geometry) => geometry.at(query.dwCharPos),
        None if query.dwCharPos == 0 => rect,
        None => None,
    };
    let Some((x, y, _, height)) = position else {
        return 0;
    };
    let mut caret = POINT { x, y };
    let mut document = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut document);
    }
    let mut top = POINT {
        x: document.left,
        y: document.top,
    };
    let mut bottom = POINT {
        x: document.right,
        y: document.bottom,
    };
    // 2. 【Windows输入法】【屏幕坐标】光标与客户区范围统一转换为物理屏幕坐标，候选位置不受窗口边框偏移影响
    if unsafe { ClientToScreen(hwnd, &mut caret) } == 0
        || unsafe { ClientToScreen(hwnd, &mut top) } == 0
        || unsafe { ClientToScreen(hwnd, &mut bottom) } == 0
    {
        return 0;
    }
    query.pt = caret;
    query.cLineHeight = height.max(1) as u32;
    query.rcDocument = windows_sys::Win32::Foundation::RECT {
        left: top.x,
        top: top.y,
        right: bottom.x,
        bottom: bottom.y,
    };
    1
}
