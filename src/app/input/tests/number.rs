//! 【序号测试】【输入链路】覆盖独立序号、连续计数、属性面板、撤销和控制点取消

use super::{editor, key, pointer, widget_center};
use crate::app::input::{handle_modifiers, handle_pointer_button, handle_scroll, select_tool};
use crate::app::panels;
use crate::editor::{EditorState, number_edits};
use crate::features::number::{NumberMark, NumberStyle, drag};
use crate::tools::Tool;
use crate::types::{AnnotationShape, MouseButton, SelectionHandle, StrokeGroup};
use crate::ui::panel::AnimatedPanel;
use crate::ui::settings_panel::{SettingsAction, SettingsWidget, StepperRange};

/// 【序号测试】【编辑器构造】返回已选择独立序号工具的编辑器；dirty 接收初始化重绘，笔宽和样式使用明确值
fn number_editor(dirty: &mut u32) -> EditorState {
    let mut state = editor();
    state.tool_settings.number_style = NumberStyle::Arabic;
    state.tool_settings.set_stroke(StrokeGroup::Number, 3.0);
    state.tool_settings.color = tiny_skia::Color::from_rgba8(220, 40, 50, 255);
    key(&mut state, "N", dirty);
    state.toolbar.opacity = 1.0;
    state.toolbar.render_pos = state.toolbar.position;
    panels::settings::update_settings_panel(&mut state, dirty);
    state
}

/// 【序号测试】【完成绘图】在 state 中从 bubble 拖到 tip 后提交；dirty 接收重绘，无返回值
fn draw(state: &mut EditorState, bubble: (f64, f64), tip: (f64, f64), dirty: &mut u32) {
    pointer(state, bubble.0, bubble.1, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    pointer(state, tip.0, tip.1, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【序号测试】【标记读取】返回 state 中 index 对应的独立序号，不接受旧编号箭头
fn mark(state: &EditorState, index: usize) -> NumberMark {
    let AnnotationShape::Number { mark } = state.annotations[index].shape else {
        panic!("number expected")
    };
    mark
}

/// 【序号测试】【弹层打开】打开 state 的样式面板并完成透明度动画；dirty 接收重绘，无返回值
fn open(state: &mut EditorState, dirty: &mut u32) {
    panels::number::toggle(state, dirty);
    state.number_popover.animate_step(1.0);
    panels::number::update(state, dirty);
}

/// 【序号测试】【快捷键兼容】N 进入原版独立序号，旧编号箭头保留 Shift 加 N，笔宽分组编号不重排
#[test]
fn standalone_numbers_and_legacy_numbered_arrows_have_distinct_entry_points() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    assert_eq!(state.selected_tool, Tool::Number);
    assert_eq!(StrokeGroup::Number as usize, 10);
    assert_eq!(StrokeGroup::Numbered as usize, 4);
    key(&mut state, "Shift+N", &mut dirty);
    assert_eq!(state.selected_tool, Tool::NumeratedArrow);
    assert_eq!(state.number_sequence.next(), Some(1));
}

/// 【序号测试】【创建与递增】单击生成圆框，拖动生成指向尾部，编号只在提交后推进
#[test]
fn click_and_drag_commit_numbers_with_reference_point_order() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (140.0, 120.0), &mut dirty);
    assert_eq!(mark(&state, 0).bubble, (140.0, 120.0));
    assert_eq!(mark(&state, 0).tip, (140.0, 120.0));
    assert_eq!(mark(&state, 0).number, 1);
    assert_eq!(state.number_sequence.next(), Some(2));
    draw(&mut state, (280.0, 160.0), (370.0, 180.0), &mut dirty);
    assert_eq!(mark(&state, 1).bubble, (280.0, 160.0));
    assert_eq!(mark(&state, 1).tip, (370.0, 180.0));
    assert_eq!(mark(&state, 1).number, 2);
    assert_eq!(state.number_sequence.next(), Some(3));
    assert_eq!(state.undo_stack.len(), 2);
}

/// 【序号测试】【草稿取消】Escape、撤销和工具切换取消草稿，不消耗编号、不关闭会话，右键不创建序号
#[test]
fn canceled_drafts_do_not_consume_numbers_or_close_the_session() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    for cancel in ["Escape", "Ctrl+Z", "P"] {
        key(&mut state, "N", &mut dirty);
        pointer(&mut state, 140.0, 120.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 280.0, 160.0, &mut dirty);
        assert!(state.pending.is_some());
        assert_eq!(state.number_sequence.next(), Some(1));
        key(&mut state, cancel, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert!(state.pending.is_none());
        assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
        assert_eq!(state.number_sequence.next(), Some(1));
        assert!(!state.cancel);
    }
    key(&mut state, "N", &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, false, &mut dirty);
    assert!(state.annotations.is_empty());
}

