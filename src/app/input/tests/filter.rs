//! 【滤镜测试】【绘图链路】覆盖最小尺寸、取消释放、区域拖动、旋转和历史

use super::{filter_support::*, key, pointer};
use crate::app::input::{handle_modifiers, handle_pointer_button};
use crate::features::filter::{FilterMode, drag};
use crate::tools::Tool;
use crate::types::{MouseButton, SelectionHandle};

/// 【滤镜测试】【入口兼容】M 使用原版滤镜入口，X 保留旧入口，不改变默认强度或模式
#[test]
fn reference_and_legacy_filter_shortcuts_share_one_tool() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    assert_eq!(state.selected_tool, Tool::Redact);
    assert_eq!(state.tool_settings.strength, 14.0);
    assert_eq!(state.tool_settings.filter_mode, FilterMode::Pixelate);
    key(&mut state, "P", &mut dirty);
    key(&mut state, "X", &mut dirty);
    assert_eq!(state.selected_tool, Tool::Redact);
}

/// 【滤镜测试】【创建尺寸】矩形两条边都至少两个原图像素，反向拖动有效，右键不创建区域
#[test]
fn tiny_and_non_left_gestures_do_not_create_filters_or_history() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    for end in [(140.0, 120.0), (141.0, 180.0), (280.0, 121.0)] {
        draw(&mut state, (140.0, 120.0), end, &mut dirty);
        assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
    }
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, false, &mut dirty);
    assert!(state.pending.is_none());
    assert_eq!(state.selected_tool, crate::tools::Tool::Pick);
    key(&mut state, "M", &mut dirty);
    draw(&mut state, (280.0, 180.0), (140.0, 120.0), &mut dirty);
    let region = filter(&state, 0).rect().unwrap();
    assert_eq!(
        (region.left(), region.top(), region.width(), region.height()),
        (140.0, 120.0, 140.0, 60.0)
    );
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【滤镜测试】【取消缓存】Escape、撤销和工具切换释放未提交原图区域，不关闭会话或生成空历史
#[test]
fn canceled_drafts_release_pixels_without_committing_history() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    for cancel in ["Escape", "Ctrl+Z", "P"] {
        key(&mut state, "M", &mut dirty);
        pointer(&mut state, 140.0, 120.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 280.0, 180.0, &mut dirty);
        state.refresh_filters();
        assert!(state.filters.retained_bytes() > 0);
        key(&mut state, cancel, &mut dirty);
        assert_eq!(state.filters.retained_bytes(), 0, "{cancel}");
        assert!(
            state.pending.is_none() && state.annotations.is_empty() && state.undo_stack.is_empty()
        );
        assert!(!state.cancel);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    }
}

/// 【滤镜测试】【源图边界】原图外起点不创建区域，拖动终点限制在可用原图范围
#[test]
fn creation_requires_a_source_start_and_clamps_the_endpoint() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    draw(&mut state, (-10.0, 120.0), (280.0, 180.0), &mut dirty);
    assert!(state.annotations.is_empty());
    draw(&mut state, (140.0, 120.0), (900.0, 700.0), &mut dirty);
    assert_eq!(filter(&state, 0).end, (640.0, 480.0));
}

/// 【滤镜测试】【移动撤销】完整移动只生成一次历史，撤销和重做恢复源框、模式及强度
#[test]
fn move_undo_and_redo_restore_filter_geometry_and_style() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (280.0, 180.0), &mut dirty);
    let before = state.annotations.clone();
    pick(&mut state, 0, &mut dirty);
    draw(&mut state, (210.0, 150.0), (240.0, 170.0), &mut dirty);
    assert_eq!(filter(&state, 0).start, (170.0, 140.0));
    assert_eq!(state.undo_stack.len(), 2);
    let moved = state.annotations.clone();
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations, before);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(state.annotations, moved);
    key(&mut state, "Delete", &mut dirty);
    assert!(state.annotations.is_empty());
    assert_eq!(state.filters.retained_bytes(), 0);
}

/// 【滤镜测试】【旋转命中】正文外旋转把手由真实选择分派命中，Shift 吸附，Escape 恢复快照而非关闭会话
#[test]
fn selected_external_rotation_handle_has_priority_and_cancel_restores_it() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    draw(&mut state, (140.0, 140.0), (300.0, 260.0), &mut dirty);
    pick(&mut state, 0, &mut dirty);
    let before = state.annotations.clone();
    let region = filter(&state, 0);
    let start = drag::rotation_handle(region).unwrap();
    pointer(
        &mut state,
        f64::from(start.0),
        f64::from(start.1),
        &mut dirty,
    );
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert_eq!(
        state.ann_drag.as_ref().unwrap().handle,
        SelectionHandle::Rotate
    );
    handle_modifiers(&mut state, false, true, &mut dirty);
    let center = region.center();
    pointer(
        &mut state,
        f64::from(center.0 + 100.0),
        f64::from(center.1 - 100.0),
        &mut dirty,
    );
    assert!(state.pending.is_some());
    key(&mut state, "Escape", &mut dirty);
    assert_eq!(state.annotations, before);
    assert!(state.pending.is_none() && state.ann_drag.is_none() && !state.cancel);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.undo_stack.len(), 1);
}
