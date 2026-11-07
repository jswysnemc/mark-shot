//! 【尺子测试】【输入链路】验证实际工具分派、三点草稿、控制点变换、取消与导出

use tiny_skia::{Color, Pixmap, Rect};

use super::{editor, key, pointer, widget_center};
use crate::app::input::{handle_modifiers, handle_pointer_button, select_tool};
use crate::app::panels;
use crate::editor::EditorState;
use crate::editor::file_canvas::FileCanvas;
use crate::features::ruler::drag;
use crate::features::ruler::{Measurement, RulerGeometry, RulerMode};
use crate::tools::Tool;
use crate::types::{AnnotationShape, MouseButton, StrokeGroup};
use crate::ui::panel::AnimatedPanel;
use crate::ui::settings_panel::{SettingsAction, SettingsWidget};

/// 【尺子测试】【工具构造】返回指定 mode 的尺子编辑器；dirty 返回初始化重绘显示器
fn ruler_editor(mode: RulerMode, dirty: &mut u32) -> EditorState {
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.tool_settings.ruler_mode = mode;
    select_tool(&mut state, Tool::Ruler, dirty);
    state.toolbar.opacity = 1.0;
    state.toolbar.render_pos = state.toolbar.position;
    panels::settings::update_settings_panel(&mut state, dirty);
    state
}

