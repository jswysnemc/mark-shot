//! 【线条测试】【属性交互】验证样式、独立默认值、确认、关闭、滚动和空闲动画

use super::line_support::{choose, click, draw, item_center, line_editor, open, pick};
use super::{key, pointer, widget_center};
use crate::app::input::{compute_cursor, handle_modifiers, handle_scroll, select_tool};
use crate::app::panels;
use crate::editor::line_edits;
use crate::features::{arrow::ArrowStyle, line::LineStyle};
use crate::tools::Tool;
use crate::types::{AnnotationShape, CursorIcon, StrokeGroup};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget, StepperRange};
use crate::ui::{
    line_properties::LineChoice,
    panel::{AnimatedPanel, UiPanel},
};

/// 【线条测试】【属性按钮索引】返回 state 属性栏 action 对应按钮的位置；缺失时使测试失败
fn action_index(state: &crate::editor::EditorState, action: SettingsAction) -> usize {
    state.settings_panel.widgets.iter().position(|widget| matches!(widget, SettingsWidget::Action { action: value, .. } if *value == action)).unwrap()
}

/// 【线条测试】【选中属性】选中箭头仍显示两类独立样式，直线只显示线型，不再使用画笔属性
#[test]
fn picked_arrow_and_line_annotations_keep_their_own_property_widgets() {
    for tool in [Tool::Arrow, Tool::Line] {
        let mut dirty = 0;
        let mut state = line_editor(tool, &mut dirty);
        draw(&mut state, (140.0, 180.0), (420.0, 220.0), &mut dirty);
        pick(&mut state, 0, &mut dirty);
        action_index(&state, SettingsAction::LineStyles);
        assert_eq!(
            state.settings_panel.widgets.iter().any(|widget| matches!(
                widget,
                SettingsWidget::Action {
                    action: SettingsAction::ArrowStyles,
                    ..
                }
            )),
            tool == Tool::Arrow
        );
        assert!(state.settings_panel.widgets.iter().any(|widget| matches!(
            widget,
            SettingsWidget::Stepper {
                range: StepperRange::LineWidth,
                ..
            }
        )));
        if tool == Tool::Line {
            panels::line::toggle(&mut state, LineChoice::Heads, &mut dirty);
            assert!(!state.arrow_popover.open);
        }
    }
}

/// 【线条测试】【全部样式选择】全部头部和线型可用键盘确认，确认不导出，不产生默认值修改的空历史
#[test]
fn every_head_and_pattern_can_be_chosen_without_export_or_empty_history() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    for (index, expected) in ArrowStyle::ALL.into_iter().enumerate() {
        choose(&mut state, LineChoice::Heads, index, &mut dirty);
        assert_eq!(state.tool_settings.arrow_style, expected);
        assert_eq!(state.settings_panel.arrow_style, expected);
        assert!(!state.arrow_popover.open && state.finish.is_none());
    }
    for (index, expected) in LineStyle::ALL.into_iter().enumerate() {
        choose(&mut state, LineChoice::Pattern, index, &mut dirty);
        assert_eq!(state.tool_settings.arrow_line_style, expected);
        assert_eq!(state.tool_settings.line_style, LineStyle::Solid);
        assert_eq!(state.settings_panel.line_style, expected);
        assert!(!state.line_popover.open && state.finish.is_none());
    }
    choose(&mut state, LineChoice::Pattern, 1, &mut dirty);
    select_tool(&mut state, Tool::Line, &mut dirty);
    for (index, expected) in LineStyle::ALL.into_iter().enumerate() {
        choose(&mut state, LineChoice::Pattern, index, &mut dirty);
        assert_eq!(state.tool_settings.line_style, expected);
        assert_eq!(state.tool_settings.arrow_line_style, LineStyle::Dashed);
        assert!(state.finish.is_none());
    }
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
}

/// 【线条测试】【已有样式事务】选中标注修改头部和线型分别形成一次撤销，不改变后续工具的默认样式
#[test]
fn changing_existing_heads_and_patterns_does_not_override_future_defaults() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    draw(&mut state, (140.0, 180.0), (420.0, 220.0), &mut dirty);
    pick(&mut state, 0, &mut dirty);
    choose(&mut state, LineChoice::Heads, 3, &mut dirty);
    choose(&mut state, LineChoice::Pattern, 2, &mut dirty);
    assert!(matches!(
        state.annotations[0].shape,
        AnnotationShape::Arrow {
            style: ArrowStyle::BidirectionalKde,
            line_style: LineStyle::Dotted,
            ..
        }
    ));
    assert_eq!(state.tool_settings.arrow_style, ArrowStyle::Fletched);
    assert_eq!(state.tool_settings.arrow_line_style, LineStyle::Solid);
    assert_eq!(state.tool_settings.line_style, LineStyle::Solid);
    assert_eq!(state.undo_stack.len(), 3);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(line_edits::current_style(&state), LineStyle::Solid);
    assert_eq!(
        line_edits::current_head(&state),
        ArrowStyle::BidirectionalKde
    );
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(line_edits::current_head(&state), ArrowStyle::Fletched);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(line_edits::current_style(&state), LineStyle::Dotted);
    choose(&mut state, LineChoice::Heads, 3, &mut dirty);
    choose(&mut state, LineChoice::Pattern, 2, &mut dirty);
    assert_eq!(state.undo_stack.len(), 3);
}

