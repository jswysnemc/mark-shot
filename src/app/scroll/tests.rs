//! 【滚动截图测试】【输入与原图】验证真实事务边界、暂停期限和浏览后的原始图像交接

use super::*;
use crate::types::{MouseButton, ScrollEvent};
use tiny_skia::Rect;

/// 【滚动截图测试】【会话准备】返回有完整目标区域的测试运行状态，无外部窗口
fn run() -> ScrollRun {
    let region = Rect::from_xywh(0.0, 0.0, 400.0, 300.0).unwrap();
    ScrollRun {
        session: ScrollSession::new(region, 0).unwrap(),
        layout: crate::scroll::panel::layout(region, (1280.0, 800.0), None),
        hovered: None,
        pressed: None,
        pointer: (0.0, 0.0),
        axis_drag: None,
        overview_drag: None,
        preview: PreviewState::new(),
        scale: 1.5,
        dirty: false,
        needs_clean_frame: false,
        panel_limits: None,
    }
}

/// 【滚动截图测试】【按钮定位】按 button 将 state 指针移动到真实布局中的按钮中心，无返回值
fn point(state: &mut ScrollRun, button: PanelButton) {
    let rect = state
        .layout
        .buttons
        .iter()
        .find(|(value, _)| *value == button)
        .unwrap()
        .1;
    input::motion(
        state,
        (
            (rect.left() + rect.width() / 2.0) as f64,
            (rect.top() + rect.height() / 2.0) as f64,
        ),
        (1280.0, 800.0),
    );
}

/// 【滚动截图测试】【非左键保护】右键和中键都不得激活取消或改变左键事务，无参数、无返回值
#[test]
fn other_buttons_cannot_activate_or_steal_the_left_press() {
    let mut state = run();
    point(&mut state, PanelButton::Cancel);
    for button in [MouseButton::Right, MouseButton::Middle] {
        assert_eq!(input::button(&mut state, button, true), None);
        assert_eq!(input::button(&mut state, button, false), None);
        assert_eq!(state.pressed, None);
    }
    assert_eq!(input::button(&mut state, MouseButton::Left, true), None);
    assert_eq!(input::button(&mut state, MouseButton::Right, false), None);
    assert_eq!(state.pressed, Some(PanelButton::Cancel));
    assert_eq!(
        input::button(&mut state, MouseButton::Left, false),
        Some(PanelButton::Cancel)
    );
    assert_eq!(input::button(&mut state, MouseButton::Left, false), None);
}

/// 【滚动截图测试】【按钮取消】在按钮外释放和在另一按钮释放均不得交付按下动作，无参数、无返回值
#[test]
fn release_must_match_the_button_that_was_pressed() {
    let mut state = run();
    point(&mut state, PanelButton::Cancel);
    input::button(&mut state, MouseButton::Left, true);
    point(&mut state, PanelButton::TogglePause);
    assert_eq!(input::button(&mut state, MouseButton::Left, false), None);
    point(&mut state, PanelButton::Cancel);
    input::button(&mut state, MouseButton::Left, true);
    input::motion(&mut state, (0.0, 0.0), (1280.0, 800.0));
    assert_eq!(input::button(&mut state, MouseButton::Left, false), None);
}

/// 【滚动截图测试】【方向握柄】拖动仅沿选定轴移动并限制输出边界，不得误触方向切换，无参数、无返回值
#[test]
fn dragging_axis_moves_region_without_toggling_axis() {
    let mut state = run();
    point(&mut state, PanelButton::ToggleAxis);
    let start = state.pointer;
    input::button(&mut state, MouseButton::Left, true);
    input::motion(
        &mut state,
        (start.0 + 50.0, start.1 + 1000.0),
        (1280.0, 800.0),
    );
    assert_eq!(state.session.region.left(), 0.0);
    assert_eq!(state.session.region.bottom(), 800.0);
    assert_eq!(input::button(&mut state, MouseButton::Left, false), None);
    assert_eq!(state.session.axis(), shot_stitch::Axis::Vertical);
}

