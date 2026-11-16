//! 【X11覆盖层测试】【滚轮方向】通过真实表面与协议分派核对截图和图片编辑的刻度

use super::*;
use crate::backend::ScreenOverlay;
use x11rb::protocol::{Event, xproto::*};

/// 【X11覆盖层测试】【上下契约】无参数，在明确标记的隔离 X11 校验按钮 4 向上、按钮 5 向下，松开不重复产生滚动
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated X11 server"]
fn wheel_buttons_keep_up_positive_in_capture_and_image_editor() {
    let name = std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap();
    assert_eq!(std::env::var("DISPLAY").unwrap(), name);
    assert_eq!(std::env::var("XDG_SESSION_TYPE").unwrap(), "x11");
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_none());
    for image_editor in [false, true] {
        let display = Display::connect().unwrap();
        let mut overlay = Overlay::new(display.clone()).unwrap();
        if image_editor {
            overlay.configure_image_window((600, 340), (320, 240));
        }
        overlay.present(&[0]).unwrap();
        let window = overlay.surfaces[0].1.window;
        for (button, expected) in [(4, 120.0), (5, -120.0)] {
            overlay.pending.clear();
            let event = ButtonPressEvent {
                response_type: BUTTON_PRESS_EVENT,
                detail: button,
                event: window,
                root: display.screen().root,
                event_x: 80,
                event_y: 100,
                same_screen: true,
                ..Default::default()
            };
            assert!(matches!(
                overlay.translate(Event::ButtonPress(event)).unwrap(),
                Some(OverlayEvent::PointerMove { .. })
            ));
            let Some(OverlayEvent::Scroll(scroll)) = overlay.pending.pop_front() else {
                panic!("wheel press did not deliver a scroll event");
            };
            assert_eq!(scroll.angle_y(), expected, "image_editor={image_editor}");
            assert_eq!(scroll.legacy_y(), expected as f32 / 12.0);
            assert!(overlay.pending.is_empty());
            overlay
                .translate(Event::ButtonRelease(ButtonReleaseEvent {
                    response_type: BUTTON_RELEASE_EVENT,
                    ..event
                }))
                .unwrap();
            assert!(overlay.pending.is_empty());
        }
    }
}

/// 【X11跟随测试】【连续位置】无参数，向真实截图窗口发送连续移动，要求一次分派采用最新位置并保留轨迹，无返回值
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated X11 server"]
fn queued_motion_uses_latest_position_without_losing_the_path() {
    let name = std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap();
    assert_eq!(std::env::var("DISPLAY").unwrap(), name);
    let display = Display::connect().unwrap();
    let mut overlay = Overlay::new(display.clone()).unwrap();
    overlay.present(&[0]).unwrap();
    while !matches!(
        overlay.next_event(Some(std::time::Duration::ZERO)).unwrap(),
        OverlayEvent::Tick
    ) {}
    let window = overlay.surfaces[0].1.window;
    // 1. 【X11跟随测试】【事件积压】模拟绘制期间已经到达的鼠标采样，服务器往返确保全部事件可读取
    for x in 100..132 {
        display
            .connection
            .send_event(
                false,
                window,
                EventMask::POINTER_MOTION,
                MotionNotifyEvent {
                    response_type: MOTION_NOTIFY_EVENT,
                    event: window,
                    root: display.screen().root,
                    event_x: x,
                    event_y: 100,
                    same_screen: true,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    display
        .connection
        .get_input_focus()
        .unwrap()
        .reply()
        .unwrap();
    let OverlayEvent::PointerMove { x, history, .. } = overlay
        .next_event(Some(std::time::Duration::from_secs(1)))
        .unwrap()
    else {
        panic!("queued motion was not delivered");
    };
    assert_eq!(x, 131.0, "window selection is behind the current pointer");
    assert_eq!(history.len(), 32);
}

/// 【X11跟随测试】【按钮边界】无参数，在私有服务器发送按下、移动、松开，要求起点和按钮分派顺序保持原样，无返回值
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated X11 server"]
fn queued_motion_preserves_press_and_release_coordinates() {
    assert_eq!(
        std::env::var("DISPLAY").unwrap(),
        std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap()
    );
    let display = Display::connect().unwrap();
    let mut overlay = Overlay::new(display.clone()).unwrap();
    overlay.present(&[0]).unwrap();
    while !matches!(
        overlay.next_event(Some(std::time::Duration::ZERO)).unwrap(),
        OverlayEvent::Tick
    ) {}
    let window = overlay.surfaces[0].1.window;
    let button = ButtonPressEvent {
        response_type: BUTTON_PRESS_EVENT,
        detail: 1,
        event: window,
        root: display.screen().root,
        event_x: 100,
        event_y: 100,
        same_screen: true,
        ..Default::default()
    };
    display
        .connection
        .send_event(false, window, EventMask::BUTTON_PRESS, button)
        .unwrap();
    display
        .connection
        .send_event(
            false,
            window,
            EventMask::POINTER_MOTION,
            MotionNotifyEvent {
                response_type: MOTION_NOTIFY_EVENT,
                event: window,
                root: display.screen().root,
                event_x: 150,
                event_y: 100,
                same_screen: true,
                ..Default::default()
            },
        )
        .unwrap();
    display
        .connection
        .send_event(
            false,
            window,
            EventMask::BUTTON_RELEASE,
            ButtonReleaseEvent {
                response_type: BUTTON_RELEASE_EVENT,
                event_x: 150,
                ..button
            },
        )
        .unwrap();
    display
        .connection
        .get_input_focus()
        .unwrap()
        .reply()
        .unwrap();
    for (x, pressed) in [
        (Some(100.0), None),
        (None, Some(true)),
        (Some(150.0), None),
        (Some(150.0), None),
        (None, Some(false)),
    ] {
        let event = overlay
            .next_event(Some(std::time::Duration::from_secs(1)))
            .unwrap();
        match event {
            OverlayEvent::PointerMove { x: actual, .. } => assert_eq!(Some(actual), x),
            OverlayEvent::PointerButton {
                pressed: actual, ..
            } => assert_eq!(Some(actual), pressed),
            _ => panic!("unexpected event: {event:?}"),
        }
    }
}