/// 【序号测试】【提交撤销】撤销和重做同时恢复持久序号与下一编号，新分支不会复用错误编号
#[test]
fn undo_and_redo_restore_sequence_with_annotation_state() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (140.0, 120.0), &mut dirty);
    draw(&mut state, (280.0, 160.0), (370.0, 180.0), &mut dirty);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(state.number_sequence.next(), Some(2));
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(state.annotations.len(), 2);
    assert_eq!(state.number_sequence.next(), Some(3));
    key(&mut state, "Ctrl+Z", &mut dirty);
    draw(&mut state, (340.0, 240.0), (400.0, 270.0), &mut dirty);
    assert_eq!(mark(&state, 1).number, 2);
    assert_eq!(state.number_sequence.next(), Some(3));
    assert!(state.redo_stack.is_empty());
}

/// 【序号测试】【删除空缺】删除旧序号不自动填补编号空缺，后续序号继续按会话计数递增
#[test]
fn deleting_old_numbers_does_not_fill_sequence_gaps() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (140.0, 120.0), &mut dirty);
    draw(&mut state, (280.0, 160.0), (370.0, 180.0), &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.select_annotation_index(Some(0));
    key(&mut state, "Delete", &mut dirty);
    assert_eq!(state.number_sequence.next(), Some(3));
    key(&mut state, "N", &mut dirty);
    draw(&mut state, (340.0, 240.0), (400.0, 270.0), &mut dirty);
    assert_eq!(mark(&state, 1).number, 3);
}

/// 【序号测试】【重置撤销】重置保留全部标注，单独形成可撤销事务；重复重置不产生空历史
#[test]
fn resetting_the_sequence_is_an_undoable_non_annotation_transaction() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (140.0, 120.0), &mut dirty);
    let before = state.annotations.clone();
    number_edits::reset_sequence(&mut state, &mut dirty);
    assert_eq!(state.annotations, before);
    assert_eq!(state.number_sequence.next(), Some(1));
    assert_eq!(state.undo_stack.len(), 2);
    number_edits::reset_sequence(&mut state, &mut dirty);
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.number_sequence.next(), Some(2));
    assert_eq!(state.annotations, before);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(state.number_sequence.next(), Some(1));
}

/// 【序号测试】【全部样式】七个面板选项都可用键盘确认，确认不导出且默认值变化不产生标注历史
#[test]
fn every_number_style_can_be_chosen_without_exporting_or_empty_history() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    for (index, expected) in NumberStyle::ALL.into_iter().enumerate() {
        open(&mut state, &mut dirty);
        key(&mut state, "Home", &mut dirty);
        for _ in 0..index {
            key(&mut state, "Right", &mut dirty);
        }
        key(&mut state, "Enter", &mut dirty);
        assert_eq!(state.tool_settings.number_style, expected);
        assert_eq!(state.settings_panel.number_style, expected);
        assert!(!state.number_popover.open && state.finish.is_none());
    }
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
}

/// 【序号测试】【已有样式】修改选中序号形成单次撤销，不改变后续绘图样式或下一编号
#[test]
fn editing_existing_styles_does_not_change_future_style_or_sequence() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (140.0, 120.0), &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.select_annotation_index(Some(0));
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    open(&mut state, &mut dirty);
    key(&mut state, "End", &mut dirty);
    key(&mut state, "Enter", &mut dirty);
    assert_eq!(mark(&state, 0).style, NumberStyle::Chinese);
    assert_eq!(state.tool_settings.number_style, NumberStyle::Arabic);
    assert_eq!(state.number_sequence.next(), Some(2));
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(mark(&state, 0).style, NumberStyle::Arabic);
    assert_eq!(state.settings_panel.number_style, NumberStyle::Arabic);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(mark(&state, 0).style, NumberStyle::Chinese);
    assert_eq!(state.number_sequence.next(), Some(2));
}

/// 【序号测试】【独立尺寸】序号上限为七十二像素，不复用普通笔宽上限，属性确认和撤销不修改其他分组
#[test]
fn number_width_accepts_seventy_two_pixels_without_changing_other_tools() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    state.tool_settings.set_stroke(StrokeGroup::Numbered, 11.0);
    state.tool_settings.set_stroke(StrokeGroup::Freehand, 7.0);
    draw(&mut state, (140.0, 120.0), (140.0, 120.0), &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.select_annotation_index(Some(0));
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Stepper {
                    range: StepperRange::NumberWidth,
                    ..
                }
            )
        })
        .unwrap();
    let center = widget_center(&state, index);
    pointer(&mut state, center.0 - 12.0, center.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    handle_modifiers(&mut state, true, false, &mut dirty);
    key(&mut state, "Ctrl+A", &mut dirty);
    crate::app::input::handle_key(&mut state, None, Some("72".to_string()), &mut dirty);
    handle_modifiers(&mut state, false, false, &mut dirty);
    key(&mut state, "Enter", &mut dirty);
    assert_eq!(state.annotations[0].stroke_width, 72.0);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Numbered), 11.0);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Freehand), 7.0);
    assert_eq!(state.number_sequence.next(), Some(2));
    assert!(state.finish.is_none());
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations[0].stroke_width, 3.0);
    assert_eq!(state.number_sequence.next(), Some(2));
}

