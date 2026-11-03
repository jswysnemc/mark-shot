//! 【标注】【文字输入】处理键盘编辑和剪贴板动作

use super::*;

pub(super) fn key_to_action(key: SpecialKey, ctrl: bool) -> Option<Action> {
    match key {
        SpecialKey::KeyA => None,
        SpecialKey::KeyC => None,
        SpecialKey::KeyV => None,
        SpecialKey::KeyX => None,
        SpecialKey::Left => Some(Action::Motion(if ctrl {
            Motion::LeftWord
        } else {
            Motion::Left
        })),
        SpecialKey::Right => Some(Action::Motion(if ctrl {
            Motion::RightWord
        } else {
            Motion::Right
        })),
        SpecialKey::Home => Some(Action::Motion(if ctrl {
            Motion::BufferStart
        } else {
            Motion::Home
        })),
        SpecialKey::End => Some(Action::Motion(if ctrl {
            Motion::BufferEnd
        } else {
            Motion::End
        })),
        SpecialKey::Up => Some(Action::Motion(Motion::Up)),
        SpecialKey::Down => Some(Action::Motion(Motion::Down)),
        SpecialKey::Backspace => {
            if ctrl {
                None
            } else {
                Some(Action::Backspace)
            }
        }
        SpecialKey::Delete => {
            if ctrl {
                None
            } else {
                Some(Action::Delete)
            }
        }
        SpecialKey::Enter => Some(Action::Enter),
    }
}

pub fn apply_key_to_editor(
    editor: &mut Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
    key: SpecialKey,
    ctrl: bool,
    shift: bool,
) -> bool {
    // Ctrl+A
    if ctrl && matches!(key, SpecialKey::KeyA) {
        let start_cursor = cosmic_text::Cursor::new(0, 0);
        editor.set_selection(Selection::Normal(start_cursor));
        editor.action(font_system, Action::Motion(Motion::BufferEnd));
        return false;
    }

    // clipboard
    if ctrl {
        match key {
            SpecialKey::KeyC | SpecialKey::KeyX => {
                if let Some(text) = extract_selected_text(editor)
                    && let Err(e) = crate::utils::copy_to_clipboard(&text)
                {
                    info!("【文字事务】【剪贴板复制】复制失败: {e}");
                    return false;
                }

                if matches!(key, SpecialKey::KeyX) {
                    editor.delete_selection();
                    return true;
                }

                return false;
            }
            SpecialKey::KeyV => {
                if let Some(text) = crate::utils::paste_from_clipboard() {
                    if editor.selection_bounds().is_some() {
                        editor.delete_selection();
                    }
                    editor.insert_string(&text.replace("\r\n", "\n").replace('\r', "\n"), None);
                    return true;
                }
                return false;
            }
            _ => {}
        }
    }

    let is_motion = matches!(
        key,
        SpecialKey::Left
            | SpecialKey::Right
            | SpecialKey::Home
            | SpecialKey::End
            | SpecialKey::Up
            | SpecialKey::Down
    );

    if is_motion {
        if shift {
            // Shift + Motion
            if editor.selection_bounds().is_none() {
                editor.set_selection(Selection::Normal(editor.cursor()));
            }
        } else {
            // Motion without Shift
            editor.set_selection(Selection::None);
        }
    }

    // Ctrl+Backspace
    if ctrl && matches!(key, SpecialKey::Backspace) {
        if editor.selection_bounds().is_none() {
            editor.set_selection(Selection::Normal(editor.cursor()));
        }
        editor.action(font_system, Action::Motion(Motion::LeftWord));
        editor.delete_selection();
        return true;
    }

    // Ctrl+Delete
    if ctrl && matches!(key, SpecialKey::Delete) {
        if editor.selection_bounds().is_none() {
            editor.set_selection(Selection::Normal(editor.cursor()));
        }
        editor.action(font_system, Action::Motion(Motion::RightWord));
        editor.delete_selection();
        return true;
    }

    if let Some(action) = key_to_action(key, ctrl) {
        let modifies = matches!(action, Action::Backspace | Action::Delete | Action::Enter);
        editor.action(font_system, action);
        return modifies;
    }

    false
}

// ── ToolBehavior ─────────────────────────────────────────────────────────────
