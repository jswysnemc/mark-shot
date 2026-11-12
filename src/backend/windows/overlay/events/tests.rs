//! 【Windows弹层测试】【点击归属】通过原生消息和覆盖层分发验证打开菜单的单击不会立即关闭菜单

use super::*;
use crate::{backend::ScreenOverlay, types::popup::PopupFrame};
use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP};

/// 【Windows弹层测试】【截图表面】无参数，返回具有隐藏父窗口的覆盖层，不捕获桌面像素
fn overlay() -> Overlay {
    let mut overlay = Overlay::new().unwrap();
    overlay.present(&[0]).unwrap();
    drain(&mut overlay);
    overlay.pending.clear();
    overlay
}

/// 【Windows弹层测试】【原生分派】按 overlay 消费窗口回调收集的全部消息，无返回值
fn drain(overlay: &mut Overlay) {
    while let Some(event) = overlay.queue.pop() {
        overlay.collect(event);
    }
}

/// 【Windows弹层测试】【鼠标消息】按 overlay、popup 与 pressed 向真实窗口发送左键消息，无返回值
fn click(overlay: &mut Overlay, popup: bool, pressed: bool) {
    let handle = if popup {
        overlay.popup.as_ref().unwrap().2.handle()
    } else {
        overlay.surfaces[0].1.handle()
    };
    unsafe {
        SendMessageW(
            handle as _,
            if pressed {
                WM_LBUTTONDOWN
            } else {
                WM_LBUTTONUP
            },
            usize::from(pressed),
            12 | (12 << 16),
        );
    }
    drain(overlay);
}

/// 【Windows弹层测试】【菜单呈现】按 overlay 和 id 创建真实下拉窗口，无返回值，测试使用独立桌面
fn open(overlay: &mut Overlay, id: u64) {
    overlay
        .set_popup(Some(PopupFrame {
            id,
            monitor_idx: 0,
            serial: None,
            anchor: (40.0, 40.0),
            size: (80, 60),
            scale: 1.0,
            pixels: &[255; 80 * 60 * 4],
            pixel_size: (80, 60),
        }))
        .unwrap();
    drain(overlay);
    overlay.pending.clear();
}

/// 【Windows弹层测试】【单击展开】父窗口按下后创建菜单，原按钮松开仍交给父窗口且菜单保持展开
#[test]
fn opening_click_release_keeps_dropdown_open() {
    let mut overlay = overlay();
    click(&mut overlay, false, true);
    open(&mut overlay, 41);
    click(&mut overlay, false, false);
    assert!(
        overlay.popup.is_some(),
        "opening release closed the dropdown"
    );
    assert!(
        overlay
            .pending
            .iter()
            .any(|event| matches!(event, OverlayEvent::PointerButton { pressed: false, .. }))
    );
    assert!(!overlay.pending.iter().any(|event| matches!(
        event,
        OverlayEvent::Popup {
            event: PopupEvent::Closed,
            ..
        }
    )));
}

/// 【Windows弹层测试】【外部关闭】展开后再次按下父窗口只关闭一次菜单，不把该次按下交给画布
#[test]
fn outside_press_closes_once_without_clicking_through() {
    let mut overlay = overlay();
    open(&mut overlay, 42);
    click(&mut overlay, false, true);
    click(&mut overlay, false, false);
    assert!(overlay.popup.is_none());
    assert_eq!(
        overlay
            .pending
            .iter()
            .filter(|event| matches!(
                event,
                OverlayEvent::Popup {
                    id: 42,
                    event: PopupEvent::Closed
                }
            ))
            .count(),
        1
    );
    assert!(
        !overlay
            .pending
            .iter()
            .any(|event| matches!(event, OverlayEvent::PointerButton { pressed: true, .. }))
    );
}

/// 【Windows弹层测试】【菜单选择】菜单内部的按下和松开完整交给原菜单身份，不提前销毁窗口
#[test]
fn popup_click_delivers_both_edges_to_the_menu() {
    let mut overlay = overlay();
    open(&mut overlay, 43);
    click(&mut overlay, true, true);
    click(&mut overlay, true, false);
    let buttons: Vec<_> = overlay
        .pending
        .iter()
        .filter_map(|event| match event {
            OverlayEvent::Popup {
                id: 43,
                event: PopupEvent::Button { pressed, .. },
            } => Some(*pressed),
            _ => None,
        })
        .collect();
    assert_eq!(buttons, [true, false]);
    assert!(overlay.popup.is_some());
    assert!(!overlay.pending.iter().any(|event| matches!(
        event,
        OverlayEvent::PointerButton { .. }
            | OverlayEvent::Popup {
                event: PopupEvent::Closed,
                ..
            }
    )));
}

/// 【Windows图片窗口测试】【窗口边框】真实 HWND 保留调整和最大化样式，客户区使用完整绘制尺寸并遵守工具栏最小宽度
#[test]
fn image_window_has_resizable_frame_and_exact_client_dimensions() {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let mut overlay = Overlay::new().unwrap();
    assert!(overlay.configure_image_window((700, 360), (640, 280)));
    overlay.present(&[0]).unwrap();
    drain(&mut overlay);
    let hwnd = overlay.surfaces[0].1.handle() as _;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        assert_eq!(
            style & (WS_THICKFRAME | WS_MAXIMIZEBOX | WS_MINIMIZEBOX),
            WS_THICKFRAME | WS_MAXIMIZEBOX | WS_MINIMIZEBOX
        );
        let mut client = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);
        assert_eq!(
            (client.right - client.left, client.bottom - client.top),
            (700, 360)
        );
        let mut limits: MINMAXINFO = std::mem::zeroed();
        SendMessageW(hwnd, WM_GETMINMAXINFO, 0, &mut limits as *mut _ as isize);
        assert_eq!(
            (limits.ptMinTrackSize.x, limits.ptMinTrackSize.y),
            (640, 280)
        );
        ShowWindow(hwnd, SW_SHOW);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            760,
            420,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        GetClientRect(hwnd, &mut client);
        assert_eq!(
            (client.right - client.left, client.bottom - client.top),
            (760, 420),
            "native client size"
        );
    }
    drain(&mut overlay);
    assert_eq!(overlay.image_window_size(), Some((760, 420)));
}