/// 【尺子测试】【完整拖动】在 state 中从 start 拖动到 end；dirty 返回重绘显示器，无返回值
fn draw(state: &mut EditorState, start: (f64, f64), end: (f64, f64), dirty: &mut u32) {
    pointer(state, start.0, start.1, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    pointer(state, end.0, end.1, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【尺子测试】【单击定点】在 state 中单击 point，使用真实鼠标分派；dirty 返回重绘显示器，无返回值
fn click(state: &mut EditorState, point: (f64, f64), dirty: &mut u32) {
    pointer(state, point.0, point.1, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【尺子测试】【测量读取】返回 state 的 index 持久尺子，类型错误时使测试失败
fn measurement(state: &EditorState, index: usize) -> Measurement {
    let AnnotationShape::Ruler { measurement } = state.annotations[index].shape else {
        panic!("ruler input committed a different annotation type")
    };
    measurement
}

/// 【尺子测试】【平直与空手势】点击不产生标注，平直长度尺可提交，反向框测量正确且撤销只有一项
#[test]
fn empty_and_straight_length_gestures_have_correct_history() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    click(&mut state, (140.0, 130.0), &mut dirty);
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
    draw(&mut state, (140.0, 130.0), (300.0, 130.0), &mut dirty);
    assert_eq!(
        measurement(&state, 0).length_metrics(),
        Some((160, 0, 160, 0))
    );
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert!(state.annotations.is_empty());
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(
        measurement(&state, 0).length_metrics(),
        Some((160, 0, 160, 0))
    );
    draw(&mut state, (450.0, 350.0), (350.0, 230.0), &mut dirty);
    assert_eq!(
        measurement(&state, 1).length_metrics(),
        Some((100, 120, 156, 12000))
    );
}

/// 【尺子测试】【原图边界】长度按原始像素测量；源图外按下不产生尺子，结束点限定在源图范围
#[test]
fn ruler_image_gestures_use_native_pixels_and_require_an_inside_start() {
    let mut dirty = 0;
    let mut state = editor();
    let mut source = Pixmap::new(1200, 800).unwrap();
    source.fill(Color::from_rgba8(30, 34, 38, 255));
    let file = FileCanvas::new(source, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
    let viewport = file.viewport;
    let scale = file.original.width() as f32 / viewport.width();
    state.selection.zone = Some(viewport);
    state.file_canvas = Some(file);
    state.tool_settings.ruler_mode = RulerMode::Length;
    select_tool(&mut state, Tool::Ruler, &mut dirty);
    draw(
        &mut state,
        (viewport.left() as f64 - 3.0, 160.0),
        (200.0, 220.0),
        &mut dirty,
    );
    assert!(state.annotations.is_empty());
    draw(&mut state, (200.0, 200.0), (300.0, 260.0), &mut dirty);
    let ruler = measurement(&state, 0);
    assert_eq!(ruler.pixel_scale, scale);
    assert_eq!(
        ruler.length_metrics().unwrap().0,
        (100.0 * scale).round() as u64
    );
    draw(&mut state, (400.0, 300.0), (900.0, 750.0), &mut dirty);
    let RulerGeometry::Length { end, .. } = measurement(&state, 1).geometry else {
        panic!("length mode changed")
    };
    assert_eq!(end, (viewport.right(), viewport.bottom()));
}

/// 【尺子测试】【角度三点】单击顶点后无需保持按下，第一边和第二边分别确认，只有完整角度进入历史
#[test]
fn angle_points_can_be_placed_with_three_clicks() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Angle, &mut dirty);
    click(&mut state, (180.0, 170.0), &mut dirty);
    assert!(state.pending.is_some() && state.annotations.is_empty());
    assert!(!state.input.mouse_down && !state.tool_active);
    pointer(&mut state, 330.0, 170.0, &mut dirty);
    click(&mut state, (330.0, 170.0), &mut dirty);
    assert!(state.pending.is_some() && state.undo_stack.is_empty());
    pointer(&mut state, 180.0, 320.0, &mut dirty);
    click(&mut state, (180.0, 320.0), &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(
        measurement(&state, 0).info_text().as_deref(),
        Some("90.0\u{b0}")
    );
    assert!(state.pending.is_none());
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【尺子测试】【角度拖动】第一边可用拖动确认，短于四逻辑像素的点击不会提前确认第二边
#[test]
fn drag_then_click_and_short_angle_releases_keep_the_correct_stage() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Angle, &mut dirty);
    draw(&mut state, (180.0, 170.0), (330.0, 170.0), &mut dirty);
    click(&mut state, (332.0, 172.0), &mut dirty);
    assert!(state.annotations.is_empty() && state.pending.is_some());
    click(&mut state, (180.0, 320.0), &mut dirty);
    assert_eq!(
        measurement(&state, 0).info_text().as_deref(),
        Some("90.0\u{b0}")
    );
}

/// 【尺子测试】【Shift 即时约束】按下或释放 Shift 都会更新原地草稿，不要求再次移动指针
#[test]
fn shift_snapping_updates_an_angle_without_extra_pointer_motion() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Angle, &mut dirty);
    click(&mut state, (180.0, 170.0), &mut dirty);
    pointer(&mut state, 280.0, 218.0, &mut dirty);
    let original = state.pending.as_ref().unwrap().clone();
    handle_modifiers(&mut state, false, true, &mut dirty);
    let AnnotationShape::Ruler { measurement } = state.pending.as_ref().unwrap().shape else {
        panic!("angle type changed")
    };
    let RulerGeometry::Angle { points, .. } = measurement.geometry else {
        panic!("angle mode changed")
    };
    assert!((points[1].0 - 180.0 - (points[1].1 - 170.0)).abs() < 0.001);
    handle_modifiers(&mut state, false, false, &mut dirty);
    assert_eq!(state.pending.as_ref().unwrap(), &original);
}

/// 【尺子测试】【再次激活】再次按 U 切换长度与角度，撤去旧草稿但保留已完成尺子
#[test]
fn selecting_the_active_ruler_toggles_mode_and_only_cancels_the_draft() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    draw(&mut state, (160.0, 130.0), (300.0, 240.0), &mut dirty);
    let completed = state.annotations[0].clone();
    pointer(&mut state, 430.0, 300.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 500.0, 400.0, &mut dirty);
    key(&mut state, "U", &mut dirty);
    assert_eq!(state.tool_settings.ruler_mode, RulerMode::Angle);
    assert!(state.pending.is_none() && !state.input.mouse_down);
    assert_eq!(state.annotations, [completed]);
    key(&mut state, "U", &mut dirty);
    assert_eq!(state.tool_settings.ruler_mode, RulerMode::Length);
}

/// 【尺子测试】【模式弹层】Enter 只选择模式，Escape 只关闭弹层，重复点击按钮不会重新打开
#[test]
fn ruler_mode_popover_consumes_confirmation_and_close_actions() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    panels::ruler::toggle(&mut state, &mut dirty);
    state.ruler_popover.animate_step(1.0);
    key(&mut state, "End", &mut dirty);
    key(&mut state, "Return", &mut dirty);
    assert_eq!(state.tool_settings.ruler_mode, RulerMode::Angle);
    assert!(!state.ruler_popover.open && state.finish.is_none());
    assert!(state.undo_stack.is_empty());
    panels::ruler::toggle(&mut state, &mut dirty);
    state.ruler_popover.animate_step(1.0);
    key(&mut state, "Escape", &mut dirty);
    assert!(!state.cancel && !state.ruler_popover.open);
    panels::ruler::toggle(&mut state, &mut dirty);
    state.ruler_popover.animate_step(1.0);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Action {
                    action: SettingsAction::RulerModes,
                    ..
                }
            )
        })
        .unwrap();
    let point = widget_center(&state, index);
    click(&mut state, point, &mut dirty);
    assert!(!state.ruler_popover.open);
}

