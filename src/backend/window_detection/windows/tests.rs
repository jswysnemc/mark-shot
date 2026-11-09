//! 【Windows控件测试】【真实控件】在隔离桌面创建Win32控件，验证UIA物理边界、缓存及覆盖层排除

use super::automation::Automation;
use crate::features::region_detection::PixelRect;
use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::*};

/// 【Windows控件测试】【窗口字符串】按text返回以零结尾UTF16字符串，供同步Win32调用使用
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// 【Windows控件测试】【原生矩形】按handle读取物理边界，返回可与UIA直接比较的矩形
fn rectangle(handle: windows_sys::Win32::Foundation::HWND) -> PixelRect {
    let mut rect = RECT::default();
    assert_ne!(unsafe { GetWindowRect(handle, &mut rect) }, 0);
    PixelRect::new(
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    )
}

/// 【Windows控件测试】【缓存命中】无参数，在所属隔离桌面验证覆盖层下的按钮物理边界，重复命中不重复访问外部控件
#[test]
#[ignore = "requires the private Windows test runtime or Windows CI desktop"]
fn native_controls_under_overlay_keep_exact_bounds_and_reuse_cache() {
    assert!(
        std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref() == Ok("1")
            || std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
    );
    crate::backend::windows::native::enable_dpi().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || unsafe {
        let parent = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Control fixture").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            40,
            60,
            480,
            320,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        assert!(!parent.is_null());
        let button = CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide("Select control").as_ptr(),
            WS_CHILD | WS_VISIBLE,
            55,
            65,
            160,
            40,
            parent,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        assert!(!button.is_null());
        let other = CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide("Other control").as_ptr(),
            WS_CHILD | WS_VISIBLE,
            55,
            130,
            180,
            40,
            parent,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        assert!(!other.is_null());
        let overlay = CreateWindowExW(
            WS_EX_TOPMOST,
            wide("STATIC").as_ptr(),
            wide("Overlay fixture").as_ptr(),
            WS_POPUP | WS_VISIBLE,
            30,
            40,
            600,
            420,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        sender
            .send((
                windows_sys::Win32::System::Threading::GetCurrentThreadId(),
                parent as usize,
                rectangle(parent),
                rectangle(button),
                rectangle(other),
            ))
            .unwrap();
        let mut message = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        DestroyWindow(overlay);
        DestroyWindow(parent);
    });
    let (thread_id, handle, bounds, button, other) = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let result = std::panic::catch_unwind(|| {
        let mut automation = Automation::new().expect("UI Automation initialization");
        for rect in [button, other] {
            let point = (rect.left + 8, rect.top + 8);
            let mut path = Vec::new();
            automation.query(handle, bounds, point, || false, |result| path = result);
            assert_eq!(
                path.first(),
                Some(&rect),
                "UIA did not resolve the actual button under the overlay"
            );
            let calls = automation.calls;
            let started = std::time::Instant::now();
            for _ in 0..100 {
                automation.query(handle, bounds, point, || false, |result| path = result);
            }
            assert_eq!(
                automation.calls, calls,
                "cached hover repeated COM acquisition"
            );
            assert_eq!(path.first(), Some(&rect));
            println!(
                "【Windows控件测试】【缓存性能】100次查询耗时 {:?}",
                started.elapsed()
            );
        }
    });
    unsafe {
        PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
    }
    thread.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
