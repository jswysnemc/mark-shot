//! 【界面】【键盘输入】解析文字、快捷键和属性输入

use super::*;

pub(super) fn handle_text_input(editor_state: &mut EditorState, ch: char, dirty_mask: &mut u32) {
    if editor_state.color_popover.fields.is_editing() {
        handle_color_field_text_input(editor_state, ch, dirty_mask);
        return;
    }
    if editor_state.settings_panel.is_editing() {
        handle_settings_text_input(editor_state, ch, dirty_mask);
        return;
    }
    dispatch_text(editor_state.selected_tool, editor_state, ch, dirty_mask);
    apply_damage_rects(editor_state, dirty_mask);
}

pub(super) fn typing(editor_state: &EditorState) -> bool {
    editor_state.color_popover.fields.is_editing()
        || editor_state.settings_panel.is_editing()
        || (editor_state.selected_tool == Tool::Text && editor_state.text.editing.is_some())
}

pub(super) fn special_key(chord: Chord) -> Option<SpecialKey> {
    Some(match chord.key {
        Key::Backspace => SpecialKey::Backspace,
        Key::Enter => SpecialKey::Enter,
        Key::Left => SpecialKey::Left,
        Key::Right => SpecialKey::Right,
        Key::Home => SpecialKey::Home,
        Key::End => SpecialKey::End,
        Key::Delete => SpecialKey::Delete,
        Key::Up => SpecialKey::Up,
        Key::Down => SpecialKey::Down,
        Key::Char('a') if chord.mods.ctrl => SpecialKey::KeyA,
        Key::Char('c') if chord.mods.ctrl => SpecialKey::KeyC,
        Key::Char('x') if chord.mods.ctrl => SpecialKey::KeyX,
        Key::Char('v') if chord.mods.ctrl => SpecialKey::KeyV,
        _ => return None,
    })
}

pub(super) fn hovers_stepper(editor_state: &EditorState) -> bool {
    let local = editor_state.input.pointer.local;
    editor_state.settings_panel.visible
        && matches!(
            editor_state
                .settings_panel
                .hit_test(local)
                .1
                .and_then(|idx| editor_state.settings_panel.widgets.get(idx)),
            Some(SettingsWidget::Stepper { .. })
        )
}

