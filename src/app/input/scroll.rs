//! 【界面】【滚轮输入】调整工具属性和颜色字段

use super::*;

/// 【界面】【滚轮分派】根据 editor_state 的弹层或属性消费 delta_x、delta_y；dirty_mask 返回重绘显示器
#[cfg(test)]
pub fn handle_scroll(
    editor_state: &mut EditorState,
    delta_x: f32,
    delta_y: f32,
    dirty_mask: &mut u32,
) {
    handle_axis_scroll(
        editor_state,
        crate::types::ScrollEvent::pixels(delta_x as f64, delta_y as f64),
        dirty_mask,
    );
}

/// 【界面】【轴事件分派】按 event 原始轴数据处理 editor_state 的文字输入，旧工具继续使用兼容像素；dirty_mask 返回重绘显示器
pub fn handle_axis_scroll(
    editor_state: &mut EditorState,
    event: crate::types::ScrollEvent,
    dirty_mask: &mut u32,
) {
    if crate::app::capture_start::input::hit(editor_state).is_some()
        || editor_state.capture_start.pressed.is_some()
    {
        return;
    }
    if editor_state
        .color_inspector
        .panel
        .is_some_and(|(monitor, rect)| {
            monitor == editor_state.input.pointer.monitor_idx
                && crate::features::color_sample::inspector::contains(
                    rect,
                    editor_state.input.pointer.local,
                )
        })
    {
        return;
    }
    if event.canvas_steps() != 0.0
        && matches!(
            hit_test_ui(editor_state, editor_state.input.pointer.local),
            UiHit::None
        )
    {
        super::recent_annotation::select(editor_state, dirty_mask);
    }
    let before = super::wheel_preview::capture(editor_state);
    if matches!(
        hit_test_ui(editor_state, editor_state.input.pointer.local),
        UiHit::None
    ) && crate::features::color_sample::edit::wheel(editor_state, event, dirty_mask)
    {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    dispatch_axis_scroll(editor_state, event, dirty_mask);
    if let Some(after) = super::wheel_preview::capture(editor_state)
        && before.is_some_and(|before| before != after)
    {
        super::super::canvas_navigation::preview::show_sample(
            editor_state,
            after,
            Instant::now(),
            dirty_mask,
        );
        apply_damage_rects(editor_state, dirty_mask);
    }
}

/// 【界面】【轴事件处理】按event调整实际属性或导航，dirty_mask返回原有业务损坏范围，无返回值
fn dispatch_axis_scroll(
    editor_state: &mut EditorState,
    event: crate::types::ScrollEvent,
    dirty_mask: &mut u32,
) {
    if super::super::external_actions::scroll(editor_state, event, dirty_mask) {
        return;
    }
    if editor_state.text_context_menu.is_some() {
        return;
    }
    if super::super::selection_size::input::scroll(editor_state, event, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if matches!(
        hit_test_ui(editor_state, editor_state.input.pointer.local),
        UiHit::None
    ) && super::super::region_detection::input::scroll(editor_state, event, dirty_mask)
    {
        return;
    }
    let local = editor_state.input.pointer.local;
    let delta_y = event.legacy_y() * crate::config::get().input.scroll_sensitivity;
    if font_popup::scroll(editor_state, event, dirty_mask)
        || (matches!(hit_test_ui(editor_state, local), UiHit::SettingsItem(_))
            && super::super::panels::text::size_wheel::properties(editor_state, event, dirty_mask))
    {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }

    if super::super::canvas_navigation::input::scroll(editor_state, event, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }

    if matches!(hit_test_ui(editor_state, local), UiHit::None)
        && (editor_state.text_session.is_some()
            || editor_state.settings_panel.text_layout.is_some())
    {
        super::super::panels::text::size_wheel::canvas(
            editor_state,
            event,
            Instant::now(),
            dirty_mask,
        );
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    editor_state.commit_text_size_wheel();
    // 1. 【文字滚轮】【上下文切换】其他画布工具接收滚轮后，文字不能继承此前的半点余量
    if editor_state.settings_panel.text_layout.is_none() && event.canvas_steps() != 0.0 {
        editor_state.text_size_history.context = None;
        editor_state.text_size_history.input.reset();
    }
    if filter::handle_scroll(editor_state, delta_y, dirty_mask)
        || line::handle_scroll(editor_state, delta_y, dirty_mask)
        || ruler::handle_scroll(editor_state, delta_y, dirty_mask)
        || number::handle_scroll(editor_state, delta_y, dirty_mask)
        || marker::handle_scroll(editor_state, delta_y, dirty_mask)
    {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }

    if editor_state.settings_panel.visible
        && let (_, Some(widget_idx)) = editor_state.settings_panel.hit_test(local)
        && matches!(
            editor_state.settings_panel.widgets.get(widget_idx),
            Some(SettingsWidget::Stepper { .. })
        )
    {
        handle_stepper_scroll(editor_state, widget_idx, delta_y, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }

    if matches!(hit_test_ui(editor_state, local), UiHit::None)
        && let Some(widget_idx) = editor_state
            .settings_panel
            .widgets
            .iter()
            .position(|w| matches!(w, SettingsWidget::Stepper { .. }))
    {
        handle_stepper_scroll(editor_state, widget_idx, delta_y, dirty_mask);
        let snapshot = editor_state.settings_panel.pre_edit_snapshot.take();
        editor_state.commit_snapshot(snapshot);
    }

    apply_damage_rects(editor_state, dirty_mask);
}

pub(super) fn languages_button_hit(editor_state: &EditorState) -> bool {
    let (_, widget_idx) = editor_state
        .settings_panel
        .hit_test(editor_state.input.pointer.local);
    matches!(
        widget_idx.and_then(|idx| editor_state.settings_panel.widgets.get(idx)),
        Some(SettingsWidget::Action {
            action: crate::ui::settings_panel::SettingsAction::OcrLanguages,
            ..
        })
    )
}