/// 【序号测试】【打开按钮】已打开面板再次点击样式按钮只关闭一次，滚轮和 Escape 不影响编号与笔宽
#[test]
fn popover_toggle_scroll_and_escape_are_isolated_from_annotation_input() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Action {
                    action: SettingsAction::NumberStyles,
                    ..
                }
            )
        })
        .unwrap();
    let center = widget_center(&state, index);
    pointer(&mut state, center.0, center.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.number_popover.open);
    state.number_popover.animate_step(1.0);
    handle_scroll(&mut state, 0.0, -3.0, &mut dirty);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Number), 3.0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(!state.number_popover.open);
    open(&mut state, &mut dirty);
    key(&mut state, "Escape", &mut dirty);
    assert!(!state.number_popover.open && !state.cancel);
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
}

/// 【序号测试】【拖动取消】已有序号圆心拖动暂存后，Escape 恢复原始几何和计数，不关闭编辑器
#[test]
fn escape_restores_an_existing_number_drag_without_consuming_a_counter() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (280.0, 160.0), &mut dirty);
    let before = state.annotations[0].clone();
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.select_annotation_index(Some(0));
    pointer(&mut state, 140.0, 120.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 190.0, 180.0, &mut dirty);
    assert!(state.pending.is_some() && state.ann_drag.is_some());
    key(&mut state, "Escape", &mut dirty);
    assert_eq!(state.annotations, [before]);
    assert!(state.pending.is_none() && state.ann_drag.is_none());
    assert!(!state.cancel);
    assert_eq!(state.number_sequence.next(), Some(2));
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【序号测试】【外部控制点】已选序号的外框和旋转把手在正文之外仍须开始正确拖动，不能取消选择或撤销别的标注
#[test]
fn selected_number_handles_outside_the_body_start_real_annotation_drags() {
    for expected in [SelectionHandle::TopLeft, SelectionHandle::Rotate] {
        let mut dirty = 0;
        let mut state = number_editor(&mut dirty);
        draw(&mut state, (140.0, 120.0), (280.0, 160.0), &mut dirty);
        let before = state.annotations.clone();
        select_tool(&mut state, Tool::Pick, &mut dirty);
        state.select_annotation_index(Some(0));
        let position = if expected == SelectionHandle::Rotate {
            drag::rotation_handle(mark(&state, 0), 3.0).unwrap()
        } else {
            drag::controls(mark(&state, 0), 3.0)
                .into_iter()
                .find(|(handle, _)| *handle == expected)
                .unwrap()
                .1
        };
        let position = if expected == SelectionHandle::TopLeft {
            (position.0 - 9.0, position.1 - 9.0)
        } else {
            position
        };
        assert!(!state.annotations[0].initial_hit_test((position.0 as f64, position.1 as f64)));
        pointer(&mut state, position.0 as f64, position.1 as f64, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert_eq!(
            state.ann_drag.as_ref().map(|drag| drag.handle),
            Some(expected)
        );
        pointer(
            &mut state,
            position.0 as f64 + 25.0,
            position.1 as f64 - 20.0,
            &mut dirty,
        );
        assert!(state.pending.is_some());
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.annotations.len(), 1);
        assert_ne!(state.annotations, before);
        let changed = state.annotations.clone();
        key(&mut state, "Ctrl+Z", &mut dirty);
        assert_eq!(state.annotations, before);
        key(&mut state, "Ctrl+Shift+Z", &mut dirty);
        assert_eq!(state.annotations, changed);
        assert_eq!(state.number_sequence.next(), Some(2));
    }
}

/// 【序号测试】【首次拖动】首次选中圆心并拖动时只移动圆心，不把实际尖端作为整体移动的一部分
#[test]
fn selecting_a_number_and_dragging_its_bubble_preserves_the_tip() {
    let mut dirty = 0;
    let mut state = number_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (280.0, 160.0), &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    assert_eq!(state.selected_annotation_index(), None);
    pointer(&mut state, 140.0, 120.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert_eq!(
        state.ann_drag.as_ref().map(|drag| drag.handle),
        Some(SelectionHandle::NumberBubble)
    );
    pointer(&mut state, 190.0, 180.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(mark(&state, 0).bubble, (190.0, 180.0));
    assert_eq!(
        mark(&state, 0).to_visual(mark(&state, 0).tip),
        (280.0, 160.0)
    );
    assert_eq!(state.number_sequence.next(), Some(2));
}
