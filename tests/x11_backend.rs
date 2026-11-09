//! 【X11集成测试】【私有显示】在显式指定的独立显示器验证持续捕获与滚动面板，不连接用户桌面

#![cfg(target_os = "linux")]

use mark_shot::{
    backend::{self, CaptureMethod, ScreenOverlay, session::Session, x11},
    types::OverlayEvent,
};
use std::time::Duration;
use x11rb::{connection::Connection, protocol::xproto::*, wrapper::ConnectionExt as _};

/// 【X11集成测试】【会话校验】无参数，只连接测试显式标记的X11显示器，返回私有连接
fn private_display() -> x11::connection::Display {
    let display =
        std::env::var("MARK_SHOT_TEST_X11_DISPLAY").expect("private X11 test marker required");
    assert_eq!(std::env::var("DISPLAY").unwrap(), display);
    assert_eq!(std::env::var("XDG_SESSION_TYPE").unwrap(), "x11");
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_none());
    x11::connection::Display::connect().unwrap()
}

/// 【X11集成测试】【连续像素】无参数，修改真实根窗口后验证同一捕获会话返回变化帧，并检查输出身份
#[tokio::test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated Xvfb"]
async fn continuous_capture_reads_new_pixels_and_keeps_output_identity() {
    let display = private_display();
    let outputs = display.outputs().unwrap();
    let method = x11::capture::Method {
        display: display.clone(),
        include_cursor: false,
    };
    let mut session = method.open_session(&outputs).await.unwrap();
    let first = session.next_frame().await.unwrap();
    let gc = display.connection.generate_id().unwrap();
    display
        .connection
        .create_gc(
            gc,
            display.screen().root,
            &CreateGCAux::new().foreground(0xc52a71),
        )
        .unwrap()
        .check()
        .unwrap();
    display
        .connection
        .poly_fill_rectangle(
            display.screen().root,
            gc,
            &[Rectangle {
                x: 600,
                y: 500,
                width: 80,
                height: 70,
            }],
        )
        .unwrap()
        .check()
        .unwrap();
    display.connection.free_gc(gc).unwrap().check().unwrap();
    let second = session.next_frame().await.unwrap();
    assert_eq!(first.frames.len(), outputs.len());
    assert_eq!(second.frames[0].output, 0);
    assert_eq!(
        second.frames[0].info.position,
        outputs[0].info.logical_position
    );
    assert_ne!(first.frames[0].pixels, second.frames[0].pixels);
    session.close().await.unwrap();
}

