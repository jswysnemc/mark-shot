//! 【Windows文字测试】【原生输入】在独立桌面验证真实窗口的输入上下文、重绘焦点和候选消息

use super::*;
use std::time::Duration;
use windows_sys::Win32::UI::{
    Input::{Ime::*, KeyboardAndMouse::GetFocus},
    WindowsAndMessaging::*,
};

/// 【Windows文字测试】【窗口准备】无参数，返回私有桌面的文字窗口和事件队列，不读取用户剪贴板
fn window() -> (Window, EventQueue) {
    assert_eq!(
        std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref(),
        Ok("1")
    );
    let queue = EventQueue::default();
    let mut window = Window::new(
        WindowOptions {
            title: "Mark Shot text input regression".into(),
            bounds: Bounds {
                left: 80,
                top: 80,
                width: 320,
                height: 180,
            },
            minimum: (1, 1),
            topmost: true,
            activate: true,
            layered: false,
            owner: 0,
        },
        queue.clone(),
    )
    .unwrap();
    window.set_rgba((320, 180), &[255; 320 * 180 * 4]).unwrap();
    dispatch(&[], Some(Duration::ZERO)).unwrap();
    while queue.pop().is_some() {}
    (window, queue)
}

/// 【Windows文字测试】【重绘稳定】无参数，验证开启文字后输入上下文有效且连续重绘不重新聚焦，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn text_input_keeps_its_context_and_focus_across_caret_frames() {
    let (mut window, queue) = window();
    let hwnd = window.handle() as _;
    window.set_text_input(41, Some((30, 40, 1, 24))).unwrap();
    let context = unsafe { ImmGetContext(hwnd) };
    assert!(!context.is_null(), "text tool has no Windows input context");
    unsafe {
        ImmReleaseContext(hwnd, context);
    }
    for _ in 0..12 {
        window.set_rgba((320, 180), &[255; 320 * 180 * 4]).unwrap();
        dispatch(&[], Some(Duration::ZERO)).unwrap();
        assert_eq!(unsafe { GetFocus() }, hwnd);
        while let Some(event) = queue.pop() {
            assert!(
                !matches!(event, Event::Focus { .. }),
                "caret frame changed input focus"
            );
        }
        let next = unsafe { ImmGetContext(hwnd) };
        assert_eq!(next, context, "caret frame replaced input context");
        unsafe {
            ImmReleaseContext(hwnd, next);
        }
    }
}

/// 【Windows文字测试】【默认上下文恢复】无参数，启用输入不依赖创建时保存的旧句柄，恢复系统默认输入上下文，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn enabling_text_recovers_the_default_input_context() {
    let (window, _) = window();
    let hwnd = window.handle() as _;
    assert!(unsafe { ImmGetContext(hwnd) }.is_null());
    super::ime::sync(hwnd, 0, 47, Some((30, 40, 1, 24))).unwrap();
    let context = unsafe { ImmGetContext(hwnd) };
    assert!(
        !context.is_null(),
        "enabling text left the default input context detached"
    );
    unsafe {
        ImmReleaseContext(hwnd, context);
    }
}

/// 【Windows文字测试】【候选光标更新】无参数，仅更新候选光标的原生消息仍需交付完整预编辑内容，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn cursor_only_composition_message_preserves_the_current_preedit() {
    let (mut window, queue) = window();
    let hwnd = window.handle() as _;
    window.set_text_input(42, Some((30, 40, 1, 24))).unwrap();
    let context = unsafe { ImmGetContext(hwnd) };
    assert!(!context.is_null());
    let units: Vec<u16> = "中文候选".encode_utf16().collect();
    assert_ne!(
        unsafe {
            ImmSetCompositionStringW(
                context,
                SCS_SETSTR,
                units.as_ptr().cast(),
                (units.len() * 2) as u32,
                std::ptr::null(),
                0,
            )
        },
        0,
        "private runtime cannot provide IMM composition strings"
    );
    while queue.pop().is_some() {}
    unsafe {
        SendMessageW(hwnd, WM_IME_COMPOSITION, 0, GCS_CURSORPOS as isize);
    }
    let update = queue.pop().expect("composition update was not delivered");
    assert!(matches!(update, Event::Composition { preedit, .. } if preedit == "中文候选"));
    unsafe {
        ImmReleaseContext(hwnd, context);
    }
}

