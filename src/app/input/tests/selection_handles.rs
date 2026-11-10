//! 【选区把手回归】【跨工具输入】验证八点在各工具和缩放画布中始终调整选区

use super::{editor, pointer};
use crate::{
    app::input::{handle_pointer_button, select_tool},
    tools::Tool,
    types::MouseButton,
};

/// 【选区把手回归】【光标边界】无参数，真实移动后只在八点显示缩放光标，选区外恢复箭头
#[test]
fn cursor_changes_only_at_handles_and_stays_visible_outside() {
    for tool in [Tool::Selection, Tool::Pen, Tool::Text, Tool::Eyedropper] {
        let mut state = editor();
        select_tool(&mut state, tool, &mut 0);
        state.selection.zone = tiny_skia::Rect::from_xywh(100.0, 100.0, 300.0, 200.0);
        for (point, expected) in [
            ((40.0, 200.0), crate::types::CursorIcon::Default),
            ((100.0, 200.0), crate::types::CursorIcon::EwResize),
            ((430.0, 200.0), crate::types::CursorIcon::Default),
        ] {
            pointer(&mut state, point.0, point.1, &mut 0);
            state.toolbar.opacity = 0.0;
            state.action_toolbar.opacity = 0.0;
            state.settings_panel.visible = false;
            assert_eq!(
                super::super::compute_cursor(&state),
                expected,
                "{tool:?} {point:?}"
            );
        }
        pointer(&mut state, 170.0, 100.0, &mut 0);
        state.toolbar.opacity = 0.0;
        state.action_toolbar.opacity = 0.0;
        state.settings_panel.visible = false;
        assert_ne!(
            super::super::compute_cursor(&state),
            crate::types::CursorIcon::NsResize
        );
    }
}

/// 【选区把手回归】【所有工具】无参数，逐项检查八点缩放保留工具、不创建标注，释放后结束手势
#[test]
fn all_tools_resize_from_all_eight_handles() {
    for tool in [
        Tool::Pick,
        Tool::Selection,
        Tool::Text,
        Tool::Pen,
        Tool::Arrow,
        Tool::Rectangle,
        Tool::Circle,
        Tool::Number,
        Tool::Magnifier,
        Tool::Eyedropper,
        Tool::Redact,
        Tool::Ruler,
        Tool::Highlighter,
        Tool::Marker,
        Tool::Laser,
        Tool::NumeratedArrow,
        Tool::Ocr,
    ] {
        for point in [
            (100.0, 100.0),
            (250.0, 100.0),
            (400.0, 100.0),
            (100.0, 200.0),
            (400.0, 200.0),
            (100.0, 300.0),
            (250.0, 300.0),
            (400.0, 300.0),
        ] {
            let mut state = editor();
            select_tool(&mut state, tool, &mut 0);
            let before = tiny_skia::Rect::from_xywh(100.0, 100.0, 300.0, 200.0).unwrap();
            state.selection.zone = Some(before);
            state.toolbar.opacity = 0.0;
            state.action_toolbar.opacity = 0.0;
            state.settings_panel.visible = false;
            pointer(&mut state, point.0, point.1, &mut 0);
            state.toolbar.opacity = 0.0;
            state.action_toolbar.opacity = 0.0;
            state.settings_panel.visible = false;
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
            pointer(&mut state, point.0 + 15.0, point.1 + 12.0, &mut 0);
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
            assert_ne!(
                state.selection.zone,
                Some(before),
                "tool={tool:?}, point={point:?}"
            );
            assert_eq!(state.selected_tool, tool);
            assert!(state.annotations.is_empty() && state.pending.is_none());
            assert!(!state.input.mouse_down && !state.tool_active);
        }
    }
}

/// 【全屏标注回归】【选区固定】无参数，全屏模式下八点及内部均不能拖动或清除选区，标注工具仍能正常绘制
#[test]
fn fullscreen_selection_has_no_resize_or_move_gestures() {
    let mut state = editor();
    let before = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    state.selection.zone = Some(before);
    state.fullscreen_scope = true;
    select_tool(&mut state, Tool::Selection, &mut 0);
    for (x, y) in [(0.0, 0.0), (320.0, 0.0), (639.0, 240.0), (300.0, 200.0)] {
        pointer(&mut state, x, y, &mut 0);
        state.toolbar.opacity = 0.0;
        state.action_toolbar.opacity = 0.0;
        state.settings_panel.visible = false;
        assert!(super::super::selection_handles::hit(&state).is_none());
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
        pointer(&mut state, x + 30.0, y + 20.0, &mut 0);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
        assert_eq!(state.selection.zone, Some(before));
        assert!(!state.tool_active);
    }
    select_tool(&mut state, Tool::Pen, &mut 0);
    pointer(&mut state, 150.0, 150.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 220.0, 180.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(state.selection.zone, Some(before));
}
