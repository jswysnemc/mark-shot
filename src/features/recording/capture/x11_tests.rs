//! 【X11录屏测试】【新连接光标】真实服务器验证独立采集连接的光标初始化和连续取帧

use super::{Source, x11::Capture};
use crate::{backend::x11::connection::Display, features::recording::Target};
use std::time::Duration;

/// 【X11录屏测试】【光标协议】无参数，在明确标记的隔离显示器上分别建立无光标及有光标来源，两种模式均能连续取帧
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated X11 server"]
fn fresh_connections_capture_multiple_frames_with_and_without_cursor() {
    assert_eq!(
        std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap(),
        std::env::var("DISPLAY").unwrap()
    );
    assert_eq!(std::env::var("XDG_SESSION_TYPE").unwrap(), "x11");
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_none());
    for cursor in [false, true] {
        let display = Display::connect().unwrap();
        let target = Target::default();
        let mut capture = Capture::new(display, &target, cursor).unwrap();
        for _ in 0..2 {
            assert!(capture.next(Duration::from_millis(100)).unwrap());
            let frame = capture.frame().unwrap();
            assert_eq!(frame.pixels.len(), frame.stride * frame.size.1 as usize);
            assert!(
                frame
                    .pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|pixel| pixel[3] == 255)
            );
        }
    }
}
