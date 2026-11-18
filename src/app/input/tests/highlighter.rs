//! 【荧光笔测试】【绘制模式】真实工具手势验证直线、自由笔迹、角度约束与属性切换

use super::*;
use crate::{
    features::highlighter::HighlighterStyle,
    tools::Tool,
    types::{AnnotationShape, MouseButton, StrokeGroup},
};

/// 【荧光笔测试】【输入准备】无参数，返回启用荧光笔的真实编辑状态，避免属性栏截获画布采样
fn prepared() -> EditorState {
    let mut state = editor();
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.selection.prev_zone = state.selection.zone;
    state.ui_hidden = true;
    super::super::select_tool(&mut state, Tool::Highlighter, &mut 0);
    state
}

/// 【荧光笔测试】【默认直线】无参数，独立默认笔宽为六，多个采样只保留初始点和最新终点，无返回值
#[test]
fn default_highlighter_draws_one_straight_line_at_its_independent_width() {
    let mut state = prepared();
    assert_eq!(
        state.tool_settings.highlighter_style,
        HighlighterStyle::StraightLine
    );
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Highlighter), 6.0);
    pointer(&mut state, 100.0, 200.0, &mut 0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    for point in [(160.0, 270.0), (210.0, 210.0), (300.0, 240.0)] {
        pointer(&mut state, point.0, point.1, &mut 0);
    }
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let AnnotationShape::Highlighter { points, flat_ends } = &state.annotations[0].shape else {
        panic!("highlighter expected");
    };
    assert_eq!(points, &[(100.0, 200.0), (300.0, 240.0)]);
    assert_eq!(*flat_ends, [false; 2]);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【荧光笔测试】【自由模式】无参数，自由笔迹保留多个采样，Shift 不把自由模式变为直线，无返回值
#[test]
fn freehand_highlighter_keeps_samples_while_shift_is_held() {
    let mut state = prepared();
    state.tool_settings.highlighter_style = HighlighterStyle::Freehand;
    state.input.shift = true;
    pointer(&mut state, 100.0, 200.0, &mut 0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    for point in [(160.0, 270.0), (210.0, 210.0), (300.0, 240.0)] {
        pointer(&mut state, point.0, point.1, &mut 0);
    }
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let AnnotationShape::Highlighter { points, .. } = &state.annotations[0].shape else {
        panic!("highlighter expected");
    };
    assert_eq!(points.len(), 4);
}
