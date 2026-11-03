//! 【字号输入测试】【实际探针对照】期望序列来自原版 Qt 控件探针，区分画布、滑条、数字框和字体列表

use mark_shot::{
    features::text::size_input::SizeInput,
    types::{ScrollAxis, ScrollDirection, ScrollEvent, ScrollSource},
};

/// 【字号输入测试】【序列执行】按 deltas 构造输入并调用 apply，返回从 start 开始的数值序列
fn sequence(start: i32, deltas: &[i32], mut apply: impl FnMut(i32) -> i32) -> Vec<i32> {
    let mut result = vec![start];
    for delta in deltas {
        result.push((result.last().unwrap() + apply(*delta)).clamp(8, 300));
    }
    result
}

/// 【字号输入测试】【画布刻度】验证一格一点五点及小刻度余量，无参数、无返回值
#[test]
fn canvas_matches_real_qt_whole_and_high_resolution_trace() {
    let mut input = SizeInput::default();
    assert_eq!(
        sequence(20, &[120, 120, -120, -120], |delta| input
            .canvas(ScrollEvent::wheel(delta).canvas_steps())),
        [20, 21, 23, 22, 20]
    );
    assert_eq!(input.remainder(), 0.0);
    assert_eq!(
        sequence(20, &[30; 8], |delta| input
            .canvas(ScrollEvent::wheel(delta).canvas_steps())),
        [20, 20, 20, 21, 21, 21, 22, 22, 23]
    );
    assert_eq!(input.remainder(), 0.0);
}

/// 【字号输入测试】【画布像素】验证八十像素一格及反向余量，无参数、无返回值
#[test]
fn canvas_pixels_and_limits_keep_signed_remainders() {
    let mut input = SizeInput::default();
    assert_eq!(
        sequence(20, &[20, 20, 20, 20, -40, -40], |delta| input
            .canvas(ScrollEvent::pixels(0.0, delta as f64).canvas_steps())),
        [20, 20, 20, 21, 21, 21, 20]
    );
    assert_eq!(input.remainder(), 0.0);
    assert_eq!(
        sequence(300, &[120, -120, -120], |delta| input
            .canvas(ScrollEvent::wheel(delta).canvas_steps())),
        [300, 300, 299, 298]
    );
    assert_eq!(input.remainder(), -0.5);
}

/// 【字号输入测试】【滑条刻度】验证一格三值及反向清除余量，与按行字体列表使用相同增量，无参数、无返回值
#[test]
fn slider_and_font_list_match_real_qt_direction_changes() {
    let mut input = SizeInput::default();
    assert_eq!(
        sequence(20, &[120, 120, -120, -120], |delta| input.slider(
            delta as f64,
            false,
            10
        )),
        [20, 23, 26, 23, 20]
    );
    assert_eq!(
        sequence(20, &[30, 30, -30, 30, 30, 30, 30], |delta| input.slider(
            delta as f64,
            false,
            10
        )),
        [20, 20, 21, 21, 21, 22, 23, 24]
    );
    assert_eq!(
        sequence(20, &[120, -120], |delta| input.slider(
            delta as f64,
            true,
            10
        )),
        [20, 30, 20]
    );
}

/// 【字号输入测试】【数字框刻度】验证整刻度才改变字号、反向保留余量及修饰键十点增量，无参数、无返回值
#[test]
fn spin_matches_real_qt_accumulated_angle_and_control_step() {
    let mut input = SizeInput::default();
    assert_eq!(
        sequence(20, &[120, 120, -120, -120], |delta| input
            .spin(delta as f64, false)),
        [20, 21, 22, 21, 20]
    );
    assert_eq!(
        sequence(20, &[30, 30, -30, 30, 30, 30, 30], |delta| input
            .spin(delta as f64, false)),
        [20, 20, 20, 20, 20, 20, 21, 21]
    );
    input.reset();
    assert_eq!(
        sequence(20, &[120, -120], |delta| input.spin(delta as f64, true)),
        [20, 30, 20]
    );
}

/// 【字号输入测试】【轴优先级】验证高分辨率刻度不重复叠加旧刻度和像素，自然滚动方向信息不再次反号，无参数、无返回值
#[test]
fn metadata_prefers_value120_and_preserves_stop_only_events() {
    let event = ScrollEvent {
        vertical: ScrollAxis {
            pixels: 10.0,
            value120: 30,
            angle_delta: None,
            discrete: 1,
            direction: Some(ScrollDirection::Inverted),
            stop: true,
        },
        source: Some(ScrollSource::Wheel),
        ..Default::default()
    };
    assert_eq!(event.angle_y(), 30.0);
    assert_eq!(event.canvas_steps(), 0.25);
    assert!(!event.is_empty());
    let discrete = ScrollEvent {
        vertical: ScrollAxis {
            discrete: -1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(discrete.canvas_steps(), -1.0);
    let stop = ScrollEvent {
        vertical: ScrollAxis {
            stop: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!stop.is_empty());
    assert_eq!(stop.canvas_steps(), 0.0);
    assert!(ScrollEvent::default().is_empty());
}

/// 【字号输入测试】【非有限输入】验证非法像素不破坏累积器，触控板像素不合成控件角度，无参数、无返回值
#[test]
fn pixel_only_controls_and_invalid_deltas_do_not_change_values() {
    let mut input = SizeInput::default();
    for delta in [20.0, f64::NAN, f64::INFINITY, -40.0] {
        let event = ScrollEvent::pixels(0.0, delta);
        assert_eq!(input.slider(event.angle_y(), false, 10), 0);
        assert_eq!(input.spin(event.angle_y(), false), 0);
    }
    assert_eq!(input.canvas(0.5), 0);
    assert_eq!(input.canvas(f64::NAN), 0);
    assert_eq!(input.remainder(), 0.75);
    input.reset();
    assert_eq!(input.remainder(), 0.0);
    let old_wheel = ScrollEvent {
        vertical: ScrollAxis {
            pixels: 10.0,
            ..Default::default()
        },
        source: Some(ScrollSource::Wheel),
        ..Default::default()
    };
    assert_eq!(old_wheel.angle_y(), 120.0);
}
