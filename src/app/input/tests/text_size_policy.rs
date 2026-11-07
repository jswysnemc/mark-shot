//! 【字号输入测试】【原版语义】字号修改同步默认值并缩放换行框，撤销只恢复标注，不恢复工具偏好

use super::*;
use crate::{
    editor::{
        annotation_drag::rebuild_annotation,
        text_edits::{self, Property},
    },
    tools::Tool,
    types::AnnotationShape,
    ui::text_properties::layout::{Control, TextLayout},
};

/// 【字号输入测试】【场景准备】返回选中已有文字的全图编辑器，wrapped 决定是否使用九十像素换行框
fn prepared(wrapped: bool) -> EditorState {
    let mut state = editor();
    state.tool_settings.font_size = 20.0;
    super::text::add_text(&mut state);
    state.selected_tool = Tool::Pick;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    if let AnnotationShape::Text {
        content,
        bold,
        frame,
        ..
    } = &mut state.annotations[0].shape
    {
        *bold = false;
        *content = if wrapped {
            "The quick brown fox jumps over the lazy dog"
        } else {
            "Mark Shot"
        }
        .into();
        frame.width = wrapped.then_some(90.0);
    }
    rebuild_annotation(&mut state, 0);
    super::super::refresh_panels(&mut state, &mut 0);
    state
}

#[test]
fn point_input_syncs_default_size_and_proportional_wrap_width() {
    let mut state = prepared(true);
    let mut dirty = 0;
    assert_eq!(state.annotations[0].bbox.width(), 90.0);
    for (points, width) in [(30.0, 135.0), (40.0, 180.0)] {
        text_edits::set(&mut state, Property::Points(points), true, &mut dirty);
        assert_eq!(state.tool_settings.font_size, points);
        assert!(
            matches!(&state.annotations[0].shape,AnnotationShape::Text { frame,.. } if frame.width==Some(width))
        );
        assert_eq!(state.annotations[0].bbox.width(), width);
    }
}

#[test]
fn undo_size_does_not_revert_synchronized_default() {
    let mut state = prepared(false);
    let mut dirty = 0;
    text_edits::set(&mut state, Property::Points(30.0), true, &mut dirty);
    text_edits::set(&mut state, Property::Points(40.0), true, &mut dirty);
    for points in [30.0, 20.0] {
        state.undo(&mut dirty);
        assert_eq!(text_edits::current_points(&state), points);
        assert_eq!(state.tool_settings.font_size, 40.0);
    }
}

#[test]
fn slider_moves_record_each_actual_value_change_like_qt() {
    let mut state = prepared(false);
    let mut dirty = 0;
    let cell = state
        .settings_panel
        .widget_rect(Control::Size.index())
        .unwrap();
    let handle = TextLayout::slider_handle(cell, 1, (-11, 281));
    pointer(
        &mut state,
        (handle.left() + 7.0) as f64,
        (handle.top() + 6.0) as f64,
        &mut dirty,
    );
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    pointer(
        &mut state,
        (cell.left() + 31.0) as f64,
        (cell.top() + 7.0) as f64,
        &mut dirty,
    );
    let intermediate = text_edits::current_points(&state);
    pointer(
        &mut state,
        (cell.left() + 51.0) as f64,
        (cell.top() + 7.0) as f64,
        &mut dirty,
    );
    let final_points = text_edits::current_points(&state);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert_eq!(state.undo_stack.len(), 2);
    state.undo(&mut dirty);
    assert_eq!(text_edits::current_points(&state), intermediate);
    assert_eq!(state.tool_settings.font_size, final_points);
}

/// 【字号输入测试】【同值输入】同值字号不覆盖不同默认值，也不创建历史，无参数、无返回值
#[test]
fn unchanged_points_do_not_sync_a_different_default() {
    let mut state = prepared(false);
    state.tool_settings.font_size = 50.0;
    text_edits::set(&mut state, Property::Points(20.0), true, &mut 0);
    assert_eq!(state.tool_settings.font_size, 50.0);
    assert!(state.undo_stack.is_empty());
}

