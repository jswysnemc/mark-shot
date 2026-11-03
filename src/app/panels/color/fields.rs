//! 【属性取色】【字段输入】十六进制文本只在确认或失去焦点时应用，不逐字符修改标注

use crate::editor::{
    EditorState,
    edits::{apply_color_selection, damage_color_popover},
};
use crate::interaction::ClickTarget;
use crate::types::SpecialKey;
use crate::ui::color_popover::{self, ColorField};
use crate::ui::text_field::{CursorInit, is_hex_char, is_rgba_channel_char};

/// 【属性取色】【文本约束】校验 state 的 Hex 字段；previous 用于拒绝非法位置的井号与超长输入，无返回值
fn validate_text(state: &mut EditorState, previous: Option<crate::ui::text_field::LineEditState>) {
    if let Some(edit) = &mut state.color_popover.fields.editing
        && edit.key == ColorField::Hex
    {
        let text = edit
            .field
            .text
            .strip_prefix('#')
            .unwrap_or(&edit.field.text);
        if (text.len() > 8 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()))
            && let Some(previous) = previous
        {
            edit.field = previous;
        }
    }
}

/// 【属性取色】【字符过滤】按 field 返回字符校验方法，拒绝非十六进制输入
fn field_char_filter(field: ColorField) -> impl Fn(char) -> bool {
    move |ch| match field {
        ColorField::Hex => is_hex_char(ch),
        ColorField::R | ColorField::G | ColorField::B | ColorField::A => is_rgba_channel_char(ch),
    }
}

/// 【属性取色】【字段应用】把 text 按 field 应用到 editor_state；record、record_undo 控制记录，dirty_mask 返回重绘区域，返回输入是否有效
fn apply_color_field_text(
    editor_state: &mut EditorState,
    field: ColorField,
    text: &str,
    record: bool,
    record_undo: bool,
    dirty_mask: &mut u32,
) -> bool {
    let applied = match field {
        ColorField::Hex => editor_state.color_popover.try_apply_hex_text(text),
        _ => editor_state.color_popover.try_apply_rgba_text(field, text),
    };

    let Some(color) = applied else { return false };

    if record {
        editor_state.color_popover.record_used_color(color);
    }
    editor_state.color_popover.sync_field_values();
    apply_color_selection(editor_state, color, record_undo, dirty_mask);
    true
}

/// 【属性取色】【字段聚焦】按 local 为 editor_state 的 field 定位光标或双击全选；dirty_mask 返回字段重绘区域
pub(super) fn begin_color_field_edit(
    editor_state: &mut EditorState,
    field: ColorField,
    local: (f64, f64),
    dirty_mask: &mut u32,
) {
    if editor_state.color_popover.fields.is_editing() {
        commit_color_field_edit(editor_state, dirty_mask);
    }

    let current_text = editor_state.color_popover.field_text(field);

    let click_pos = (local.0 as f32, local.1 as f32);
    let is_double_click = editor_state
        .input
        .clicks
        .register(ClickTarget::ColorField(field), click_pos);

    let cursor_init = if is_double_click {
        CursorInit::SelectAll
    } else if let Some(rect) = editor_state.color_popover.field_rect(field) {
        let click_x = local.0 as f32 - rect.left();
        let idx = editor_state
            .color_popover
            .picker_fonts
            .get_or_insert_with(color_popover::fonts::PickerFonts::new)
            .char_index(&current_text, click_x);
        CursorInit::At(idx)
    } else {
        CursorInit::End
    };

    editor_state
        .color_popover
        .fields
        .begin_edit(field, current_text, cursor_init);
    damage_color_popover(editor_state, dirty_mask);
}

/// 【属性取色】【字段确认】提交 editor_state 当前文本并恢复规范格式；dirty_mask 返回重绘区域，无效输入不修改颜色
pub fn commit_color_field_edit(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let Some((field, text)) = editor_state.color_popover.fields.commit_edit() else {
        return;
    };

    apply_color_field_text(editor_state, field, &text, true, false, dirty_mask);

    editor_state.color_popover.refresh_palette();
    editor_state.color_popover.sync_field_values();
    damage_color_popover(editor_state, dirty_mask);
}

/// 【属性取色】【确认保留】确认 state 当前字段但保留有效文本的局部历史，dirty 返回显示；规范文本不同才重置编辑器，不确认候选文字
fn confirm_in_place(state: &mut EditorState, dirty: &mut u32) {
    let Some(mut edit) = state.color_popover.fields.editing.take() else {
        return;
    };
    apply_color_field_text(state, edit.key, &edit.field.text, true, false, dirty);
    state.color_popover.refresh_palette();
    state.color_popover.sync_field_values();
    let canonical = state.color_popover.confirmed_field_text(edit.key);
    let reset = !edit.field.text.eq_ignore_ascii_case(&canonical);
    if reset {
        edit.field = crate::ui::text_field::LineEditState::new(canonical);
    }
    let cancelled = edit.field.cancel_preedit();
    if reset || cancelled {
        edit.id = crate::features::text::input_method::focus::identity();
    }
    state.color_popover.fields.editing = Some(edit);
    damage_color_popover(state, dirty);
}

/// 【属性取色】【字符输入】把 ch 插入 editor_state 当前字段；dirty_mask 返回重绘区域，不提前修改标注
pub fn handle_color_field_text_input(
    editor_state: &mut EditorState,
    ch: char,
    dirty_mask: &mut u32,
) {
    let Some(field) = editor_state
        .color_popover
        .fields
        .editing
        .as_ref()
        .map(|e| e.key)
    else {
        return;
    };
    let allowed = field_char_filter(field);
    let previous = editor_state
        .color_popover
        .fields
        .editing
        .as_ref()
        .map(|edit| edit.field.clone());
    if editor_state.color_popover.fields.insert_char(ch, allowed) {
        validate_text(editor_state, previous);
        damage_color_popover(editor_state, dirty_mask);
    }
}

/// 【属性取色】【字段按键】把 key 交给 editor_state 当前字段，确认键提交文本；dirty_mask 返回重绘区域
pub fn handle_color_field_key_press(
    editor_state: &mut EditorState,
    key: SpecialKey,
    dirty_mask: &mut u32,
) {
    let Some(field) = editor_state
        .color_popover
        .fields
        .editing
        .as_ref()
        .map(|e| e.key)
    else {
        return;
    };
    let allowed = field_char_filter(field);
    let ctrl = editor_state.input.ctrl;
    let shift = editor_state.input.shift;
    let previous = editor_state
        .color_popover
        .fields
        .editing
        .as_ref()
        .map(|edit| edit.field.clone());

    let (changed, commit) = editor_state
        .color_popover
        .fields
        .handle_key(key, ctrl, shift, allowed);

    if changed {
        validate_text(editor_state, previous);
        damage_color_popover(editor_state, dirty_mask);
    }

    if commit {
        confirm_in_place(editor_state, dirty_mask);
    }
}