/// 【X11集成测试】【滚动隔离】无参数，验证覆盖层释放焦点、局部面板不污染选区，重叠时拒绝并隐藏
#[tokio::test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated Xvfb"]
async fn scroll_panel_releases_focus_and_rejects_covered_frames() {
    let display = private_display();
    let client = display.connection.generate_id().unwrap();
    display
        .connection
        .create_window(
            display.screen().root_depth,
            client,
            display.screen().root,
            20,
            20,
            300,
            220,
            0,
            WindowClass::INPUT_OUTPUT,
            display.screen().root_visual,
            &CreateWindowAux::new().background_pixel(0x214f81),
        )
        .unwrap()
        .check()
        .unwrap();
    display
        .connection
        .map_window(client)
        .unwrap()
        .check()
        .unwrap();
    display
        .connection
        .set_input_focus(InputFocus::POINTER_ROOT, client, x11rb::CURRENT_TIME)
        .unwrap()
        .check()
        .unwrap();
    display
        .connection
        .change_property32(
            PropMode::REPLACE,
            display.screen().root,
            display.atom("_NET_ACTIVE_WINDOW").unwrap(),
            AtomEnum::WINDOW,
            &[client],
        )
        .unwrap()
        .check()
        .unwrap();
    let mut overlay = x11::overlay::Overlay::new(display.clone()).unwrap();
    let output = overlay.discovered_outputs()[0].clone();
    overlay.set_composited_background(true);
    overlay.present(&[0]).unwrap();
    let size = output.info.logical_size.unwrap();
    overlay
        .stage_frame(0, &vec![255; size.0 as usize * size.1 as usize * 4], None)
        .unwrap();
    let region = tiny_skia::Rect::from_xywh(30.0, 30.0, 200.0, 180.0).unwrap();
    let panel = tiny_skia::Rect::from_xywh(700.0, 500.0, 150.0, 80.0).unwrap();
    overlay.configure_scroll_panel(panel, false).unwrap();
    overlay.begin_scroll(0).unwrap();
    assert!(
        !overlay
            .allow_scroll_frame(
                &mark_shot::types::MonitorFrame {
                    output: 0,
                    pixels: Vec::new(),
                    pw_width: 0,
                    pw_height: 0,
                    pw_stride: 0,
                    info: mark_shot::types::StreamInfo {
                        node_id: 0,
                        size: None,
                        position: None
                    },
                },
                region
            )
            .unwrap()
    );
    std::thread::sleep(Duration::from_millis(160));
    assert_eq!(
        display
            .connection
            .get_input_focus()
            .unwrap()
            .reply()
            .unwrap()
            .focus,
        client
    );
    let pixels = vec![255; 150 * 80 * 4];
    overlay
        .stage_scroll_panel(
            0,
            &mark_shot::types::scroll_panel::ScrollPanelFrame {
                pixels: &pixels,
                size: (150, 80),
                bounds: panel.round_out().unwrap(),
                region,
                scale: 1.0,
                border: tiny_skia::Color::BLACK,
            },
        )
        .unwrap();
    let method = x11::capture::Method {
        display: display.clone(),
        include_cursor: false,
    };
    let frame = method
        .capture_frame(std::slice::from_ref(&output))
        .await
        .unwrap()
        .frames
        .remove(0);
    assert!(!overlay.allow_scroll_frame(&frame, region).unwrap());
    assert!(overlay.allow_scroll_frame(&frame, region).unwrap());
    let point = ((40 * frame.pw_width + 40) * 4) as usize;
    assert_eq!(&frame.pixels[point..point + 4], &[0x21, 0x4f, 0x81, 255]);
    assert_eq!(overlay.surface_output(0).unwrap().info.id, output.info.id);
    overlay.configure_scroll_panel(region, false).unwrap();
    assert!(!overlay.allow_scroll_frame(&frame, region).unwrap());
    assert!(overlay.scroll_controls_hidden());
    assert!(!overlay.allow_scroll_frame(&frame, region).unwrap());
    std::thread::sleep(Duration::from_millis(160));
    assert!(!overlay.allow_scroll_frame(&frame, region).unwrap());
    assert!(overlay.allow_scroll_frame(&frame, region).unwrap());
    drop(overlay);
    display
        .connection
        .destroy_window(client)
        .unwrap()
        .check()
        .unwrap();
    display
        .connection
        .delete_property(
            display.screen().root,
            display.atom("_NET_ACTIVE_WINDOW").unwrap(),
        )
        .unwrap()
        .check()
        .unwrap();
}

/// 【X11集成测试】【超时与唤醒】无参数，验证覆盖层无限等待可由后台句柄唤醒，超时不会空转
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated Xvfb"]
fn overlay_wakes_without_polling_and_preserves_timeout() {
    let display = private_display();
    let mut overlay = backend::initialize_overlay(Session::X11(display)).unwrap();
    let wake = overlay.task_wake().unwrap();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        wake.notify();
    });
    assert!(matches!(
        overlay.next_event(None).unwrap(),
        OverlayEvent::Tick
    ));
    thread.join().unwrap();
    let started = std::time::Instant::now();
    assert!(matches!(
        overlay.next_event(Some(Duration::from_millis(30))).unwrap(),
        OverlayEvent::Tick
    ));
    assert!(started.elapsed() >= Duration::from_millis(25));
}
