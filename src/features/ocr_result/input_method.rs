//! 【文字识别】【原生输入】只向当前可编辑原文或单元格分派，菜单、原图和失焦窗口不接受候选

use super::window::{ResultWindow, View};

#[cfg(test)]
mod tests;
use crate::{
    backend::wayland::window::WindowAction,
    types::text_input::{ChangeCause, Edit, Focus, Snapshot},
};

/// 【文字识别】【输入身份】返回 state 当前可见编辑器的稳定身份，非编辑页面或失焦窗口返回空值
pub(super) fn identity(state: &ResultWindow) -> Option<u64> {
    if !state.focused
        || state.menu
        || state.source_menu.is_some()
        || state.focus_target != super::control_input::FocusTarget::Content
    {
        return None;
    }
    match state.view {
        View::Text => Some(state.source.input_identity()),
        View::Table => state
            .table
            .as_ref()?
            .editor
            .as_ref()
            .map(|editor| editor.input_identity()),
        View::Image => None,
    }
}

/// 【文字识别】【批次分派】将 edit 交给 state 当前编辑器，返回窗口动作；候选不写表格，只有正文变化重置复制状态
pub(super) fn apply(state: &mut ResultWindow, edit: Edit) -> WindowAction {
    if identity(state).is_none() {
        return WindowAction::None;
    }
    state.ensure_fonts(&edit.commit);
    state.ensure_fonts(&edit.preedit.text);
    let update = match state.view {
        View::Text => state.source.input_method(&mut state.fonts, edit),
        View::Table => state
            .table
            .as_mut()
            .unwrap()
            .editor
            .as_mut()
            .unwrap()
            .input_method(&mut state.fonts, edit),
        View::Image => return WindowAction::None,
    };
    if update.text_changed {
        state.status = None;
        state.copied = false;
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
    let table = state
        .table
        .as_mut()
        .and_then(|table| table.editor.as_mut())
        .is_some_and(|editor| editor.cancel_preedit());
    source || table
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
    match state.view {
        View::Text => state.source.input_snapshot(
            &mut state.fonts,
            crate::ui::text_editor::InputArea {
                rect: state.layout.editor,
                offset: 0.0,
            },
            focus,
            target_id,
            previous,
            cause,
        ),
        View::Table => {
            let table = state.table.as_mut()?;
            let rect = table.input_rect(state.layout.body)?;
            table.editor.as_mut()?.input_snapshot(
                &mut state.fonts,
                crate::ui::text_editor::InputArea {
                    rect,
                    offset: table.editor_offset,
                },
                focus,
                target_id,
                previous,
                cause,
            )
        }
        View::Image => None,
    }
}