/// 【Windows文字测试】【候选位置请求】无参数，实际输入法查询应返回文字光标屏幕位置与行高，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn input_method_can_query_the_text_caret_position() {
    let (mut window, _) = window();
    let hwnd = window.handle() as _;
    window.set_text_input(43, Some((30, 40, 1, 24))).unwrap();
    let mut query: IMECHARPOSITION = unsafe { std::mem::zeroed() };
    query.dwSize = std::mem::size_of::<IMECHARPOSITION>() as u32;
    assert_ne!(
        unsafe {
            SendMessageW(
                hwnd,
                WM_IME_REQUEST,
                IMR_QUERYCHARPOSITION as usize,
                &mut query as *mut _ as isize,
            )
        },
        0,
        "input method cannot query the custom text caret"
    );
    assert_eq!((query.pt.x, query.pt.y, query.cLineHeight), (110, 120, 24));
}

/// 【Windows文字测试】【组合字符查询】无参数，原生查询区分首字、非零 UTF-16 字符偏移与当前光标，代理对内部返回不支持，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn input_method_queries_the_requested_composition_character() {
    let (mut window, _) = window();
    let hwnd = window.handle() as _;
    window.set_text_input(46, Some((90, 40, 1, 24))).unwrap();
    let context = unsafe { ImmGetContext(hwnd) };
    let text: Vec<u16> = "中𠮷文".encode_utf16().collect();
    assert_ne!(
        unsafe {
            ImmSetCompositionStringW(
                context,
                SCS_SETSTR,
                text.as_ptr().cast(),
                (text.len() * 2) as u32,
                std::ptr::null(),
                0,
            )
        },
        0
    );
    let geometry = crate::types::text_input::CompositionGeometry {
        positions: vec![
            (0, (30, 40, 1, 24)),
            (1, (50, 40, 1, 24)),
            (3, (70, 40, 1, 24)),
            (4, (90, 40, 1, 24)),
        ],
    };
    window
        .set_text_input_with_geometry(46, Some((90, 40, 1, 24)), Some(&geometry))
        .unwrap();
    for (offset, (x, y, _, height)) in &geometry.positions {
        let mut query: IMECHARPOSITION = unsafe { std::mem::zeroed() };
        query.dwSize = std::mem::size_of::<IMECHARPOSITION>() as u32;
        query.dwCharPos = *offset;
        assert_ne!(
            unsafe {
                SendMessageW(
                    hwnd,
                    WM_IME_REQUEST,
                    IMR_QUERYCHARPOSITION as usize,
                    &mut query as *mut _ as isize,
                )
            },
            0
        );
        assert_eq!(
            (query.pt.x, query.pt.y, query.cLineHeight),
            (x + 80, y + 80, *height as u32)
        );
    }
    let mut query: IMECHARPOSITION = unsafe { std::mem::zeroed() };
    query.dwSize = std::mem::size_of::<IMECHARPOSITION>() as u32;
    query.dwCharPos = 2;
    assert_eq!(
        unsafe {
            SendMessageW(
                hwnd,
                WM_IME_REQUEST,
                IMR_QUERYCHARPOSITION as usize,
                &mut query as *mut _ as isize,
            )
        },
        0
    );
    unsafe {
        ImmReleaseContext(hwnd, context);
    }
}

/// 【Windows文字测试】【语言切换】无参数，原生中文键盘请求与输入法开启状态不因光标更新丢失，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn chinese_layout_and_open_status_survive_text_updates() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, LoadKeyboardLayoutW};
    let (mut window, _) = window();
    let hwnd = window.handle() as _;
    window.set_text_input(44, Some((30, 40, 1, 24))).unwrap();
    let name: Vec<u16> = "00000804".encode_utf16().chain([0]).collect();
    let chinese = unsafe { LoadKeyboardLayoutW(name.as_ptr(), 0) };
    assert!(
        !chinese.is_null(),
        "private runtime cannot load Chinese keyboard layout"
    );
    unsafe {
        SendMessageW(hwnd, WM_INPUTLANGCHANGEREQUEST, 0, chinese as isize);
    }
    assert_eq!(unsafe { GetKeyboardLayout(0) }, chinese);
    let context = unsafe { ImmGetContext(hwnd) };
    assert!(!context.is_null());
    assert_ne!(unsafe { ImmSetOpenStatus(context, 1) }, 0);
    for x in 30..42 {
        window.set_text_input(44, Some((x, 40, 1, 24))).unwrap();
        window.set_rgba((320, 180), &[255; 320 * 180 * 4]).unwrap();
        dispatch(&[], Some(Duration::ZERO)).unwrap();
        assert_ne!(unsafe { ImmGetOpenStatus(context) }, 0);
        assert_eq!(unsafe { GetKeyboardLayout(0) }, chinese);
    }
    unsafe {
        ImmReleaseContext(hwnd, context);
    }
}

