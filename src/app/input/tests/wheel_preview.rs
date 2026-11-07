//! 【滚轮提示测试】【实际输入】验证工具数值提示、绘图保护及独立到期期限

use super::{editor, pointer};
use crate::{
    app::input::{handle_axis_scroll, handle_pointer_button, select_tool},
    tools::Tool,
    types::{MouseButton, ScrollEvent},
};
use tiny_skia::Rect;

/// 【滚轮提示测试】【真实工具滚轮】普通工具在指针旁显示当前数值，提示包含像素单位且没有新增标注
#[test]
fn every_width_tool_displays_actual_value_after_canvas_wheel() {
    for tool in [
        Tool::Pen,
        Tool::Highlighter,
        Tool::Rectangle,
        Tool::Circle,
        Tool::Line,
        Tool::Arrow,
        Tool::Marker,
        Tool::Ruler,
        Tool::Number,
        Tool::Redact,
    ] {
        let mut state = editor();
        let mut dirty = 0;
        state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
        select_tool(&mut state, tool, &mut dirty);
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
        let preview = state
            .navigation
            .preview
            .as_ref()
            .unwrap_or_else(|| panic!("no cursor value preview for {tool:?}"));
        assert!(preview.text.ends_with(" px"), "{tool:?}: {}", preview.text);
        assert!(preview.rect.left() > 320.0 && preview.rect.top() > 240.0);
        assert!(state.annotations.is_empty(), "{tool:?}");
    }
}

/// 【滚轮提示测试】【抓取保护】正在绘图时滚轮不生成提示，松开后提示到期可以独立清理，无额外鼠标事件
#[test]
fn preview_is_blocked_during_drawing_and_expires_without_pointer_motion() {
    let mut state = editor();
    let mut dirty = 0;
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    select_tool(&mut state, Tool::Pen, &mut dirty);
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    assert!(state.navigation.preview.is_none());
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    let deadline = state
        .navigation
        .preview
        .as_ref()
        .expect("released tool should show value")
        .deadline;
    crate::app::canvas_navigation::preview::tick(&mut state, deadline, &mut dirty);
    assert!(state.navigation.preview.is_none());
}

/// 【滚轮提示测试】【真实数值与跟随】编号显示气泡直径，文字显示点字号；移动提示不延长期限，进入界面后清除
#[test]
fn numeric_units_follow_actual_properties_and_pointer_without_extending_lifetime() {
    let mut state = editor();
    let mut dirty = 0;
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    select_tool(&mut state, Tool::Number, &mut dirty);
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    let width = state
        .tool_settings
        .stroke(crate::types::StrokeGroup::Number);
    let preview = state.navigation.preview.as_ref().unwrap();
    assert_eq!(
        preview.text,
        format!("{} px", (26.0 + width * 2.7).round() as i32)
    );
    let deadline = preview.deadline;
    pointer(&mut state, 400.0, 260.0, &mut dirty);
    let preview = state.navigation.preview.as_ref().unwrap();
    assert_eq!(preview.point, (400.0, 260.0));
    assert_eq!(preview.deadline, deadline);
    select_tool(&mut state, Tool::Text, &mut dirty);
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    let points = crate::editor::text_edits::current_points(&state);
    assert_eq!(
        state.navigation.preview.as_ref().unwrap().text,
        format!("{} pt", points.round() as i32)
    );
}