/// 【尺子测试】【选区入口退出】从选区进入时 Escape 先取消草稿，再清除本次测量并返回选区，第三次才退出
#[test]
fn escape_cancels_the_draft_then_returns_to_selection_before_exiting() {
    let mut dirty = 0;
    let mut state = editor();
    state.tool_settings.ruler_mode = RulerMode::Length;
    key(&mut state, "U", &mut dirty);
    assert!(state.ruler.entered_from_selection);
    draw(&mut state, (140.0, 130.0), (260.0, 240.0), &mut dirty);
    select_tool(&mut state, Tool::Ruler, &mut dirty);
    click(&mut state, (400.0, 330.0), &mut dirty);
    assert!(state.pending.is_some());
    key(&mut state, "Escape", &mut dirty);
    assert!(state.pending.is_none() && !state.cancel);
    assert_eq!(state.annotations.len(), 1);
    key(&mut state, "Escape", &mut dirty);
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
    assert!(state.selection.zone.is_none());
    assert_eq!(state.selected_tool, Tool::Selection);
    assert!(!state.cancel);
    key(&mut state, "Escape", &mut dirty);
    assert!(state.cancel);
}

/// 【尺子测试】【内部新测量】尺子工具下长度框内部用于新测量，而不是把已有测量整体拖动
#[test]
fn length_frame_interior_starts_a_new_ruler() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    draw(&mut state, (100.0, 110.0), (430.0, 390.0), &mut dirty);
    let original = state.annotations[0].clone();
    draw(&mut state, (210.0, 210.0), (320.0, 310.0), &mut dirty);
    assert_eq!(state.annotations.len(), 2);
    assert_eq!(state.annotations[0], original);
    assert_eq!(
        measurement(&state, 1).length_metrics(),
        Some((110, 100, 149, 11000))
    );
}

/// 【尺子测试】【控制点与历史】角度端点通过真实拖动修改，旋转与 Ctrl 选中框调整只形成一次历史
#[test]
fn angle_handles_rotation_and_proportional_resize_are_undoable() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Angle, &mut dirty);
    draw(&mut state, (150.0, 150.0), (290.0, 150.0), &mut dirty);
    click(&mut state, (250.0, 290.0), &mut dirty);
    let original = state.annotations[0].clone();
    draw(&mut state, (290.0, 150.0), (320.0, 200.0), &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    assert_ne!(state.annotations[0], original);
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations[0], original);
    let ruler = measurement(&state, 0);
    let handle = drag::rotation_handle(ruler, 3.0).unwrap();
    draw(
        &mut state,
        (handle.0 as f64, handle.1 as f64),
        (340.0, 220.0),
        &mut dirty,
    );
    assert_ne!(measurement(&state, 0).geometry, ruler.geometry);
    assert_eq!(measurement(&state, 0).info_text(), ruler.info_text());
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations[0], original);
    let frame = drag::frame(ruler).unwrap();
    handle_modifiers(&mut state, true, false, &mut dirty);
    draw(
        &mut state,
        (frame.right() as f64, frame.bottom() as f64),
        (frame.right() as f64 + 70.0, frame.bottom() as f64 + 20.0),
        &mut dirty,
    );
    assert_eq!(measurement(&state, 0).info_text(), ruler.info_text());
    handle_modifiers(&mut state, false, false, &mut dirty);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations[0], original);
}

/// 【尺子测试】【选择工具旋转】按实际笔宽命中已选尺子的框外旋转把手，验证完整拖动与撤销，无参数、无返回值
#[test]
fn pick_tool_can_rotate_a_selected_ruler_using_its_outside_handle() {
    for width in [3.0, 7.0, 15.0] {
        let mut dirty = 0;
        let mut state = ruler_editor(RulerMode::Length, &mut dirty);
        state.tool_settings.set_stroke(StrokeGroup::Ruler, width);
        draw(&mut state, (224.0, 220.0), (384.0, 345.0), &mut dirty);
        select_tool(&mut state, Tool::Pick, &mut dirty);
        click(&mut state, (304.0, 221.0), &mut dirty);
        assert_eq!(state.selected_annotation_index(), Some(0));
        let original = state.annotations[0].clone();
        assert_eq!(original.stroke_width, width);
        let handle = drag::rotation_handle(measurement(&state, 0), original.stroke_width).unwrap();
        assert_eq!(
            crate::editor::annotation_drag::handle_hit_test_for_annotation(
                &original,
                (handle.0 as f64, handle.1 as f64),
            ),
            crate::types::SelectionHandle::Rotate,
        );
        // 1. 【尺子测试】【按下事务】框外把手必须建立旋转事务，不能取消当前选择
        pointer(&mut state, handle.0 as f64, handle.1 as f64, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert_eq!(state.selected_annotation_index(), Some(0));
        assert_eq!(
            state.ann_drag.as_ref().unwrap().handle,
            crate::types::SelectionHandle::Rotate
        );
        // 2. 【尺子测试】【移动交付】释放后只提交一次历史，撤销恢复完整原标注
        pointer(&mut state, 424.0, 282.5, &mut dirty);
        assert!(state.pending.is_some());
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.selected_annotation_index(), Some(0));
        assert_ne!(state.annotations[0], original);
        assert_eq!(state.undo_stack.len(), 2);
        key(&mut state, "Ctrl+Z", &mut dirty);
        assert_eq!(state.annotations, [original]);
    }
}

