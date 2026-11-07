//! 【选区尺寸】【键盘归属】活动字段消费编辑键和文字，比例列表独占键盘，精确 Ctrl+R 聚焦宽度

use crate::{
    editor::EditorState,
    features::selection_size::ratio::Field,
    keys::{Chord, Key, Mods},
    ui::selection_size::navigation,
};

/// 【选区尺寸】【单一修饰】按 mods、ctrl 与 shift 判断是否精确匹配标准快捷键，返回判断结果
fn modifiers(mods: Mods, ctrl: bool, shift: bool) -> bool {
    mods == Mods {
        ctrl,
        shift,
        ..Mods::default()
    }
}

/// 【选区尺寸】【宽高聚焦】按 side 在 state 结束前一个字段并全选目标字段，dirty 返回编辑变化，无返回值
pub(in crate::app) fn focus(state: &mut EditorState, side: Field, dirty: &mut u32) {
    super::popup::close(state, dirty);
    if state
        .selection_size
        .panel
        .as_ref()
        .and_then(|panel| panel.editing())
        .is_some_and(|editing| editing != side)
    {
        super::apply::commit(state, dirty);
    }
    if let Some(panel) = state
        .selection_size
        .panel
        .as_mut()
        .filter(|panel| panel.rect.is_some())
    {
        panel.field_mut(side).begin();
    }
    super::panels::damage(state, dirty);
    super::caret::activity(state, dirty);
}

/// 【选区尺寸】【字段剪贴板】按 key 和 mods 在 state 当前字段复制、剪切或粘贴，返回是否属于剪贴板操作
fn clipboard(state: &mut EditorState, side: Field, key: Key, mods: Mods) -> bool {
    let copy = modifiers(mods, true, false) && matches!(key, Key::Char('c') | Key::Insert);
    let cut = modifiers(mods, true, false) && key == Key::Char('x')
        || modifiers(mods, false, true) && key == Key::Delete;
    let paste = modifiers(mods, true, false) && key == Key::Char('v')
        || modifiers(mods, false, true) && key == Key::Insert;
    if !copy && !cut && !paste {
        return false;
    }
    let field = state.selection_size.panel.as_mut().unwrap().field_mut(side);
    if paste {
        if let Some(text) = crate::utils::paste_from_clipboard() {
            field.insert(&text);
        }
    } else if let Some(text) = field.editor.selected_text() {
        if let Err(error) = crate::utils::copy_to_clipboard(&text) {
            log::error!("【选区尺寸】【剪贴板】无法复制字段文字: {error}");
        } else if cut {
            field.edit(|editor| editor.backspace_selection_only());
        }
    }
    true
}

/// 【选区尺寸】【字段按键】按 chord 和 text 在 state 当前输入处理导航、确认和原子文字插入，dirty 返回局部变化
pub(in crate::app) fn key(
    state: &mut EditorState,
    chord: Option<Chord>,
    text: Option<&str>,
    dirty: &mut u32,
) -> bool {
    if let Some(popup) = &mut state.selection_size.popup {
        let event = chord.map_or(
            crate::ui::selection_size::ratio_popup::Event::None,
            |chord| popup.key(chord.key, chord.mods.shift),
        );
        super::popup::chosen(state, event, dirty);
        return true;
    }
    if chord.is_some_and(|chord| {
        chord.key == Key::Char('r')
            && chord.mods
                == Mods {
                    ctrl: true,
                    ..Mods::default()
                }
    }) && state.mode == crate::types::Mode::Editor
        && super::apply::usable(state)
        && !state.input.mouse_down
        && !state.tool_active
        && state.ann_drag.is_none()
        && !state.fullscreen_scope
    {
        super::panels::update(state, dirty);
        focus(state, Field::Width, dirty);
        return true;
    }
    let Some(side) = state
        .selection_size
        .panel
        .as_ref()
        .filter(|panel| panel.rect.is_some())
        .and_then(|panel| panel.editing())
    else {
        return false;
    };
    if let Some(chord) = chord {
        let field = state.selection_size.panel.as_mut().unwrap().field_mut(side);
        match chord.key {
            Key::Enter => {
                super::apply::commit(state, dirty);
                return true;
            }
            Key::Escape => {
                field.cancel();
                super::panels::damage(state, dirty);
                super::caret::tick(state, std::time::Instant::now(), dirty);
                return true;
            }
            Key::Tab => {
                super::apply::commit(state, dirty);
                focus(
                    state,
                    if side == Field::Width {
                        Field::Height
                    } else {
                        Field::Width
                    },
                    dirty,
                );
                return true;
            }
            Key::Up | Key::Down => field.arrow(
                if chord.key == Key::Up { 1 } else { -1 } * if chord.mods.shift { 10 } else { 1 },
            ),
            Key::Char('a') if modifiers(chord.mods, true, false) => field.editor.select_all(),
            Key::Char('z') if chord.mods.ctrl && !chord.mods.alt && !chord.mods.logo => {
                field.edit(|editor| {
                    if chord.mods.shift {
                        editor.redo();
                    } else {
                        editor.undo();
                    }
                });
            }
            Key::Char('y') if modifiers(chord.mods, true, false) => {
                field.edit(|editor| {
                    editor.redo();
                });
            }
            Key::Left if !chord.mods.alt && !chord.mods.logo => {
                if chord.mods.ctrl {
                    navigation::move_word(field, true, chord.mods.shift);
                } else {
                    field.edit(|editor| editor.move_left(chord.mods.shift));
                }
            }
            Key::Right if !chord.mods.alt && !chord.mods.logo => {
                if chord.mods.ctrl {
                    navigation::move_word(field, false, chord.mods.shift);
                } else {
                    field.edit(|editor| editor.move_right(chord.mods.shift));
                }
            }
            Key::Home if !chord.mods.ctrl && !chord.mods.alt && !chord.mods.logo => {
                field.edit(|editor| editor.move_home(chord.mods.shift));
            }
            Key::End if !chord.mods.ctrl && !chord.mods.alt && !chord.mods.logo => {
                field.edit(|editor| editor.move_end(chord.mods.shift));
            }
            Key::Backspace if !chord.mods.alt && !chord.mods.logo => {
                if chord.mods.ctrl && chord.mods.shift {
                    navigation::extend_delete_backward(field);
                } else if chord.mods.ctrl {
                    navigation::delete_word(field, true);
                } else {
                    field.edit(|editor| editor.backspace());
                }
            }
            Key::Delete if !chord.mods.shift && !chord.mods.alt && !chord.mods.logo => {
                if chord.mods.ctrl {
                    navigation::delete_word(field, false);
                } else {
                    field.edit(|editor| editor.delete_forward());
                }
            }
            _ => {
                if !clipboard(state, side, chord.key, chord.mods)
                    && !chord.mods.ctrl
                    && !chord.mods.alt
                    && !chord.mods.logo
                    && let Some(text) = text
                {
                    state
                        .selection_size
                        .panel
                        .as_mut()
                        .unwrap()
                        .field_mut(side)
                        .insert(text);
                }
            }
        }
    } else if let Some(text) = text {
        state
            .selection_size
            .panel
            .as_mut()
            .unwrap()
            .field_mut(side)
            .insert(text);
    }
    super::panels::damage(state, dirty);
    super::caret::activity(state, dirty);
    true
}