/// 【界面】【键盘分派】优先把 chord 交给选择弹层，再处理 text 或全局快捷键；dirty_mask 返回重绘显示器
pub fn handle_key(
    editor_state: &mut EditorState,
    chord: Option<Chord>,
    text: Option<String>,
    dirty_mask: &mut u32,
) {
    if editor_state.color_inspector.panel.is_some()
        && chord.is_some_and(|chord| chord.key == keys::Key::Escape)
    {
        crate::features::color_sample::inspector::close(editor_state, dirty_mask);
        return;
    }
    if editor_state.selected_tool == Tool::Eyedropper
        && chord.is_some_and(|chord| {
            chord.mods.ctrl
                && !chord.mods.alt
                && !chord.mods.shift
                && chord.key == keys::Key::Char('c')
        })
        && let Some(color) = editor_state.color_inspector.live
    {
        crate::tools::eyedropper::copy_value(
            editor_state,
            crate::ui::settings_panel::ValueField::Hex,
            color,
        );
        mark_all_dirty(dirty_mask, editor_state.placements.len());
        return;
    }
    // 1. 【截图界面】【主题快捷键】主题切换不提交文字或改变拖动事务，所有弹层与字段共用入口
    if chord.is_some_and(crate::theme::is_toggle) {
        super::super::theme_switch::toggle(editor_state, dirty_mask);
        return;
    }
    if super::super::selection_translation::input::key(editor_state, chord, dirty_mask) {
        return;
    }
    if super::super::external_actions::key(editor_state, chord, dirty_mask) {
        return;
    }
    if super::text_context_menu::key(editor_state, chord, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if super::super::selection_size::keyboard::key(editor_state, chord, text.as_deref(), dirty_mask)
    {
        return;
    }
    if super::super::text_input::fields::key(editor_state, chord, dirty_mask) {
        return;
    }
    editor_state.commit_text_size_wheel();
    if super::super::panels::text::keyboard::handle(
        editor_state,
        chord,
        text.as_deref(),
        dirty_mask,
    ) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if let Some(chord) = chord
        && (filter::handle_key(editor_state, chord, dirty_mask)
            || line::handle_key(editor_state, chord, dirty_mask)
            || ruler::handle_key(editor_state, chord, dirty_mask)
            || number::handle_key(editor_state, chord, dirty_mask)
            || marker::handle_key(editor_state, chord, dirty_mask))
    {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    // 1. 【字段输入】【键盘归属】字段优先于仍存在的文字会话，确认、选择和输入不能进入标注正文
    if editor_state.color_popover.fields.is_editing() || editor_state.settings_panel.is_editing() {
        if let Some(key) = chord.and_then(special_key) {
            handle_key_press(editor_state, key, dirty_mask);
            return;
        }
        if let Some(text) = text.as_ref() {
            for ch in text.chars() {
                handle_text_input(editor_state, ch, dirty_mask);
            }
            return;
        }
    }
    if editor_state.text_session.is_some() {
        if let Some(action) = chord.and_then(|chord| keys::map().lookup(chord))
            && (matches!(
                action,
                keys::Action::Finish(crate::types::Finish::Save | crate::types::Finish::Pin)
                    | keys::Action::SaveAs
                    | keys::Action::OpenWith
                    | keys::Action::Extensions
            ) || matches!(action, keys::Action::Finish(crate::types::Finish::Copy))
                && !crate::tools::text::session_input::has_selection(editor_state))
        {
            super::super::actions::run(editor_state, action, dirty_mask);
            return;
        }
        if crate::tools::text::session_input::key(editor_state, chord, text.as_deref(), dirty_mask)
        {
            super::refresh_panels(editor_state, dirty_mask);
            apply_damage_rects(editor_state, dirty_mask);
            return;
        }
    }
    let special = chord.and_then(special_key);
    // when typing ignore other actions
    let claimed = ((special.is_some() || text.is_some()) && typing(editor_state))
        || (matches!(special, Some(SpecialKey::Up | SpecialKey::Down))
            && hovers_stepper(editor_state));
    if !claimed && let Some(action) = chord.and_then(|c| keys::map().lookup(c)) {
        super::super::actions::run(editor_state, action, dirty_mask);
        return;
    }
    if !claimed && super::super::selection_history::key(editor_state, chord, dirty_mask) {
        return;
    }

    if let Some(key) = special {
        handle_key_press(editor_state, key, dirty_mask);
    } else if let Some(text) = text {
        for ch in text.chars() {
            handle_text_input(editor_state, ch, dirty_mask);
        }
    }
}

pub(super) fn handle_key_press(
    editor_state: &mut EditorState,
    key: SpecialKey,
    dirty_mask: &mut u32,
) {
    // to not stack a lot of scroll events
    // any other action cancels the scroll in progress
    // so that user can do anything afterwards, wiithout waiting for the scroll to finish
    editor_state.settings_panel.cancel_scroll();
    editor_state.color_popover.cancel_scroll();

    if matches!(key, SpecialKey::Up | SpecialKey::Down) {
        let sign: i32 = if matches!(key, SpecialKey::Up) { 1 } else { -1 };

        if let Some(widget_idx) = editor_state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .map(|e| e.key)
        {
            let arrow = if sign > 0 {
                StepperArrow::Up
            } else {
                StepperArrow::Down
            };
            apply_stepper_arrow_step(editor_state, widget_idx, arrow, dirty_mask);
            update_settings_panel(editor_state, dirty_mask);
            sync_stepper_edit_text(editor_state, widget_idx);
            apply_damage_rects(editor_state, dirty_mask);
            return;
        }

        let local = editor_state.input.pointer.local;

        if editor_state.settings_panel.visible
            && let (_, Some(widget_idx)) = editor_state.settings_panel.hit_test(local)
            && matches!(
                editor_state.settings_panel.widgets.get(widget_idx),
                Some(SettingsWidget::Stepper { .. })
            )
        {
            let arrow = if sign > 0 {
                StepperArrow::Up
            } else {
                StepperArrow::Down
            };
            apply_stepper_arrow_step(editor_state, widget_idx, arrow, dirty_mask);
            update_settings_panel(editor_state, dirty_mask);
            apply_damage_rects(editor_state, dirty_mask);
            return;
        }
    }

    if editor_state.color_popover.fields.is_editing() {
        handle_color_field_key_press(editor_state, key, dirty_mask);
        return;
    }
    if editor_state.settings_panel.is_editing() {
        handle_settings_key_press(editor_state, key, dirty_mask);
        return;
    }
    dispatch_key(editor_state.selected_tool, editor_state, key, dirty_mask);
    apply_damage_rects(editor_state, dirty_mask);
}

// ──── scroll: hit-test first (color popover fields -> settings stepper), then apply ──────────