/// 【尺子测试】【独立笔宽】修改尺子默认笔宽不改变标记、激光或普通绘图；未完成撤销不产生空历史
#[test]
fn ruler_width_is_independent_and_draft_undo_does_not_remove_completed_measurements() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    let previous = state.tool_settings.clone();
    key(&mut state, "]", &mut dirty);
    assert!(state.tool_settings.stroke(StrokeGroup::Ruler) > previous.stroke(StrokeGroup::Ruler));
    for group in StrokeGroup::ALL
        .into_iter()
        .filter(|group| *group != StrokeGroup::Ruler)
    {
        assert_eq!(state.tool_settings.stroke(group), previous.stroke(group));
    }
    assert!(state.undo_stack.is_empty());
    draw(&mut state, (140.0, 130.0), (280.0, 240.0), &mut dirty);
    let completed = state.annotations[0].clone();
    pointer(&mut state, 410.0, 300.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 510.0, 410.0, &mut dirty);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations, [completed]);
    assert!(state.pending.is_none() && !state.input.mouse_down && !state.tool_active);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【尺子测试】【调整撤销】鼠标仍按住时撤销控制点拖动，恢复原标注并结束输入状态，不要求释放后才能切换工具
#[test]
fn undoing_a_ruler_handle_drag_restores_original_geometry_and_ends_input() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    draw(&mut state, (140.0, 130.0), (280.0, 240.0), &mut dirty);
    let original = state.annotations[0].clone();
    pointer(&mut state, 280.0, 240.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 330.0, 300.0, &mut dirty);
    assert!(state.ann_drag.is_some() && state.pending.is_some());
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations, [original]);
    assert!(state.pending.is_none() && state.ann_drag.is_none());
    assert!(!state.input.mouse_down && !state.tool_active);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【尺子测试】【空闲轮询】稳定尺子悬停、测量说明和已打开模式面板均不产生定时刷新
#[test]
fn stable_ruler_hover_labels_and_open_popover_do_not_poll() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    draw(&mut state, (140.0, 130.0), (280.0, 240.0), &mut dirty);
    pointer(&mut state, 440.0, 370.0, &mut dirty);
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    state.toolbar.render_pos = state.toolbar.position;
    state.toolbar.opacity = if state.toolbar.interferes { 0.0 } else { 1.0 };
    assert!(state.ruler.hover.is_some() && !state.ruler.labels.is_empty());
    assert_eq!(crate::app::event_loop::poll_timeout(&state), None);
    panels::ruler::toggle(&mut state, &mut dirty);
    state.ruler_popover.animate_step(1.0);
    assert!(!state.ruler_popover.is_animating());
    assert_eq!(crate::app::event_loop::poll_timeout(&state), None);
}

/// 【尺子测试】【临时悬停隔离】持久导出包括正文和说明，但不包括悬停十字线、源图坐标及颜色提示
#[test]
fn export_includes_measurement_labels_but_excludes_hover_chrome() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    draw(&mut state, (140.0, 130.0), (280.0, 240.0), &mut dirty);
    pointer(&mut state, 500.0, 400.0, &mut dirty);
    assert!(state.ruler.hover.is_some());
    let first = crate::app::render_final(&mut state).unwrap().0;
    pointer(&mut state, 400.0, 300.0, &mut dirty);
    let second = crate::app::render_final(&mut state).unwrap().0;
    assert_eq!(first, second);
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    assert_eq!(state.ruler.labels.len(), 1);
    let image = image::load_from_memory(&first).unwrap().into_rgba8();
    let label = &state.ruler.labels[0];
    let x = (label.rect.left() + label.rect.width() / 2.0).round() as u32;
    let y = (label.rect.top() + label.rect.height() / 2.0).round() as u32;
    assert_ne!(image.get_pixel(x, y).0, [30, 34, 38, 255]);
    assert_eq!(image.get_pixel(500, 400).0, [30, 34, 38, 255]);
}
