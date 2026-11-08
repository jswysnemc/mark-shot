//! 【字体弹层】【字段键盘】搜索与字号输入优先接收文字和剪贴板，字号确认提交当前文字但不传给截图动作

use crate::{
    editor::{EditorState, text_edits},
    keys::{Chord, Key},
    types::SpecialKey,
    ui::{font_panel::state::Field, text_field::CursorInit},
};

/// 【字体弹层】【字号验证】返回 text 的有效点字号；空值和低于八点的中间输入返回空值，不直接夹紧
fn valid_points(text: &str) -> Option<f32> {
    text.strip_suffix(" pt")
        .unwrap_or(text)
        .parse::<u32>()
        .ok()
        .filter(|value| (8..=300).contains(value))
        .map(|value| value as f32)
}

/// 【字体弹层】【实时应用】把 state 输入字段同步到搜索过滤或有效字号，dirty 返回变化，无返回值
pub(crate) fn apply(state: &mut EditorState, dirty: &mut u32) {
    if let Some(edit) = state.font_panel.fields.editing.as_ref() {
        if edit.key == Field::Search {
            state.font_panel.filter();
        } else if let Some(points) = valid_points(&edit.field.text) {
            text_edits::set_control_points(state, points, dirty);
        }
    }
    state.font_panel.dirty = true;
    super::font_popup::update(state, dirty);
    super::super::settings::update_settings_panel(state, dirty);
}

/// 【字体弹层】【输入拦截】处理 state 中 chord 与 text，dirty 返回范围，打开时返回真，快捷键不穿透到截图
pub fn handle(
    state: &mut EditorState,
    chord: Option<Chord>,
    text: Option<&str>,
    dirty: &mut u32,
) -> bool {
    if !state.font_panel.open {
        return false;
    }
    if let Some(chord) = chord {
        if chord.key == Key::Escape {
            super::font_popup::close(state, dirty);
            return true;
        }
        if chord.key == Key::Enter && state.font_panel.fields.is_editing_key(Field::Size) {
            super::font_popup::set_size_text(state);
            apply(state, dirty);
            // 1. 【字体弹层】【字号确认】字段提交保留正文会话，后续字体调整仍作用于同一对象
            if let Some(edit) = state.font_panel.fields.editing.as_mut() {
                edit.field.cursor = 0;
            }
            return true;
        }
        if chord.key == Key::Tab {
            let field = if state.font_panel.fields.is_editing_key(Field::Search) {
                Field::Size
            } else {
                Field::Search
            };
            super::font_popup::focus(state, field, CursorInit::SelectAll);
            apply(state, dirty);
            return true;
        }
        if state.font_panel.fields.is_editing_key(Field::Size)
            && matches!(chord.key, Key::Up | Key::Down)
        {
            let delta = if chord.key == Key::Up { 1.0 } else { -1.0 };
            text_edits::set_control_points(state, state.font_panel.points + delta, dirty);
            super::font_popup::set_size_text(state);
            apply(state, dirty);
            return true;
        }
        let key = match chord.key {
            Key::Backspace => Some(SpecialKey::Backspace),
            Key::Delete => Some(SpecialKey::Delete),
            Key::Left => Some(SpecialKey::Left),
            Key::Right => Some(SpecialKey::Right),
            Key::Home => Some(SpecialKey::Home),
            Key::End => Some(SpecialKey::End),
            Key::Char('a') if chord.mods.ctrl => Some(SpecialKey::KeyA),
            Key::Char('c') if chord.mods.ctrl => Some(SpecialKey::KeyC),
            Key::Char('v') if chord.mods.ctrl => Some(SpecialKey::KeyV),
            Key::Char('x') if chord.mods.ctrl => Some(SpecialKey::KeyX),
            _ => None,
        };
        if let Some(key) = key {
            let size = state.font_panel.fields.is_editing_key(Field::Size);
            let old = size
                .then(|| {
                    state
                        .font_panel
                        .fields
                        .editing
                        .as_ref()
                        .map(|edit| edit.field.clone())
                })
                .flatten();
            state
                .font_panel
                .fields
                .handle_key(key, chord.mods.ctrl, chord.mods.shift, |ch| {
                    if size {
                        ch.is_ascii_digit()
                    } else {
                        !ch.is_control()
                    }
                });
            if size {
                reject_invalid(state, old);
                navigate_suffix(state, chord.key, chord.mods.shift);
            }
            apply(state, dirty);
            return true;
        }
    }
    if let Some(text) = text {
        let size = state.font_panel.fields.is_editing_key(Field::Size);
        for ch in text.chars() {
            let old = size
                .then(|| {
                    state
                        .font_panel
                        .fields
                        .editing
                        .as_ref()
                        .map(|edit| edit.field.clone())
                })
                .flatten();
            state.font_panel.fields.insert_char(ch, |ch| {
                if size {
                    ch.is_ascii_digit()
                } else {
                    !ch.is_control()
                }
            });
            if size {
                reject_invalid(state, old);
            }
            apply(state, dirty);
        }
    }
    true
}

/// 【字体弹层】【非法恢复】按 old 回退非法字号，缺失完整后缀时恢复并清理历史，state 返回有效中间状态，无返回值
fn reject_invalid(state: &mut EditorState, old: Option<crate::ui::text_field::LineEditState>) {
    let Some(edit) = state.font_panel.fields.editing.as_mut() else {
        return;
    };
    let rule = crate::ui::text_field::validation::Rule::Integer {
        min: 8,
        max: 300,
        suffix: " pt",
    };
    let rewritten = rule.restore_suffix(&mut edit.field.text);
    if !rule.accepts(&edit.field.text) {
        if let Some(old) = old {
            edit.field = old;
        }
    } else if rewritten {
        let cursor = edit.field.cursor;
        edit.field =
            crate::ui::text_field::LineEditState::new(std::mem::take(&mut edit.field.text));
        edit.field.cursor = cursor;
        edit.id = crate::features::text::input_method::focus::identity();
    }
}

/// 【字体弹层】【单位导航】按 key 与 shift 修正 state 中非扩展方向键的后缀跳转，其他选择与原生回退位置不夹紧，无返回值
fn navigate_suffix(state: &mut EditorState, key: Key, shift: bool) {
    if shift || !matches!(key, Key::Left | Key::Right) {
        return;
    }
    let Some(edit) = state.font_panel.fields.editing.as_mut() else {
        return;
    };
    let end = edit.field.text.chars().count();
    let number = edit
        .field
        .text
        .strip_suffix(" pt")
        .map_or(end, |text| text.chars().count());
    if edit.field.cursor > number && edit.field.cursor < end {
        edit.field.cursor = if matches!(key, Key::Left) {
            number
        } else {
            end
        };
    }
}