/// 【Windows文字测试】【系统光标兼容】无参数，读取系统光标的中文输入法能获得自绘光标位置，系统不重复闪烁且停用时释放，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn custom_text_exposes_a_hidden_stable_system_caret() {
    let (mut window, _) = window();
    let hwnd = window.handle() as _;
    let expected = capture(window.bounds().unwrap(), false).unwrap();
    window.set_text_input(45, Some((30, 40, 1, 24))).unwrap();
    let mut info: GUITHREADINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    assert_ne!(unsafe { GetGUIThreadInfo(0, &mut info) }, 0);
    assert_eq!(
        info.hwndCaret, hwnd,
        "Chinese IMEs cannot find the custom text caret"
    );
    for x in 30..42 {
        window.set_text_input(45, Some((x, 40, 1, 24))).unwrap();
        dispatch(&[], Some(Duration::from_millis(75))).unwrap();
        assert!(
            capture(window.bounds().unwrap(), false).unwrap() == expected,
            "system caret changed the custom surface pixels"
        );
        let mut point = unsafe { std::mem::zeroed() };
        assert_ne!(unsafe { GetCaretPos(&mut point) }, 0);
        assert_eq!((point.x, point.y), (x, 40));
    }
    window.set_text_input(0, None).unwrap();
    assert_ne!(unsafe { GetGUIThreadInfo(0, &mut info) }, 0);
    assert_ne!(info.hwndCaret, hwnd);
}

thread_local! {
    static PREVIOUS_PROCEDURE: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
    static REENTRANT_END_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// 【Windows文字测试】【同步回调】按原生消息参数在 IMM 定位通知内模拟旧候选结束；返回原回调结果，只在本次窗口线程调用
unsafe extern "system" fn reentrant_end(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == WM_IME_NOTIFY && wparam == IMN_SETCANDIDATEPOS as usize {
        REENTRANT_END_COUNT.with(|count| count.set(count.get() + 1));
        unsafe {
            SendMessageW(hwnd, WM_IME_ENDCOMPOSITION, 0, 0);
        }
    }
    let original = PREVIOUS_PROCEDURE.with(|procedure| procedure.get());
    let procedure = unsafe { std::mem::transmute::<isize, WNDPROC>(original) };
    unsafe { CallWindowProcW(procedure, hwnd, message, wparam, lparam) }
}

/// 【Windows文字测试】【同步取消隔离】无参数，在真实 IMM 同步栈内回送结束消息，旧候选不能交付新字段，后续新候选正常交付，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn synchronous_old_composition_end_does_not_clear_the_new_field() {
    let (mut window, queue) = window();
    let hwnd = window.handle() as _;
    window.set_text_input(48, Some((30, 40, 1, 24))).unwrap();
    while queue.pop().is_some() {}
    let original =
        unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, reentrant_end as *const () as isize) };
    assert_ne!(original, 0);
    PREVIOUS_PROCEDURE.with(|procedure| procedure.set(original));
    REENTRANT_END_COUNT.with(|count| count.set(0));
    let result = window.set_text_input(49, Some((50, 40, 1, 24)));
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, original);
    }
    result.unwrap();
    assert!(
        REENTRANT_END_COUNT.with(|count| count.get()) > 0,
        "IMM did not reenter the window callback"
    );
    while let Some(event) = queue.pop() {
        assert!(
            !matches!(event, Event::Composition { .. }),
            "old synchronous candidate end reached the new field"
        );
    }
    unsafe {
        SendMessageW(hwnd, WM_IME_STARTCOMPOSITION, 0, 0);
    }
    assert!(
        matches!(queue.pop(), Some(Event::Composition { target: 49, .. })),
        "new field composition was suppressed after synchronization"
    );
}