/// 【字号输入测试】【连续滚轮】验证生产画布入口合并连续事件，停止信号不清除余量，撤销和重做只恢复标注，无参数、无返回值
#[test]
fn canvas_wheel_groups_history_and_keeps_the_updated_default() {
    let mut state = prepared(false);
    let mut dirty = 0;
    let now = std::time::Instant::now();
    let original = state.annotations[0].clone();
    for (delta, points) in [(120, 21.0), (120, 23.0)] {
        crate::app::panels::text::size_wheel::canvas(
            &mut state,
            crate::types::ScrollEvent::wheel(delta),
            now,
            &mut dirty,
        );
        assert_eq!(text_edits::current_points(&state), points);
        assert_eq!(state.tool_settings.font_size, points);
    }
    assert!(state.undo_stack.is_empty());
    assert!(state.text_size_history.pending.is_some());
    let changed = state.annotations[0].clone();
    state.undo(&mut dirty);
    assert_eq!(state.annotations[0], original);
    assert_eq!(state.tool_settings.font_size, 23.0);
    state.redo(&mut dirty);
    assert_eq!(state.annotations[0], changed);
    assert_eq!(state.tool_settings.font_size, 23.0);
}

/// 【字号输入测试】【防抖边界】验证三百五十毫秒提交期限、四百毫秒分组及时间间隔不清除余量，无参数、无返回值
#[test]
fn delayed_wheel_inputs_have_separate_undo_steps_without_resetting_remainder() {
    let mut state = prepared(false);
    let mut dirty = 0;
    let now = std::time::Instant::now();
    crate::app::panels::text::size_wheel::canvas(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        now,
        &mut dirty,
    );
    state.tick_text_size_wheel(now + std::time::Duration::from_millis(349));
    assert!(state.undo_stack.is_empty());
    state.tick_text_size_wheel(now + std::time::Duration::from_millis(350));
    assert_eq!(state.undo_stack.len(), 1);
    assert!(state.text_size_history.deadline.is_none());
    assert_eq!(state.text_size_history.input.remainder(), 0.5);
    crate::app::panels::text::size_wheel::canvas(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        now + std::time::Duration::from_millis(400),
        &mut dirty,
    );
    assert_eq!(text_edits::current_points(&state), 23.0);
    state.undo(&mut dirty);
    assert_eq!(text_edits::current_points(&state), 21.0);
    state.undo(&mut dirty);
    assert_eq!(text_edits::current_points(&state), 20.0);
    assert_eq!(state.tool_settings.font_size, 23.0);
}

/// 【字号输入测试】【目标切换】验证更换选中文字清除旧目标余量，纯小刻度和上限空操作不进入历史，无参数、无返回值
#[test]
fn wheel_target_switch_and_limit_match_real_qt() {
    let mut state = prepared(false);
    let mut dirty = 0;
    let now = std::time::Instant::now();
    let mut second = state.annotations[0].clone();
    second.id = 8;
    if let AnnotationShape::Text { font_size, .. } = &mut second.shape {
        *font_size = 40.0;
    }
    state.annotations.push(second);
    for index in [0, 1] {
        state.select_annotation_index(Some(index));
        crate::app::panels::text::size_wheel::canvas(
            &mut state,
            crate::types::ScrollEvent::wheel(60),
            now,
            &mut dirty,
        );
        assert_eq!(state.text_size_history.input.remainder(), 0.75);
    }
    assert_eq!(text_edits::current_points(&state), 40.0);
    assert!(state.text_size_history.pending.is_none());
    text_edits::set(&mut state, Property::Points(300.0), true, &mut dirty);
    state.undo_stack.clear();
    state.text_size_history.input.reset();
    for (delta, points, remainder) in [(120, 300.0, 0.5), (-120, 299.0, 0.0), (-120, 298.0, -0.5)] {
        crate::app::panels::text::size_wheel::canvas(
            &mut state,
            crate::types::ScrollEvent::wheel(delta),
            now,
            &mut dirty,
        );
        assert_eq!(text_edits::current_points(&state), points);
        assert_eq!(state.text_size_history.input.remainder(), remainder);
    }
    state.undo(&mut dirty);
    assert_eq!(text_edits::current_points(&state), 300.0);
    assert_eq!(state.tool_settings.font_size, 298.0);
}

