//! 【文字识别】【原生输入】只向当前可编辑原文或单元格分派，菜单、原图和失焦窗口不接受候选

#[cfg(test)]
use super::window::View;
use super::{
    translation::input::{self, Field},
    window::ResultWindow,
};

#[cfg(test)]
mod tests;
use crate::{
    backend::window::WindowAction,
    types::text_input::{ChangeCause, Edit, Focus, Snapshot},
};

/// 【文字识别】【输入身份】返回 state 当前可见编辑器的稳定身份，非编辑页面或失焦窗口返回空值
pub(super) fn identity(state: &ResultWindow) -> Option<u64> {
    if !state.focused || state.source_menu.is_some() || state.translation.language_menu.is_some() {
        return None;
    }
    let field = input::field(state)?;
    input::editable(state, field)
        .then(|| input::editor(state, field).map(|editor| editor.input_identity()))
        .flatten()
}

/// 【文字识别】【批次分派】将 edit 交给 state 当前编辑器，返回窗口动作；候选不写表格，只有正文变化重置复制状态
pub(super) fn apply(state: &mut ResultWindow, edit: Edit) -> WindowAction {
    if identity(state).is_none() {
        return WindowAction::None;
    }
    state.ensure_fonts(&edit.commit);
    state.ensure_fonts(&edit.preedit.text);
    let field = input::field(state).unwrap();
    let update = input::with_editor(state, field, |editor, fonts| {
        editor.input_method(fonts, edit)
    })
    .unwrap();
    if update.text_changed {
        state.status = None;
    }
    if update.changed {
        WindowAction::Redraw
    } else {
        WindowAction::None
    }
}

/// 【文字识别】【候选清理】清除 state 的全部显示候选，返回是否有变化；保留正文、单元格草稿与撤销
pub(super) fn cancel(state: &mut ResultWindow) -> bool {
    let source = state.source.cancel_preedit();
    let translation = state.translation.editor.cancel_preedit();
    let language = state.translation.language.cancel_preedit();
    let table = state
        .table
        .as_mut()
        .and_then(|table| table.editor.as_mut())
        .is_some_and(|editor| editor.cancel_preedit());
    source || table || translation || language
}

/// 【文字识别】【切页候选】仅清除 state 的语言和单元格候选，返回是否变化；多行编辑器按 Qt 保留显示状态，协议资格由页面独立停用
pub(super) fn cancel_non_text(state: &mut ResultWindow) -> bool {
    let language = state.translation.language.cancel_preedit();
    let table = state
        .table
        .as_mut()
        .and_then(|table| table.editor.as_mut())
        .is_some_and(|editor| editor.cancel_preedit());
    language || table
}

/// 【文字识别】【上下文快照】按 state、focus、target_id、previous 和 cause 生成变化后的真实光标与有限正文，未变化返回空值
pub(super) fn snapshot(
    state: &mut ResultWindow,
    focus: Focus,
    target_id: u64,
    previous: Option<&Snapshot>,
    cause: ChangeCause,
) -> Option<Snapshot> {
    identity(state)?;
    let field = input::field(state)?;
    let area = match field {
        Field::Source => crate::ui::text_editor::InputArea {
            rect: state.layout.text_rect(),
            offset: 0.0,
        },
        Field::Translation => crate::ui::text_editor::InputArea {
            rect: state.layout.translation.as_ref()?.editor,
            offset: 0.0,
        },
        Field::Language => crate::ui::text_editor::InputArea {
            rect: state.layout.translation.as_ref()?.language_text,
            offset: state.translation.language_offset,
        },
        Field::Table => {
            let table = state.table.as_ref()?;
            crate::ui::text_editor::InputArea {
                rect: table.input_rect(state.layout.table_body)?,
                offset: table.editor_offset,
            }
        }
    };
    input::with_editor(state, field, |editor, fonts| {
        editor.input_snapshot(fonts, area, focus, target_id, previous, cause)
    })
    .flatten()
}
