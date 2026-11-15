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
