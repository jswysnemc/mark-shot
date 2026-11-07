//! 【截图区域识别回归】【真实分派】候选、逻辑阈值、滚轮、直接交付与局部像素

use super::*;
use crate::{
    editor::EditorState,
    features::region_detection::{Candidate, Detector, PixelRect, Source, window::Window},
    tools::Tool,
    types::{Mode, MouseButton, Outputs, ScrollEvent},
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【截图区域识别回归】【输入环境】按 image、windows 与 scale 建立冻结截图和独立候选，返回可直接输入的编辑器
fn editor(image: bool, windows: bool, scale: f32) -> EditorState {
    let mut state = crate::app::input::tests::editor();
    state.ui_hidden = true;
    let mut pixels = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    pixels.fill(Color::from_rgba8(30, 34, 38, 255));
    state.captures[0].pixmap = pixels.into();
    state.captures[0].scale = scale;
    state.placements[0].scale = scale;
    let space = geometry::Space::for_editor(&state).unwrap();
    let pixel = |x: f32, y: f32, w: f32, h: f32| {
        PixelRect::new(
            (x * scale) as i32,
            (y * scale) as i32,
            (w * scale) as i32,
            (h * scale) as i32,
        )
    };
    let window = Window {
        rect: pixel(40.0, 40.0, 360.0, 250.0),
        z_order: Some(2),
    };
    let detector = image.then(|| {
        let mut detector = Detector::from_candidates(
            space.size.0,
            space.size.1,
            vec![Candidate {
                rect: pixel(100.0, 80.0, 140.0, 100.0),
                quality: 0.95,
                sides: [0.95; 4],
                interior_edges: 0.0,
                sources: vec![Source::Contour],
            }],
        );
        if windows {
            detector.add_windows(&[window.rect]);
        }
        detector
    });
    state.region_detection = State {
        space: (image || windows).then_some(space),
        windows: if windows { vec![window] } else { Vec::new() },
        detector,
        ..State::default()
    };
    state
}

/// 【截图区域识别回归】【指针投递】按 state、x、y 调用正式移动分派，dirty 返回损坏显示器，无返回值
fn pointer(state: &mut EditorState, x: f64, y: f64, dirty: &mut u32) {
    crate::app::input::handle_pointer_move(state, 0, x, y, dirty);
}

/// 【截图区域识别回归】【按钮投递】按 state、button、pressed 调用正式按钮分派，dirty 返回损坏显示器，无返回值
fn button(state: &mut EditorState, button: MouseButton, pressed: bool, dirty: &mut u32) {
    crate::app::input::handle_pointer_button(state, button, pressed, dirty);
}

/// 【截图区域识别回归】【开关组合】验证四种来源组合的悬停与短点击，候选不提前成为正式选区，无参数、无返回值
#[test]
fn hover_and_click_keep_selection_separate_for_all_source_combinations() {
    for scale in [1.0, 1.5] {
        for (image, windows) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut state = editor(image, windows, scale);
            let mut dirty = 0;
            pointer(&mut state, 130.0, 120.0, &mut dirty);
            let expected = if image {
                Rect::from_xywh(100.0, 80.0, 140.0, 100.0)
            } else if windows {
                Rect::from_xywh(40.0, 40.0, 360.0, 250.0)
            } else {
                None
            };
            assert_eq!(state.region_detection.hovered, expected);
            assert!(state.selection.zone.is_none());
            button(&mut state, MouseButton::Left, true, &mut dirty);
            pointer(&mut state, 134.0, 120.0, &mut dirty);
            assert_eq!(state.region_detection.hovered, expected);
            button(&mut state, MouseButton::Left, false, &mut dirty);
            assert_eq!(state.selection.zone, expected);
            assert!(state.region_detection.hovered.is_none());
        }
    }
}

