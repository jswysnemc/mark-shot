//! 【文字事务】【输入职责】编辑预览接收正文、局部撤销和字符拖选，提交快捷键不穿透到截图动作

use crate::{
    editor::{
        EditorState,
        text_session::{self, PREVIEW_ID},
        text_session_history::Kind,
    },
    keys::{Chord, Key},
    tools::Tool,
    types::{MouseButton, SpecialKey},
};
use cosmic_text::{Action, Edit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    DeleteSelection,
    SelectAll,
}

/// 【文字事务】【菜单命令】将 command 应用于 state 当前预览，dirty 返回重绘区域；删除只移除选区，复制不导出截图
pub fn command(state: &mut EditorState, command: Command, dirty: &mut u32) {
    if matches!(command, Command::Undo | Command::Redo) {
        history(state, command == Command::Redo, dirty);
        return;
    }
    mutate(
        state,
        Kind::Standalone,
        |editor, fonts| {
            if command == Command::DeleteSelection {
                editor.delete_selection();
                return;
            }
            let key = match command {
                Command::Cut => SpecialKey::KeyX,
                Command::Copy => SpecialKey::KeyC,
                Command::Paste => SpecialKey::KeyV,
                Command::SelectAll => SpecialKey::KeyA,
                _ => unreachable!(),
            };
            super::apply_key_to_editor(editor, fonts, key, true, false);
        },
        dirty,
    );
}

