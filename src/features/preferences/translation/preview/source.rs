//! 【翻译测试】【原文编辑】复用多行编辑器，草稿保留在设置会话，支持恢复目标语言对应示例

use crate::features::preferences::{
    translation::layout::{Hit, Layout},
    window::{Preferences, contains},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowEvent},
    keys::Key,
    ui::text_editor::{EditEvent, EditorStyle, TextEditor},
};

#[derive(Default)]
pub(in crate::features::preferences) struct Source {
    pub editor: Option<TextEditor>,
    pub focused: bool,
    dragging: bool,
}

impl Source {
    /// 【翻译测试】【原文取值】按 target 返回用户草稿或默认示例，不修改配置文件
    pub fn text(&self, target: &str) -> String {
        self.editor
            .as_ref()
            .map_or_else(|| super::sample(target).into(), TextEditor::text)
    }
}

/// 【翻译测试】【原文激活】按 state 创建目标语言对应的编辑器并聚焦，无返回值
pub(in crate::features::preferences) fn activate(state: &mut Preferences) {
    let source = &mut state.translation.preview.source;
    if source.editor.is_none() {
        let target = state.translation.root["target_language"]
            .as_str()
            .unwrap_or("");
        source.editor = Some(TextEditor::new_with_style(
            super::sample(target),
            &mut state.fonts,
            EditorStyle {
                size: 12.0,
                line_height: 18.0,
                colors: None,
            },
        ));
    }
    source.focused = true;
    state.search_focus = false;
    state.translation.editing = None;
    state.translation.focus = Some(Hit::Source);
}

/// 【翻译测试】【编辑事件】按 state、layout 和 event 处理原文选择、输入、剪贴板和滚动，返回已消费动作
pub(in crate::features::preferences) fn event(
    state: &mut Preferences,
    layout: &Layout,
    event: &WindowEvent,
) -> Option<WindowAction> {
    let area = layout.preview.source;
    let input = layout.preview.source_input();
    // 1. 【翻译测试】【原文聚焦】先提交设置字段，再把指针交给原文编辑器
    if let WindowEvent::PointerButton {
        position,
        button: LEFT_BUTTON,
        pressed: true,
    } = event
    {
        if contains(area, *position) && contains(layout.viewport, *position) {
            if !super::super::input::apply(state) {
                return Some(WindowAction::Redraw);
            }
            activate(state);
            state.translation.preview.source.dragging = true;
            state.translation.preview.source.editor.as_mut()?.pointer(
                &mut state.fonts,
                (
                    position.0 - f64::from(input.left()),
                    position.1 - f64::from(input.top()),
                ),
                false,
                1,
            );
            return Some(WindowAction::Redraw);
        }
        state.translation.preview.source.focused = false;
    }
    // 2. 【翻译测试】【输入分派】选择、滚动和确认正文使用同一编辑器，正文变化才清理旧测试
    let source = &mut state.translation.preview.source;
    match event {
        WindowEvent::Focused(false) => {
            source.focused = false;
            source.dragging = false;
        }
        WindowEvent::PointerButton { pressed: false, .. } if source.dragging => {
            source.dragging = false;
            return Some(WindowAction::Redraw);
        }
        WindowEvent::PointerMove { position } if source.dragging => {
            source.editor.as_mut()?.pointer(
                &mut state.fonts,
                (
                    position.0 - f64::from(input.left()),
                    position.1 - f64::from(input.top()),
                ),
                true,
                1,
            );
            return Some(WindowAction::Redraw);
        }
        WindowEvent::Scroll { position, delta } if contains(area, *position) => {
            source
                .editor
                .as_mut()?
                .scroll(&mut state.fonts, delta.1 as f32 * 36.0);
            return Some(WindowAction::Redraw);
        }
        WindowEvent::TextInput(edit) if source.focused => {
            let changed = source
                .editor
                .as_mut()?
                .input_method(&mut state.fonts, edit.clone())
                .text_changed;
            if changed {
                state.translation.preview.reset();
            }
            return Some(WindowAction::Redraw);
        }
        WindowEvent::Key(input) if source.focused => {
            if matches!(input.key, Some(Key::Tab | Key::Escape))
                || input.mods.ctrl && input.key == Some(Key::Char('f'))
            {
                source.focused = false;
                return None;
            }
            let editor = source.editor.as_mut()?;
            let revision = editor.text_revision();
            match editor.key(&mut state.fonts, input) {
                EditEvent::Copy(text) | EditEvent::Cut(text) => {
                    if crate::utils::copy_to_clipboard(&text).is_ok()
                        && input.key == Some(Key::Char('x'))
                    {
                        editor.delete_selection(&mut state.fonts);
                    }
                }
                EditEvent::Paste => {
                    if let Some(text) = crate::utils::paste_from_clipboard() {
                        editor.insert(&mut state.fonts, &text);
                    }
                }
                EditEvent::Changed => {}
                _ => {}
            }
            if editor.text_revision() != revision {
                state.translation.preview.reset();
            }
            return Some(WindowAction::Redraw);
        }
        _ => {}
    }
    None
}