/// 【滚动截图测试】【窗口坐标基准】面板布局随区域变化时，静止的物理指针不得再次移动选区，后续位移仍基于首次按下位置，无参数
#[test]
fn window_drag_does_not_accumulate_moving_canvas_origin() {
    let mut state = run();
    point(&mut state, PanelButton::ToggleAxis);
    let start = state.pointer;
    input::button(&mut state, MouseButton::Left, true);
    input::window_motion(&mut state, (start.0, start.1 + 20.0), (1280.0, 800.0));
    assert_eq!(state.session.region.top(), 20.0);
    state.layout = crate::scroll::panel::layout(state.session.region, (1280.0, 800.0), None);
    input::window_motion(&mut state, (start.0, start.1 + 40.0), (1280.0, 800.0));
    assert_eq!(state.session.region.top(), 20.0);
    input::window_motion(&mut state, (start.0, start.1 + 60.0), (1280.0, 800.0));
    assert_eq!(state.session.region.top(), 40.0);
    assert_eq!(input::button(&mut state, MouseButton::Left, false), None);
    assert_eq!(state.session.axis(), shot_stitch::Axis::Vertical);
}

/// 【滚动截图测试】【期限暂停】暂停没有定期捕获期限，恢复必须立即接收帧且不能继承静止退避，无参数、无返回值
#[test]
fn pause_sleeps_and_resume_captures_immediately() {
    let mut state = run();
    state.session.set_paused(true);
    assert_eq!(state.session.next_capture(std::time::Instant::now()), None);
    state.session.set_paused(false);
    assert_eq!(
        state.session.next_capture(std::time::Instant::now()),
        Some(std::time::Duration::ZERO)
    );
}

/// 【滚动截图测试】【长图浏览】浏览和回到末端只改变预览，不改变复制结果的尺寸、像素或设备比例，无参数、无返回值
#[test]
fn preview_navigation_preserves_full_resolution_result() {
    let mut state = run();
    let image = shot_image::Image::from_pixels(80, 1000, vec![0xff123456; 80_000]).unwrap();
    state.session.push(image);
    state.layout =
        crate::scroll::panel::layout(state.session.region, (1280.0, 800.0), Some((80, 1000)));
    input::wheel(&mut state, ScrollEvent::wheel(120));
    assert!(!state.preview.following);
    assert!(state.preview.scrub > 0.0);
    let PendingOutcome::Copy { image, scale } = handoff::copy(&mut state).unwrap() else {
        panic!("copy result required")
    };
    let png = crate::scroll::png::encode(image.view()).unwrap();
    let decoded = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((decoded.width(), decoded.height(), scale), (80, 1000, 1.5));
    assert!(
        decoded
            .pixels()
            .iter()
            .all(|pixel| pixel.red() == 0x12 && pixel.green() == 0x34 && pixel.blue() == 0x56)
    );
}

/// 【滚动截图测试】【分数边界】物理裁切分别映射两端，确保非整数坐标宽度与对应原图端点一致，无参数、无返回值
#[test]
fn fractional_crop_rounds_both_edges() {
    let region = Rect::from_xywh(0.4, 0.4, 10.4, 10.4).unwrap();
    assert_eq!(capture::pixel_rect(region, 1.5), (1, 1, 15, 15));
}

/// 【滚动截图测试】【横轴浏览】协议横轴正值向右移动预览，纵轴回退仍沿统一正向刻度处理，无参数、无返回值
#[test]
fn horizontal_preview_uses_protocol_direction() {
    let mut state = run();
    state.session.toggle_axis();
    state
        .session
        .push(shot_image::Image::from_pixels(1000, 80, vec![0xff123456; 80_000]).unwrap());
    state.layout =
        crate::scroll::panel::layout(state.session.region, (1280.0, 800.0), Some((1000, 80)));
    let area = state.layout.preview.unwrap();
    let view =
        crate::scroll::panel::preview::layout(area, (1000, 80), shot_stitch::Axis::Horizontal)
            .unwrap();
    state.preview.set(0.0, &view);
    input::wheel(
        &mut state,
        ScrollEvent {
            horizontal: crate::types::ScrollAxis {
                value120: 120,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(state.preview.scrub > 0.0);
    input::wheel(&mut state, ScrollEvent::wheel(120));
    assert_eq!(state.preview.scrub, 0.0);
}