/// 【线条测试】【按钮切换】按钮打开和关闭只处理一次，两类弹层互斥，弹层内单击选择不会传到画布
#[test]
fn toggles_switch_between_popovers_and_inside_clicks_never_start_a_line() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    let head = widget_center(&state, action_index(&state, SettingsAction::ArrowStyles));
    click(&mut state, head, &mut dirty);
    assert!(state.arrow_popover.open);
    state.arrow_popover.animate_step(1.0);
    click(&mut state, head, &mut dirty);
    assert!(!state.arrow_popover.open);
    click(&mut state, head, &mut dirty);
    assert!(state.arrow_popover.open);
    let pattern = widget_center(&state, action_index(&state, SettingsAction::LineStyles));
    click(&mut state, pattern, &mut dirty);
    assert!(!state.arrow_popover.open && state.line_popover.open);
    state.arrow_popover.animate_step(1.0);
    state.line_popover.animate_step(1.0);
    panels::line::update(&mut state, &mut dirty);
    let item = item_center(&state, LineChoice::Pattern, 1);
    pointer(&mut state, item.0, item.1, &mut dirty);
    assert_eq!(state.line_popover.hovered, Some(1));
    assert_eq!(compute_cursor(&state), CursorIcon::Pointer);
    click(&mut state, item, &mut dirty);
    assert_eq!(state.tool_settings.arrow_line_style, LineStyle::Dashed);
    assert!(!state.line_popover.open);
    assert!(state.pending.is_none() && state.annotations.is_empty());
    assert!(state.undo_stack.is_empty());
}

/// 【线条测试】【输入优先级】弹层打开时滚轮不修改笔宽，Escape 只关闭弹层，Enter 只确认而不导出
#[test]
fn scroll_escape_and_enter_are_consumed_before_canvas_or_export_actions() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    for kind in LineChoice::ALL {
        open(&mut state, kind, &mut dirty);
        pointer(&mut state, 560.0, 360.0, &mut dirty);
        handle_scroll(&mut state, 0.0, -6.0, &mut dirty);
        assert_eq!(state.tool_settings.stroke(StrokeGroup::Arrow), 3.0);
        key(&mut state, "Escape", &mut dirty);
        assert!(!panels::line::popup(&state, kind).open && !state.cancel);
        open(&mut state, kind, &mut dirty);
        key(&mut state, "End", &mut dirty);
        key(&mut state, "Enter", &mut dirty);
        assert!(!panels::line::popup(&state, kind).open && state.finish.is_none());
    }
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
}

/// 【线条测试】【宽度输入】箭头和直线接受七十二像素宽度，文本确认和撤销不修改其他笔宽分组、不提前导出
#[test]
fn line_width_text_edits_reach_the_reference_limit_in_the_correct_group() {
    for tool in [Tool::Arrow, Tool::Line] {
        let mut dirty = 0;
        let mut state = line_editor(tool, &mut dirty);
        draw(&mut state, (140.0, 180.0), (420.0, 220.0), &mut dirty);
        pick(&mut state, 0, &mut dirty);
        let index = state
            .settings_panel
            .widgets
            .iter()
            .position(|widget| {
                matches!(
                    widget,
                    SettingsWidget::Stepper {
                        range: StepperRange::LineWidth,
                        ..
                    }
                )
            })
            .unwrap();
        let center = widget_center(&state, index);
        click(&mut state, (center.0 - 12.0, center.1), &mut dirty);
        handle_modifiers(&mut state, true, false, &mut dirty);
        key(&mut state, "Ctrl+A", &mut dirty);
        crate::app::input::handle_key(&mut state, None, Some("72".to_string()), &mut dirty);
        handle_modifiers(&mut state, false, false, &mut dirty);
        key(&mut state, "Enter", &mut dirty);
        assert_eq!(state.annotations[0].stroke_width, 72.0);
        assert_eq!(state.tool_settings.stroke(StrokeGroup::Numbered), 11.0);
        assert_eq!(state.tool_settings.stroke(StrokeGroup::Freehand), 7.0);
        assert_eq!(
            state.tool_settings.stroke(if tool == Tool::Arrow {
                StrokeGroup::Line
            } else {
                StrokeGroup::Arrow
            }),
            3.0
        );
        assert!(state.finish.is_none());
        assert_eq!(state.undo_stack.len(), 2);
        key(&mut state, "Ctrl+Z", &mut dirty);
        assert_eq!(state.annotations[0].stroke_width, 3.0);
    }
}