/// 【字号输入测试】【后续操作】验证字号滚轮后独立样式修改不会合并到同一历史，默认字号不回退，无参数、无返回值
#[test]
fn a_new_property_change_commits_pending_wheel_before_its_own_history() {
    let mut state = prepared(false);
    let mut dirty = 0;
    crate::app::panels::text::size_wheel::canvas(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        std::time::Instant::now(),
        &mut dirty,
    );
    text_edits::set(&mut state, Property::Bold(true), true, &mut dirty);
    assert_eq!(state.undo_stack.len(), 2);
    state.undo(&mut dirty);
    assert_eq!(text_edits::current_points(&state), 21.0);
    assert!(matches!(
        state.annotations[0].shape,
        AnnotationShape::Text { bold: false, .. }
    ));
    state.undo(&mut dirty);
    assert_eq!(text_edits::current_points(&state), 20.0);
    assert_eq!(state.tool_settings.font_size, 21.0);
}

/// 【字号输入测试】【工具与拖动】没有选中对象时只改默认字号，正在拖动不修改字号，无参数、无返回值
#[test]
fn default_wheel_has_no_history_and_active_gestures_ignore_it() {
    let mut state = prepared(false);
    let mut dirty = 0;
    state.selected_tool = Tool::Text;
    state.select_annotation_index(None);
    let before = state.annotations.clone();
    crate::app::panels::text::size_wheel::canvas(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        std::time::Instant::now(),
        &mut dirty,
    );
    assert_eq!(state.tool_settings.font_size, 21.0);
    assert!(state.undo_stack.is_empty());
    assert!(state.text_size_history.pending.is_none());
    assert_eq!(state.annotations, before);
    state.input.mouse_down = true;
    crate::app::panels::text::size_wheel::canvas(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        std::time::Instant::now(),
        &mut dirty,
    );
    assert_eq!(state.tool_settings.font_size, 21.0);
    assert_eq!(state.text_size_history.input.remainder(), 0.5);
}

/// 【字号输入测试】【跨工具余量】其他工具收到滚轮后，返回文字工具不继承旧字号余量，无参数、无返回值
#[test]
fn another_tool_wheel_invalidates_the_old_text_context() {
    let mut state = prepared(false);
    let mut dirty = 0;
    state.selected_tool = Tool::Text;
    state.select_annotation_index(None);
    super::super::refresh_panels(&mut state, &mut dirty);
    pointer(&mut state, 300.0, 300.0, &mut dirty);
    super::super::handle_axis_scroll(&mut state, crate::types::ScrollEvent::wheel(60), &mut dirty);
    assert_eq!(state.text_size_history.input.remainder(), 0.75);
    super::super::select_tool(&mut state, Tool::Line, &mut dirty);
    super::super::handle_axis_scroll(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        &mut dirty,
    );
    super::super::select_tool(&mut state, Tool::Text, &mut dirty);
    super::super::handle_axis_scroll(&mut state, crate::types::ScrollEvent::wheel(60), &mut dirty);
    assert_eq!(state.text_size_history.input.remainder(), 0.75);
    assert_eq!(state.tool_settings.font_size, 20.0);
}

