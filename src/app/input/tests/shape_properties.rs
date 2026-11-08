//! 【框形测试】【参数链路】验证绘图比例、圆角编辑与撤销，不依赖显示服务器

use super::*;
use crate::{
    tools::Tool,
    types::{AnnotationShape, MouseButton},
};

/// 【框形测试】【绘图起点】以 tool 和 ratio 创建选区内绘图状态，返回尚未释放的草稿
fn drawing(tool: Tool, ratio: bool) -> EditorState {
    let mut state = editor();
    state.selected_tool = tool;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.tool_settings.fixed_ratio = ratio;
    state.tool_settings.corner_radius = 12.0;
    pointer(&mut state, 250.0, 250.0, &mut 0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 390.0, 300.0, &mut 0);
    state
}

/// 【框形测试】【长宽读取】从 state 草稿返回宽高，非框形草稿直接报告失败
fn dimensions(state: &EditorState) -> (f32, f32) {
    match &state.pending.as_ref().unwrap().shape {
        AnnotationShape::Rectangle { start, end, .. }
        | AnnotationShape::Circle { start, end, .. } => {
            ((end.0 - start.0).abs(), (end.1 - start.1).abs())
        }
        _ => panic!("expected a rectangle or ellipse"),
    }
}

/// 【框形测试】【临时约束】按下和释放 Shift 即时切换等比，固定一比一不受 Shift 释放影响
#[test]
fn shift_and_fixed_ratio_use_the_live_pointer() {
    for tool in [Tool::Rectangle, Tool::Circle] {
        let mut state = drawing(tool, false);
        assert_eq!(dimensions(&state), (140.0, 50.0));
        super::super::handle_modifiers(&mut state, false, true, &mut 0);
        assert_eq!(dimensions(&state), (140.0, 140.0));
        super::super::handle_modifiers(&mut state, false, false, &mut 0);
        assert_eq!(dimensions(&state), (140.0, 50.0));
        let fixed = drawing(tool, true);
        assert_eq!(dimensions(&fixed), (140.0, 140.0));
    }
}

/// 【框形测试】【历史恢复】圆角和比例改动进入标注历史，恢复原半径与原长宽后默认参数仍保留
#[test]
fn radius_and_ratio_are_undoable_and_export_scaling_preserves_pixels() {
    let mut state = drawing(Tool::Rectangle, false);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    state.selected_tool = Tool::Pick;
    state.select_annotation_index(Some(0));
    let original = state.annotations[0].clone();
    crate::editor::shape_edits::set_radius(&mut state, 25.0, true, &mut 0);
    crate::editor::shape_edits::set_ratio(&mut state, true, &mut 0);
    assert!(
        matches!(state.annotations[0].shape, AnnotationShape::Rectangle { radius: 25.0, fixed_ratio: true, start, end, .. } if (end.0-start.0).abs()==(end.1-start.1).abs())
    );
    let scaled = state.annotations[0].scaled(2.0);
    assert!(matches!(
        scaled.shape,
        AnnotationShape::Rectangle { radius: 25.0, .. }
    ));
    assert_eq!(
        scaled.style_units().canvas(25.0),
        state.annotations[0].style_units().canvas(25.0) * 2.0
    );
    state.undo(&mut 0);
    state.undo(&mut 0);
    assert_eq!(state.annotations[0], original);
    assert_eq!(state.tool_settings.corner_radius, 25.0);
}
