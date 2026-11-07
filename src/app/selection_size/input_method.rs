//! 【选区尺寸】【原生输入法】当前字段使用普通文字内容类型，确认整批验证，候选与实际选区分开同步

use crate::{
    editor::EditorState,
    features::text::input_method::surrounding,
    types::text_input::{ContentType, Edit, Focus, Snapshot},
    ui::selection_size::field_draw,
};
use tiny_skia::Rect;

#[derive(Clone, Copy)]
pub(in crate::app) struct Layout {
    id: u64,
    revision: u64,
    rect: Rect,
    caret: (i32, i32, i32, i32),
}

/// 【选区尺寸】【输入身份】按 state 返回可见活动字段身份和显示器，没有字段时返回空值
pub(in crate::app) fn identity(state: &EditorState) -> Option<(u64, usize)> {
    if state.selection_size.popup.is_some() {
        return None;
    }
    let panel = state
        .selection_size
        .panel
        .as_ref()
        .filter(|panel| panel.rect.is_some())?;
    Some((panel.field(panel.editing()?).id, panel.monitor))
}

/// 【选区尺寸】【输入批次】按 event 原子更新 state 字段，dirty 返回候选或文字变化，返回是否有字段拥有者
pub(in crate::app) fn apply(state: &mut EditorState, event: Edit, dirty: &mut u32) -> bool {
    if identity(state).is_none() {
        return false;
    }
    let panel = state.selection_size.panel.as_mut().unwrap();
    let side = panel.editing().unwrap();
    let update = panel.field_mut(side).input_method(event);
    if update.changed {
        super::panels::damage(state, dirty);
        super::caret::activity(state, dirty);
    }
    true
}

/// 【选区尺寸】【候选取消】清理 state 两个字段候选，dirty 返回可见区域，不取消待提交文字，无返回值
pub(in crate::app) fn cancel(state: &mut EditorState, dirty: &mut u32) {
    let mut changed = false;
    if let Some(panel) = &mut state.selection_size.panel {
        for field in &mut panel.fields {
            changed |= field.editor.cancel_preedit();
        }
    }
    if changed {
        super::panels::damage(state, dirty);
    }
}

/// 【选区尺寸】【原生快照】按 state、focus 和 target_id 返回周边正文与居中光标，只在实际上下文变化后分配快照
pub(in crate::app) fn snapshot(
    state: &mut EditorState,
    focus: Focus,
    target_id: u64,
) -> Option<Snapshot> {
    let (id, monitor) = identity(state)?;
    if monitor != focus.monitor_idx {
        return None;
    }
    let panel = state.selection_size.panel.as_mut()?;
    let side = panel.editing()?;
    let rect = panel.field_rect(side)?;
    let field =
        &panel.fields[usize::from(side == crate::features::selection_size::ratio::Field::Height)];
    let cached = state.selection_size.input_layout.filter(|layout| {
        layout.id == id && layout.revision == field.editor.revision && layout.rect == rect
    });
    let caret = cached.map_or_else(
        || {
            field_draw::caret(
                &field_draw::shape(field, rect, &mut panel.fonts, &mut panel.swash),
                rect,
            )
        },
        |layout| layout.caret,
    );
    state.selection_size.input_layout = Some(Layout {
        id,
        revision: field.editor.revision,
        rect,
        caret,
    });
    let anchor = field
        .editor
        .selection_anchor
        .map_or(field.editor.cursor_byte(), |anchor| {
            field
                .editor
                .text
                .char_indices()
                .nth(anchor)
                .map_or(field.editor.text.len(), |(byte, _)| byte)
        });
    let context = surrounding::view(&field.editor.text, field.editor.cursor_byte(), anchor);
    let target = &state.input.text_input;
    if target.snapshot.as_ref().is_some_and(|old| {
        old.target_id == target_id
            && old.focus_id == focus.id
            && old.cursor_rect == Some(caret)
            && old.content == ContentType::Normal
            && match (context, old.surrounding.as_ref()) {
                (None, None) => true,
                (Some(context), Some(old)) => context.matches(old),
                _ => false,
            }
    }) {
        return None;
    }
    Some(Snapshot {
        target_id,
        focus_id: focus.id,
        monitor_idx: monitor,
        surrounding: context.map(surrounding::Window::owned),
        cursor_rect: Some(caret),
        cause: target.cause,
        content: ContentType::Normal,
    })
}