/// 【字号输入测试】【控件准备】按 name 准备 state 的实际控件坐标，dirty 返回界面范围；列表使用隔离目录与固定起始行
fn wheel_control(state: &mut EditorState, name: &str, dirty: &mut u32) {
    let rect = if name == "slider" {
        state
            .settings_panel
            .widget_rect(Control::Size.index())
            .unwrap()
    } else {
        state.font_panel.catalog = Some(crate::fonts::font_catalog::FontCatalog {
            families: (0..2500)
                .map(|index| crate::fonts::font_catalog::Family {
                    name: format!("Sample {index}"),
                    faces: Vec::new(),
                })
                .collect(),
        });
        crate::app::panels::text::font_popup::toggle(state, dirty);
        state.font_panel.offset = 100.0 * crate::ui::font_panel::layout::ROW_HEIGHT;
        let panel = &state.font_panel;
        let mut rect = if name == "spin" {
            panel.layout.size_field
        } else {
            panel.layout.list
        };
        if name == "bar" {
            rect = tiny_skia::Rect::from_xywh(
                rect.right() - 14.0,
                rect.top() + 14.0,
                14.0,
                rect.height() - 28.0,
            )
            .unwrap();
        }
        tiny_skia::Rect::from_xywh(
            rect.left() + panel.render_pos.0,
            rect.top() + panel.render_pos.1,
            rect.width(),
            rect.height(),
        )
        .unwrap()
    };
    pointer(
        state,
        (rect.left() + rect.width() / 2.0) as f64,
        (rect.top() + rect.height() / 2.0) as f64,
        dirty,
    );
}

/// 【字号输入测试】【原版差分】对照二十八组真实 Qt 记录，经生产输入分派核验字号、默认值、列表位置和画布余量；显式环境目录可保存实际结果
#[test]
fn actual_input_routes_match_frozen_qt_size_and_scroll_traces() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-text-size-inputs.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 28);
    let mut measured = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let control = case["control"].as_str().unwrap();
        let mut state = prepared(false);
        let mut dirty = 0;
        let now = std::time::Instant::now();
        text_edits::set(
            &mut state,
            Property::Points(case["initial_points"].as_f64().unwrap() as f32),
            true,
            &mut dirty,
        );
        state.undo_stack.clear();
        if control != "canvas" {
            wheel_control(&mut state, control, &mut dirty);
        }
        state.input.ctrl = case["ctrl"].as_bool().unwrap();
        let mut points = vec![text_edits::current_points(&state)];
        let mut defaults = vec![state.tool_settings.font_size];
        let mut remainders = vec![state.text_size_history.input.remainder()];
        let mut rows = vec![state.font_panel.offset / crate::ui::font_panel::layout::ROW_HEIGHT];
        for delta in case["deltas"].as_array().unwrap() {
            let delta = delta.as_i64().unwrap() as i32;
            let event = if case["pixels"].as_bool().unwrap() {
                crate::types::ScrollEvent::pixels(0.0, delta as f64)
            } else {
                crate::types::ScrollEvent::wheel(delta)
            };
            if control == "canvas" {
                crate::app::panels::text::size_wheel::canvas(&mut state, event, now, &mut dirty);
            } else {
                super::super::handle_axis_scroll(&mut state, event, &mut dirty);
            }
            points.push(text_edits::current_points(&state));
            defaults.push(state.tool_settings.font_size);
            remainders.push(state.text_size_history.input.remainder());
            rows.push(state.font_panel.offset / crate::ui::font_panel::layout::ROW_HEIGHT);
        }
        let expected = |key: &str| {
            case[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_f64().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            points.iter().map(|value| *value as f64).collect::<Vec<_>>(),
            expected("points"),
            "{name} points"
        );
        assert_eq!(
            defaults
                .iter()
                .map(|value| *value as f64)
                .collect::<Vec<_>>(),
            expected("defaults"),
            "{name} defaults"
        );
        if control == "canvas" {
            assert_eq!(remainders, expected("remainders"), "{name} remainders");
        }
        if matches!(control, "list" | "bar") {
            assert_eq!(
                rows.iter().map(|value| *value as f64).collect::<Vec<_>>(),
                expected("list_values"),
                "{name} rows"
            );
            assert_eq!(
                state.font_panel.layout.list.height() as f64,
                case["viewport_height"].as_f64().unwrap()
            );
        }
        measured.push(serde_json::json!({"name": name, "points": points, "defaults": defaults, "remainders": remainders, "list_values": rows}));
    }
    if let Ok(path) = std::env::var("MARK_SHOT_SIZE_TRACE_DIR") {
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            std::path::Path::new(&path).join("native-traces.json"),
            serde_json::to_string_pretty(&measured).unwrap(),
        )
        .unwrap();
    }
}
