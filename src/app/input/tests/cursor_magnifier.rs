//! 【光标放大区测试】【显示阶段】静止编辑、初始选区、调整、取色及面板阶段使用真实编辑器状态

use super::*;

/// 【光标放大区测试】【编辑阶段】无参数，验证初始框选显示，完成后移动或缩放选区均隐藏，重新框选时恢复预览
#[test]
fn cursor_magnifier_follows_selection_phase_instead_of_only_enabled_flag() {
    let mut state = editor();
    state.magnifier.hidden = false;
    let mut dirty = 0;
    pointer(&mut state, 300.0, 230.0, &mut dirty);
    assert!(state.magnifier_shown());
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    pointer(&mut state, 450.0, 340.0, &mut dirty);
    assert!(state.magnifier_shown());
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert!(
        !state.magnifier_shown(),
        "completed selection must hide the cursor magnifier"
    );
    for (point, handle) in [
        ((300.0, 230.0), crate::types::SelectionHandle::TopLeft),
        ((375.0, 285.0), crate::types::SelectionHandle::Move),
    ] {
        pointer(&mut state, point.0, point.1, &mut dirty);
        super::super::handle_pointer_button(
            &mut state,
            crate::types::MouseButton::Left,
            true,
            &mut dirty,
        );
        assert_eq!(state.selection.active_handle, handle);
        pointer(&mut state, point.0 + 10.0, point.1 + 10.0, &mut dirty);
        assert!(!state.magnifier_shown());
        super::super::handle_pointer_button(
            &mut state,
            crate::types::MouseButton::Left,
            false,
            &mut dirty,
        );
        assert!(!state.magnifier_shown());
    }
    pointer(&mut state, 70.0, 50.0, &mut dirty);
    assert!(!state.magnifier_shown());
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    assert!(state.magnifier_shown());
    pointer(&mut state, 250.0, 180.0, &mut dirty);
    assert!(state.selection.creating && state.magnifier_shown());
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert!(!state.magnifier_shown());
}

/// 【光标放大区测试】【工具阶段】普通标注与全图编辑不显示光标放大区，取色保持独立职责
#[test]
fn drawing_and_fullscreen_do_not_show_loupe_but_eyedropper_does() {
    let mut state = editor();
    state.magnifier.hidden = false;
    let mut dirty = 0;
    super::super::select_tool(&mut state, crate::tools::Tool::Pen, &mut dirty);
    pointer(&mut state, 300.0, 230.0, &mut dirty);
    assert!(!state.magnifier_shown());
    super::super::select_tool(&mut state, crate::tools::Tool::Selection, &mut dirty);
    state.fullscreen_scope = true;
    assert!(!state.magnifier_shown());
    super::super::select_tool(&mut state, crate::tools::Tool::Eyedropper, &mut dirty);
    assert!(state.magnifier_shown());
}

/// 【光标放大区测试】【显隐清理】初始圆环隐藏时完整旧范围进入损坏，静止状态不重复提交损坏或增加定时任务
#[test]
fn hiding_the_loupe_damages_previous_pixels_once() {
    let mut state = editor();
    state.magnifier.hidden = false;
    let mut dirty = 0;
    pointer(&mut state, 300.0, 230.0, &mut dirty);
    crate::app::cursor_magnifier::sync(&mut state, &mut dirty);
    assert!(state.magnifier.visible);
    let point = state.magnifier.current.unwrap();
    state.magnifier.prev = Some(point);
    let old = crate::ui::magnifier::magnifier_rect(
        (point.pos.0 as f32, point.pos.1 as f32),
        640.0,
        480.0,
    );
    state.damage_rects.clear();
    dirty = 0;
    state.selection.zone = tiny_skia::Rect::from_xywh(100.0, 100.0, 100.0, 80.0);
    crate::app::cursor_magnifier::sync(&mut state, &mut dirty);
    assert!(!state.magnifier.visible && dirty != 0);
    assert!(state.damage_rects.iter().any(|damage| matches!(damage, crate::editor::DamageZone::Local { monitor_idx: 0, rect }
        if rect.left() < old.left() && rect.top() < old.top() && rect.right() > old.right() && rect.bottom() > old.bottom())));
    state.damage_rects.clear();
    dirty = 0;
    crate::app::cursor_magnifier::sync(&mut state, &mut dirty);
    assert!(state.damage_rects.is_empty() && dirty == 0);
}
