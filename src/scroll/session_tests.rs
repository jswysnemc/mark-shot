use crate::scroll::frame;
use crate::scroll::session::ScrollSession;
use shot_stitch::Axis;
use tiny_skia::Rect;

fn page_frame(origin: i32, width: u32, height: u32) -> crate::types::MonitorFrame {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in origin..origin + height as i32 {
        for x in 0..width {
            let hash = (y as u32).wrapping_mul(2_654_435_761).rotate_left(7) ^ x;
            pixels.extend_from_slice(&[
                (hash & 0xff) as u8,
                ((hash >> 8) & 0xff) as u8,
                ((hash >> 16) & 0xff) as u8,
                255,
            ]);
        }
    }
    crate::types::MonitorFrame {
        output: 0,
        pixels,
        pw_width: width,
        pw_height: height,
        pw_stride: width * 4,
        info: crate::types::StreamInfo {
            node_id: 0,
            size: Some((width as i32, height as i32)),
            position: Some((0, 0)),
        },
    }
}

fn region() -> Rect {
    Rect::from_xywh(0.0, 0.0, 40.0, 160.0).unwrap()
}

const CROP: (u32, u32, u32, u32) = (0, 0, 40, 160);

#[test]
fn scrolling_frames_stitch_into_one_tall_image() {
    let mut session = ScrollSession::new(region(), 0).unwrap();
    assert!(session.can_toggle_axis());

    let first = frame::crop(&page_frame(0, 40, 160), CROP).unwrap();
    session.push(first);
    assert_eq!(session.length_label(), "160 px");
    assert!(session.can_toggle_axis());

    let second = frame::crop(&page_frame(40, 40, 160), CROP).unwrap();
    session.push(second);
    assert_eq!(session.length_label(), "200 px");
    assert!(!session.can_toggle_axis());
    assert!(!session.toggle_axis());

    let image = session.result().unwrap();
    assert_eq!((image.width(), image.height()), (40, 200));
}

#[test]
fn repeated_identical_frames_do_not_grow_the_image() {
    let mut session = ScrollSession::new(region(), 0).unwrap();
    let same = || frame::crop(&page_frame(0, 40, 160), CROP).unwrap();

    session.push(same());
    session.push(same());
    session.push(same());
    assert_eq!(session.length_label(), "160 px");
}

#[test]
fn horizontal_axis_still_stitches() {
    let mut session = ScrollSession::new(region(), 0).unwrap();
    assert!(session.toggle_axis());
    assert_eq!(session.axis(), Axis::Horizontal);

    let first = frame::crop(&page_frame(0, 40, 160), CROP).unwrap();
    session.push(first);
    let image = session.result().unwrap();
    assert_eq!((image.width(), image.height()), (40, 160));
    assert_eq!(session.length_label(), "40 px");
}

#[test]
fn tiny_region_is_rejected() {
    let small = Rect::from_xywh(0.0, 0.0, 8.0, 8.0).unwrap();
    assert!(ScrollSession::new(small, 0).is_err());
}

#[test]
fn idle_slows_capture_down_but_never_stops_it() {
    let mut session = ScrollSession::new(region(), 0).unwrap();
    let active = session.capture_interval();

    let first = frame::crop(&page_frame(0, 40, 160), CROP).unwrap();
    session.push(first);
    std::thread::sleep(std::time::Duration::from_millis(1050));
    assert!(session.should_idle(std::time::Instant::now()));

    let idle = session.capture_interval();
    assert!(
        idle > active,
        "idle {idle:?} should be slower than {active:?}"
    );
    let later = std::time::Instant::now() + idle;
    assert!(session.wants_capture(later), "capture must keep running");

    let second = frame::crop(&page_frame(40, 40, 160), CROP).unwrap();
    session.push(second);
    assert_eq!(session.capture_interval(), active);
    assert!(!session.should_idle(std::time::Instant::now()));
}
