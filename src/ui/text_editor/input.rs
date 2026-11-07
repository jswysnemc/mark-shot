//! 【界面控件】【文字输入】快捷键和鼠标动作，不在控件内部执行剪贴板进程

use cosmic_text::{Action, Edit, FontSystem, Motion, Selection};

use super::TextEditor;
use crate::{backend::window::KeyInput, keys::Key};

#[derive(Debug, PartialEq, Eq)]
pub enum EditEvent {
    None,
    Redraw,
    Changed,
    Copy(String),
    Cut(String),
    Paste,
}

/// 【界面控件】【移动映射】将 key、ctrl 转换为编辑器移动动作；非移动按键返回空值
fn motion(key: Key, ctrl: bool) -> Option<Motion> {
    Some(match key {
        Key::Left => {
            if ctrl {
                Motion::LeftWord
            } else {
                Motion::Left
            }
        }
        Key::Right => {
            if ctrl {
                Motion::RightWord
            } else {
                Motion::Right
            }
        }
        Key::Up => Motion::Up,
        Key::Down => Motion::Down,
        Key::Home => {
            if ctrl {
                Motion::BufferStart
            } else {
                Motion::Home
            }
        }
        Key::End => {
            if ctrl {
                Motion::BufferEnd
            } else {
                Motion::End
            }
        }
        Key::PageUp => Motion::PageUp,
        Key::PageDown => Motion::PageDown,
        _ => return None,
    })
}

impl TextEditor {
    /// 【界面控件】【键盘编辑】处理 input 并使用 fonts 排版；返回修改、重绘或剪贴板请求
    pub fn key(&mut self, fonts: &mut FontSystem, input: &KeyInput) -> EditEvent {
        if input.key.is_none() && input.text.is_empty() {
            return EditEvent::None;
        }
        let cancelled = if self.preedit().is_some() {
            self.interrupt_input()
        } else {
            false
        };
        let result = self.key_committed(fonts, input);
        if cancelled && result == EditEvent::None {
            EditEvent::Redraw
        } else {
            result
        }
    }

    /// 【界面控件】【已确认按键】处理 input 的正文编辑，fonts 用于排版；候选由外层键盘入口取消，返回编辑事件
    fn key_committed(&mut self, fonts: &mut FontSystem, input: &KeyInput) -> EditEvent {
        if self.input.single_line && matches!(input.key, Some(Key::Enter | Key::Tab)) {
            self.input.separator = true;
            return EditEvent::None;
        }
        let ctrl = input.mods.ctrl && !input.mods.alt;
        let shift = input.mods.shift;
        if ctrl {
            match input.key {
                Some(Key::Char('a')) => {
                    self.select_all(fonts);
                    return EditEvent::Redraw;
                }
                Some(Key::Char('c')) => {
                    let text = if shift {
                        Some(self.text())
                    } else {
                        self.selected_text()
                    };
                    return text.map_or(EditEvent::None, EditEvent::Copy);
                }
                Some(Key::Char('x')) => {
                    return self.selected_text().map_or(EditEvent::None, EditEvent::Cut);
                }
                Some(Key::Char('v')) => return EditEvent::Paste,
                Some(Key::Char('z')) => {
                    return if if shift {
                        self.redo(fonts)
                    } else {
                        self.undo(fonts)
                    } {
                        EditEvent::Changed
                    } else {
                        EditEvent::None
                    };
                }
                Some(Key::Char('y')) => {
                    return if self.redo(fonts) {
                        EditEvent::Changed
                    } else {
                        EditEvent::None
                    };
                }
                Some(Key::Backspace | Key::Delete) => {
                    let changed = self.delete_word(fonts, input.key == Some(Key::Backspace));
                    return if changed {
                        EditEvent::Changed
                    } else {
                        EditEvent::None
                    };
                }
                _ => {}
            }
        }
        if let Some(key) = input.key
            && let Some(motion) = motion(key, ctrl)
        {
            self.input.separator = true;
            if shift {
                if self.editor.selection() == Selection::None {
                    self.editor
                        .set_selection(Selection::Normal(self.editor.cursor()));
                }
            } else {
                self.editor.set_selection(Selection::None);
            }
            self.editor.action(fonts, Action::Motion(motion));
            self.editor.shape_as_needed(fonts, false);
            return EditEvent::Redraw;
        }
        if ctrl || input.mods.logo || (input.mods.alt && !input.mods.ctrl) {
            return EditEvent::None;
        }
        let action = match input.key {
            Some(Key::Backspace) => Some(Action::Backspace),
            Some(Key::Delete) => Some(Action::Delete),
            Some(Key::Enter) => Some(Action::Enter),
            Some(Key::Tab) => Some(if shift {
                Action::Unindent
            } else {
                Action::Insert('\t')
            }),
            _ => None,
        };
        let changed = if let Some(action) = action {
            self.action(fonts, action)
        } else {
            let text: String = input.text.chars().filter(|ch| !ch.is_control()).collect();
            self.type_text(fonts, &text)
        };
        if changed {
            EditEvent::Changed
        } else {
            EditEvent::None
        }
    }

    /// 【界面控件】【鼠标选择】根据本地 position、drag、clicks 选择文字；fonts 用于精确字符定位
    pub fn pointer(
        &mut self,
        fonts: &mut FontSystem,
        position: (f64, f64),
        drag: bool,
        clicks: u8,
    ) {
        self.interrupt_input();
        let (x, y) = (position.0.round() as i32, position.1.round() as i32);
        let action = if drag {
            Action::Drag { x, y }
        } else if clicks >= 3 {
            Action::TripleClick { x, y }
        } else if clicks == 2 {
            Action::DoubleClick { x, y }
        } else {
            Action::Click { x, y }
        };
        self.editor.action(fonts, action);
        self.editor.shape_as_needed(fonts, false);
    }

    /// 【界面控件】【文字滚动】按 pixels 移动视区；不改变内容、光标或撤销历史
    pub fn scroll(&mut self, fonts: &mut FontSystem, pixels: f32) {
        self.editor.action(fonts, Action::Scroll { pixels });
        self.editor.shape_as_needed(fonts, false);
    }
}