/// 【截图区域识别回归】【手动阈值】五逻辑像素和欧氏距离边界清除识别候选，移回原点后也不得重新采纳，无参数、无返回值
#[test]
fn manual_drag_dismisses_at_five_logical_pixels_and_stays_dismissed() {
    for scale in [1.0, 1.5] {
        for (dx, dy) in [(5.0, 0.0), (3.0, 4.0), (12.0, 10.0)] {
            let mut state = editor(true, true, scale);
            let mut dirty = 0;
            pointer(&mut state, 130.0, 120.0, &mut dirty);
            button(&mut state, MouseButton::Left, true, &mut dirty);
            pointer(&mut state, 130.0 + dx, 120.0 + dy, &mut dirty);
            assert!(state.region_detection.hovered.is_none());
            pointer(&mut state, 131.0, 121.0, &mut dirty);
            button(&mut state, MouseButton::Left, false, &mut dirty);
            assert!(state.selection.zone.is_none());
        }
    }
}

/// 【截图区域识别回归】【合并轨迹】中途实际越过五逻辑像素后返回原点，释放不能采纳原候选，无参数、无返回值
#[test]
fn coalesced_excursion_dismisses_candidate_at_application_dispatch() {
    use crate::types::{OverlayEvent, pointer_motion::PointerMotion};
    for scale in [1.0, 1.5] {
        for point in [(135.0, 120.0), (133.0, 124.0), (142.0, 130.0)] {
            let mut state = editor(true, true, scale);
            let mut dirty = 0;
            pointer(&mut state, 130.0, 120.0, &mut dirty);
            button(&mut state, MouseButton::Left, true, &mut dirty);
            crate::app::event_loop::handle_event(
                &mut state,
                OverlayEvent::PointerMove {
                    monitor_idx: 0,
                    x: 130.0,
                    y: 120.0,
                    history: vec![PointerMotion::new(0, point)],
                },
                &mut dirty,
            );
            assert!(state.region_detection.hovered.is_none());
            button(&mut state, MouseButton::Left, false, &mut dirty);
            assert!(state.selection.zone.is_none());
        }
    }
}

/// 【截图区域识别回归】【实际位移】横纵极值来自不同位置时，不得使用未经过的矩形角点撤销短点击，无参数、无返回值
#[test]
fn coalesced_short_points_keep_candidate_without_diagonal_overestimate() {
    use crate::types::{OverlayEvent, pointer_motion::PointerMotion};
    let mut state = editor(true, true, 1.0);
    let mut dirty = 0;
    pointer(&mut state, 130.0, 120.0, &mut dirty);
    let candidate = state.region_detection.hovered;
    button(&mut state, MouseButton::Left, true, &mut dirty);
    crate::app::event_loop::handle_event(
        &mut state,
        OverlayEvent::PointerMove {
            monitor_idx: 0,
            x: 131.0,
            y: 121.0,
            history: vec![
                PointerMotion::new(0, (134.0, 120.0)),
                PointerMotion::new(0, (130.0, 124.0)),
            ],
        },
        &mut dirty,
    );
    assert_eq!(state.region_detection.hovered, candidate);
    button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.selection.zone, candidate);
}

/// 【截图区域识别回归】【跨输出轨迹】各输出使用逻辑原点换算位移，未知输出忽略，无参数、无返回值
#[test]
fn coalesced_motion_uses_output_origin_without_scale_multiplication() {
    use crate::types::{OverlayEvent, pointer_motion::PointerMotion};
    let mut state = editor(true, true, 1.5);
    let mut output = state.placements[0].clone();
    output.position = (128, 116);
    state.placements.push(output);
    let mut dirty = 0;
    pointer(&mut state, 130.0, 120.0, &mut dirty);
    let candidate = state.region_detection.hovered;
    button(&mut state, MouseButton::Left, true, &mut dirty);
    for (point, expected) in [((4.0, 6.0), candidate), ((5.0, 8.0), None)] {
        crate::app::event_loop::handle_event(
            &mut state,
            OverlayEvent::PointerMove {
                monitor_idx: 0,
                x: 130.0,
                y: 120.0,
                history: vec![
                    PointerMotion::new(99, (500.0, 500.0)),
                    PointerMotion::new(1, point),
                ],
            },
            &mut dirty,
        );
        assert_eq!(state.region_detection.hovered, expected);
    }
}