/// 【线条测试】【外部生命周期】隐藏界面、工具切换、打开颜色或模型弹层都会关闭线条弹层，关闭颜色不反向关闭新弹层
#[test]
fn hiding_switching_and_other_popovers_close_line_properties_in_one_direction() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    open(&mut state, LineChoice::Heads, &mut dirty);
    crate::app::actions::run(&mut state, crate::keys::Action::ToggleUi, &mut dirty);
    assert!(state.ui_hidden && !state.arrow_popover.open);
    crate::app::actions::run(&mut state, crate::keys::Action::ToggleUi, &mut dirty);
    open(&mut state, LineChoice::Pattern, &mut dirty);
    select_tool(&mut state, Tool::Line, &mut dirty);
    assert!(!state.line_popover.open && !state.arrow_popover.open);
    open(&mut state, LineChoice::Pattern, &mut dirty);
    panels::color::switch_color_popover_to(
        &mut state,
        crate::ui::color_popover::ColorTarget::Main,
        &mut dirty,
    );
    assert!(!state.line_popover.open);
    open(&mut state, LineChoice::Pattern, &mut dirty);
    panels::color::close_color_popover(&mut state, &mut dirty);
    assert!(state.line_popover.open);
    panels::model::toggle_model_popover(&mut state, &mut dirty);
    assert!(!state.line_popover.open && state.model_popover.open);
}

/// 【线条测试】【快捷键透传】Ctrl 加 Z 先关闭弹层再撤销持久标注，不能只关闭弹层吞掉历史动作
#[test]
fn modified_history_keys_close_the_popover_and_continue_to_the_history_action() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    draw(&mut state, (140.0, 180.0), (420.0, 220.0), &mut dirty);
    open(&mut state, LineChoice::Heads, &mut dirty);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert!(!state.arrow_popover.open && state.annotations.is_empty());
    assert_eq!(state.redo_stack.len(), 1);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(state.annotations.len(), 1);
}

/// 【线条测试】【显示器命中】本地坐标相同但显示器不同的输入不能命中打开的头部网格
#[test]
fn popovers_reject_matching_local_coordinates_on_another_monitor() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    open(&mut state, LineChoice::Heads, &mut dirty);
    state.input.pointer.local = item_center(&state, LineChoice::Heads, 1);
    state.input.pointer.monitor_idx = state.arrow_popover.monitor_idx + 1;
    assert!(matches!(
        crate::app::input::ui_hit::hit_test_ui(&state, state.input.pointer.local),
        crate::app::input::ui_hit::UiHit::LinePopoverOutside(LineChoice::Heads)
    ));
}

/// 【线条测试】【动画空闲】两类面板打开或关闭完成后恢复无限等待，弹层动画不能留下持续刷新
#[test]
fn line_popover_animations_return_the_event_loop_to_blocking_waits() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    state.toolbar.render_pos = state.toolbar.position;
    for kind in LineChoice::ALL {
        panels::line::toggle(&mut state, kind, &mut dirty);
        assert!(crate::app::event_loop::poll_timeout(&state).is_some());
        match kind {
            LineChoice::Heads => {
                state.arrow_popover.animate_step(1.0);
            }
            LineChoice::Pattern => {
                state.line_popover.animate_step(1.0);
            }
        }
        assert_eq!(crate::app::event_loop::poll_timeout(&state), None);
        panels::line::close(&mut state, &mut dirty);
        assert!(crate::app::event_loop::poll_timeout(&state).is_some());
        state.arrow_popover.animate_step(1.0);
        state.line_popover.animate_step(1.0);
        assert_eq!(crate::app::event_loop::poll_timeout(&state), None);
        assert!(panels::line::popup(&state, kind).rect().is_none());
    }
}

/// 【线条测试】【窄界面布局】网格按显示器可用范围缩小，键盘仍可访问最后一项，不依赖固定桌面尺寸
#[test]
fn narrow_monitor_layouts_keep_every_option_accessible() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    state.placements[0].size = (180, 160);
    for kind in LineChoice::ALL {
        open(&mut state, kind, &mut dirty);
        let panel = panels::line::popup(&state, kind);
        assert!(panel.layout.size.0 <= 180.0 - crate::theme::size::margin() * 2.0);
        assert!(panel.layout.size.1 <= 160.0 - crate::theme::size::margin() * 2.0);
        key(&mut state, "End", &mut dirty);
        let panel = panels::line::popup(&state, kind);
        assert!(
            panel
                .layout
                .item_rect(panel.items().len() - 1, panel.first_row)
                .is_some()
        );
        key(&mut state, "Enter", &mut dirty);
        assert!(!panels::line::popup(&state, kind).open && state.finish.is_none());
    }
}