/// 【文字事务】【编辑操作】在 state 的预览编辑器执行 operation，kind 控制历史分组；dirty 返回变化范围，无返回值
fn mutate(
    state: &mut EditorState,
    kind: Kind,
    operation: impl FnOnce(&mut cosmic_text::Editor<'static>, &mut cosmic_text::FontSystem),
    dirty: &mut u32,
) {
    crate::editor::text_composition::cancel(state, dirty);
    text_session::sync_style(state, dirty);
    let Some(session) = state.text_session.as_mut() else {
        return;
    };
    let Some(editor) = state.text.editors.get_mut(&PREVIEW_ID) else {
        return;
    };
    if kind == Kind::Standalone {
        session.history.break_group();
    }
    let before = (editor.cursor(), editor.selection());
    editor.start_change();
    operation(editor, &mut state.text.font_system);
    let change = editor.finish_change();
    session.history.record(
        change,
        crate::editor::text_session_format::Font::read(&session.working),
        before,
        (editor.cursor(), editor.selection()),
        kind,
    );
    text_session::refresh(state, dirty);
}

/// 【文字事务】【正文插入】把 text 插入 state 的预览并规范换行，dirty 返回重绘范围；空字符串不产生历史
pub fn text(state: &mut EditorState, text: &str, dirty: &mut u32) {
    if text.is_empty() {
        return;
    }
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let kind = if text.contains('\n') {
        Kind::Standalone
    } else {
        Kind::Typing
    };
    mutate(
        state,
        kind,
        |editor, _| {
            editor.insert_string(&text, None);
        },
        dirty,
    );
}

/// 【文字事务】【局部历史】在 state 的预览上执行 redo 指定的恢复，dirty 返回重绘范围，无返回值
fn history(state: &mut EditorState, redo: bool, dirty: &mut u32) {
    crate::editor::text_composition::cancel(state, dirty);
    if let Some(session) = state.text_session.as_mut()
        && let Some(editor) = state.text.editors.get_mut(&PREVIEW_ID)
    {
        session.history.step(editor, &mut session.working, redo);
        let empty = editor
            .with_buffer(|buffer| buffer.lines.len() == 1 && buffer.lines[0].text().is_empty());
        if empty {
            // 1. 【文字事务】【空文档字体】原版空文档使用默认字体，重做正文时再恢复该条输入的原字体
            session.source.font.apply(&mut session.working);
        }
    }
    text_session::refresh(state, dirty);
}

/// 【文字事务】【键盘拦截】优先处理 state 编辑预览中的 chord 和 text，dirty 返回范围；返回是否消费，Ctrl+C 无选区交给截图导出
pub fn key(
    state: &mut EditorState,
    chord: Option<Chord>,
    value: Option<&str>,
    dirty: &mut u32,
) -> bool {
    if state.text_session.is_none() {
        return false;
    }
    if let Some(chord) = chord {
        if chord.key == Key::Escape || chord.key == Key::Enter && chord.mods.ctrl {
            text_session::commit(state, dirty);
            return true;
        }
        if chord.mods.ctrl && chord.key == Key::Char('z') {
            history(state, chord.mods.shift, dirty);
            return true;
        }
        if chord.mods.ctrl && chord.key == Key::Char('y') {
            history(state, true, dirty);
            return true;
        }
        if chord.mods.ctrl && chord.key == Key::Char('c') && !has_selection(state) {
            return false;
        }
        let special = match chord.key {
            Key::Enter => Some(SpecialKey::Enter),
            Key::Backspace => Some(SpecialKey::Backspace),
            Key::Delete => Some(SpecialKey::Delete),
            Key::Left => Some(SpecialKey::Left),
            Key::Right => Some(SpecialKey::Right),
            Key::Up => Some(SpecialKey::Up),
            Key::Down => Some(SpecialKey::Down),
            Key::Home => Some(SpecialKey::Home),
            Key::End => Some(SpecialKey::End),
            Key::Char('a') if chord.mods.ctrl => Some(SpecialKey::KeyA),
            Key::Char('c') if chord.mods.ctrl => Some(SpecialKey::KeyC),
            Key::Char('x') if chord.mods.ctrl => Some(SpecialKey::KeyX),
            Key::Char('v') if chord.mods.ctrl => Some(SpecialKey::KeyV),
            _ => None,
        };
        if let Some(special) = special {
            let kind = match special {
                SpecialKey::Backspace if !chord.mods.ctrl => Kind::Backspace,
                SpecialKey::Delete if !chord.mods.ctrl => Kind::Delete,
                _ => Kind::Standalone,
            };
            mutate(
                state,
                kind,
                |editor, fonts| {
                    super::apply_key_to_editor(
                        editor,
                        fonts,
                        special,
                        chord.mods.ctrl,
                        chord.mods.shift,
                    );
                },
                dirty,
            );
            return true;
        }
    }
    if let Some(value) = value {
        text(state, value, dirty);
        return true;
    }
    false
}

/// 【文字事务】【选区判断】返回 state 的预览是否有非空字符选区，不检查标注选择框
pub fn has_selection(state: &EditorState) -> bool {
    state.text.editors.get(&PREVIEW_ID).is_some_and(|editor| {
        editor
            .selection_bounds()
            .is_some_and(|(start, end)| start != end)
    })
}

/// 【文字事务】【位置换算】把 global 映射到 state 预览文字坐标；返回像素位置，没有预览返回空值
fn point(state: &EditorState, global: (f64, f64)) -> Option<(i32, i32)> {
    let session = state.text_session.as_ref()?;
    let crate::types::AnnotationShape::Text {
        start, appearance, ..
    } = &session.working.shape
    else {
        return None;
    };
    let padding = crate::features::text::geometry::padding(appearance.size_scale);
    Some((
        (global.0 as f32 - start.0 - padding.0).round() as i32,
        (global.1 as f32 - start.1 - padding.1).round() as i32,
    ))
}

/// 【文字事务】【区域命中】返回 state 当前全局指针是否在独立编辑器内，包含正文内边距
pub fn inside(state: &EditorState) -> bool {
    state.text_session.as_ref().is_some_and(|session| {
        let rect = session.presented().bbox;
        let (x, y) = state.input.pointer.global;
        x >= rect.left() as f64
            && x < rect.right() as f64
            && y >= rect.top() as f64
            && y < rect.bottom() as f64
    })
}

/// 【文字事务】【字符点击】处理 state 的 button、pressed，dirty 返回范围；外部点击提交，文字工具只提交而不新建空框
pub fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if state.text_session.is_none() || !matches!(button, MouseButton::Left) {
        return false;
    }
    if !pressed {
        let dragging = state.text_session.as_mut().unwrap().dragging;
        state.text_session.as_mut().unwrap().dragging = false;
        return dragging;
    }
    if !inside(state) {
        text_session::commit(state, dirty);
        if state.selected_tool == Tool::Text {
            if let Some(id) = state
                .annotations
                .iter()
                .rev()
                .find(|annotation| {
                    matches!(annotation.shape, crate::types::AnnotationShape::Text { .. })
                        && annotation.initial_hit_test(state.input.pointer.global)
                })
                .map(|annotation| annotation.id)
            {
                text_session::begin_existing(state, id, dirty);
            }
            return true;
        }
        return false;
    }
    let (x, y) = point(state, state.input.pointer.global).unwrap();
    let double = state.input.clicks.register(
        crate::interaction::ClickTarget::TextAnnotation(PREVIEW_ID),
        (
            state.input.pointer.global.0 as f32,
            state.input.pointer.global.1 as f32,
        ),
    );
    mutate(
        state,
        Kind::Standalone,
        |editor, fonts| {
            editor.action(
                fonts,
                if double {
                    Action::DoubleClick { x, y }
                } else {
                    Action::Click { x, y }
                },
            )
        },
        dirty,
    );
    state.text_session.as_mut().unwrap().dragging = true;
    true
}

/// 【文字事务】【字符拖选】按 global 继续 state 的字符选择，dirty 返回区域；返回是否存在文字拖选，不移动标注
pub fn pointer_move(state: &mut EditorState, global: (f64, f64), dirty: &mut u32) -> bool {
    if !state
        .text_session
        .as_ref()
        .is_some_and(|session| session.dragging)
    {
        return false;
    }
    let (x, y) = point(state, global).unwrap();
    mutate(
        state,
        Kind::Standalone,
        |editor, fonts| editor.action(fonts, Action::Drag { x, y }),
        dirty,
    );
    true
}