/// 【截图区域识别回归】【层级滚轮】正滚轮扩大到窗口和整图，负滚轮缩小，手动容差使用图像像素，无参数、无返回值
#[test]
fn wheel_hierarchy_and_manual_pointer_tolerance_match_image_pixels() {
    let mut state = editor(true, true, 1.5);
    let mut dirty = 0;
    pointer(&mut state, 130.0, 120.0, &mut dirty);
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(40.0, 40.0, 360.0, 250.0)
    );
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0)
    );
    pointer(&mut state, 134.0, 120.0, &mut dirty);
    assert!(state.region_detection.cursor.manual_level());
    pointer(&mut state, 135.0, 120.0, &mut dirty);
    assert!(!state.region_detection.cursor.manual_level());
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(100.0, 80.0, 140.0, 100.0)
    );
    assert!(state.selection.zone.is_none());
}

/// 【截图区域识别回归】【交付职责】短点击沿用区域直接交付与首次默认工具切换，右键关闭选择阶段，无参数、无返回值
#[test]
fn candidate_adoption_uses_existing_delivery_and_default_tool_workflows() {
    for mode in [Mode::Editor, Mode::Region] {
        let mut state = editor(true, true, 1.0);
        state.mode = mode;
        state.to = Outputs {
            copy: true,
            ..Outputs::default()
        };
        state.after_selection_tool = Tool::Pen;
        let mut dirty = 0;
        pointer(&mut state, 130.0, 120.0, &mut dirty);
        button(&mut state, MouseButton::Left, true, &mut dirty);
        button(&mut state, MouseButton::Left, false, &mut dirty);
        assert!(state.selection.zone.is_some());
        if mode == Mode::Region {
            assert_eq!(state.finish, Some(state.to));
        } else {
            assert_eq!(state.selected_tool, Tool::Pen);
        }
    }
    let mut state = editor(true, true, 1.0);
    let mut dirty = 0;
    pointer(&mut state, 130.0, 120.0, &mut dirty);
    button(&mut state, MouseButton::Right, true, &mut dirty);
    assert!(state.cancel);
    assert!(state.selection.zone.is_none());
}

/// 【截图区域识别回归】【完整帧】按 state、damage 和 dimmed_for 绘制当前帧，返回实际像素
fn frame(state: &mut EditorState, damage: Option<Rect>, dimmed_for: &mut Option<Rect>) -> Vec<u8> {
    let frame = crate::app::frame_render::Frame {
        active_text_id: None,
        scan_badge: None,
        current_color: Color::WHITE,
        outline_color: Color::BLACK,
        annotations_hidden: false,
    };
    crate::app::frame_render::render_monitor(state, 0, &frame, damage, dimmed_for);
    state.canvas[0].data().to_vec()
}

/// 【截图区域识别回归】【候选恢复】更新和移除透明候选后局部重绘必须与完整帧逐字节一致，无参数、无返回值
#[test]
fn removing_candidate_restores_its_entire_fill_and_border() {
    let mut state = editor(true, true, 1.0);
    let mut dirty = 0;
    let mut dimmed = None;
    pointer(&mut state, 130.0, 120.0, &mut dirty);
    frame(&mut state, None, &mut dimmed);
    state.damage_rects.clear();
    pointer(&mut state, 500.0, 350.0, &mut dirty);
    let damage = state.monitor_dirty_rect(0);
    assert!(damage.is_some());
    let partial = frame(&mut state, damage, &mut dimmed);
    assert_eq!(partial, frame(&mut state, None, &mut dimmed));
}
