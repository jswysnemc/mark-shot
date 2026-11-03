//! 【滤镜测试】【属性输入】验证全部模式、重复点击、默认隔离、数值编辑和窄界面

use super::{filter_support::*, key, pointer, widget_center};
use crate::app::{
    input::{handle_key, handle_modifiers, handle_pointer_button, handle_scroll},
    panels,
};
use crate::editor::filter_edits;
use crate::features::filter::FilterMode;
use crate::types::MouseButton;
use crate::ui::panel::{AnimatedPanel, UiPanel};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget, StepperRange};

/// 【滤镜测试】【模式完整】五个模式都可以键盘确认，确认不会导出或生成无标注历史
#[test]
fn every_filter_mode_can_be_confirmed_without_exporting() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    for (index, mode) in FilterMode::ALL.into_iter().enumerate() {
        choose(&mut state, index, &mut dirty);
        assert_eq!(state.tool_settings.filter_mode, mode);
        assert_eq!(state.settings_panel.filter_mode, mode);
        assert!(!state.filter_popover.open && state.finish.is_none() && !state.cancel);
    }
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
}

/// 【滤镜测试】【已有模式事务】修改选中区域只生成一次历史，不覆盖后续默认模式，重复选择不产生空历史
#[test]
fn selected_mode_changes_are_undoable_and_do_not_overwrite_defaults() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (280.0, 180.0), &mut dirty);
    pick(&mut state, 0, &mut dirty);
    choose(&mut state, FilterMode::Invert as usize, &mut dirty);
    assert_eq!(filter(&state, 0).mode, FilterMode::Invert);
    assert_eq!(state.tool_settings.filter_mode, FilterMode::Pixelate);
    assert_eq!(state.undo_stack.len(), 2);
    choose(&mut state, FilterMode::Invert as usize, &mut dirty);
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(filter(&state, 0).mode, FilterMode::Pixelate);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(filter(&state, 0).mode, FilterMode::Invert);
    key(&mut state, "M", &mut dirty);
    draw(&mut state, (350.0, 240.0), (430.0, 300.0), &mut dirty);
    assert_eq!(filter(&state, 1).mode, FilterMode::Pixelate);
}

/// 【滤镜测试】【数值输入】实际输入四十二强度生成一次历史并同步后续强度，负数限定为四，不改变模式
#[test]
fn strength_text_edits_clamp_and_follow_reference_default_sync() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    draw(&mut state, (140.0, 120.0), (280.0, 180.0), &mut dirty);
    pick(&mut state, 0, &mut dirty);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Stepper {
                    range: StepperRange::Strength,
                    ..
                }
            )
        })
        .unwrap();
    let point = widget_center(&state, index);
    pointer(&mut state, point.0 - 5.0, point.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.settings_panel.is_editing());
    handle_modifiers(&mut state, true, false, &mut dirty);
    key(&mut state, "Ctrl+A", &mut dirty);
    assert_eq!(
        state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .unwrap()
            .field
            .selection_range(),
        Some((0, 2))
    );
    handle_key(&mut state, None, Some("42".to_string()), &mut dirty);
    handle_modifiers(&mut state, false, false, &mut dirty);
    key(&mut state, "Enter", &mut dirty);
    assert_eq!(filter(&state, 0).strength, 42.0);
    assert_eq!(state.tool_settings.strength, 42.0);
    assert_eq!(state.undo_stack.len(), 2);
    filter_edits::set_strength(&mut state, -10.0, true, &mut dirty);
    assert_eq!(filter(&state, 0).strength, 4.0);
    assert_eq!(state.tool_settings.filter_mode, FilterMode::Pixelate);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(filter(&state, 0).strength, 42.0);
}

/// 【滤镜测试】【弹层重点击】点击已打开的模式按钮只关闭弹层，不在同次点击重开或创建草稿
#[test]
fn repeated_toggle_click_closes_the_panel_without_reopening_it() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    open(&mut state, &mut dirty);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Action {
                    action: SettingsAction::FilterModes,
                    ..
                }
            )
        })
        .unwrap();
    let point = widget_center(&state, index);
    pointer(&mut state, point.0, point.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(!state.filter_popover.open && state.pending.is_none());
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
}

/// 【滤镜测试】【滚轮与取消】弹层打开时阻止底层强度滚动，Escape 只关闭弹层，不导出、不取消截图
#[test]
fn panel_scroll_and_escape_do_not_change_strength_or_finish_the_shot() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    open(&mut state, &mut dirty);
    pointer(&mut state, 300.0, 300.0, &mut dirty);
    handle_scroll(&mut state, 0.0, -100.0, &mut dirty);
    assert_eq!(state.tool_settings.strength, 14.0);
    key(&mut state, "Escape", &mut dirty);
    assert!(!state.filter_popover.open && !state.cancel && state.finish.is_none());
    state.filter_popover.animate_step(1.0);
    panels::filter::update(&mut state, &mut dirty);
    assert!(!state.filter_popover.is_animating());
}

/// 【滤镜测试】【窄窗口】三百二十像素界面内完整模式网格不越界，末尾模式仍可键盘确认
#[test]
fn filter_picker_fits_a_compact_window_and_keeps_all_options_reachable() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    state.placements[0].size = (320, 240);
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    open(&mut state, &mut dirty);
    let rect = state.filter_popover.rect().unwrap();
    assert!(
        rect.left() >= 0.0 && rect.top() >= 0.0 && rect.right() <= 320.0 && rect.bottom() <= 240.0
    );
    key(&mut state, "End", &mut dirty);
    key(&mut state, "Enter", &mut dirty);
    assert_eq!(state.tool_settings.filter_mode, FilterMode::Brighten);
}
