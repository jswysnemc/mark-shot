//! 【输入法】【确认事务】先验证整个替换范围，再一次记录选区删除和确认文字，候选变动不记录正文历史

use cosmic_text::{Cursor, Edit as _, Selection};

use crate::{
    editor::{
        EditorState, text_session::PREVIEW_ID, text_session_format::Font,
        text_session_history::Kind,
    },
    features::text::input_method::positions,
    types::{AnnotationShape, text_input::Edit},
};

pub(super) struct Plan {
    document: String,
    range: (Cursor, Cursor),
    visible_cursor: Cursor,
    editing: bool,
}

/// 【输入法】【范围准备】按 state 与 event 返回完整替换计划；先验证候选和全部字节边界，非法事件不修改正文或原候选
pub(super) fn prepare(state: &EditorState, event: &Edit) -> Option<Plan> {
    if let Some((begin, end)) = event.preedit.cursor
        && (!event.preedit.text.is_char_boundary(begin)
            || !event.preedit.text.is_char_boundary(end))
    {
        return None;
    }
    let session = state.text_session.as_ref()?;
    let AnnotationShape::Text { content, .. } = &session.working.shape else {
        return None;
    };
    let editor = state.text.editors.get(&PREVIEW_ID)?;
    let mut document = content.clone();
    let mut cursor = positions::document_offset(content, editor.cursor())?;
    let editing = !event.preedit.text.is_empty()
        || !event.commit.is_empty()
        || event
            .replacement
            .is_some_and(|replacement| replacement.length != 0);
    // 1. 【输入法】【选区语义】非空输入先删除选区，替换偏移从收起后的光标计算；空事件保持原选区
    if editing && let Some((start, end)) = editor.selection_bounds() {
        let start = positions::document_offset(content, start)?;
        let end = positions::document_offset(content, end)?;
        document.replace_range(start..end, "");
        cursor = start;
    }
    let replacement = event
        .replacement
        .unwrap_or(crate::types::text_input::Replacement {
            start: 0,
            length: 0,
        });
    let start = cursor.checked_add_signed(replacement.start)?;
    let end = start.checked_add(replacement.length)?;
    let range = (
        positions::editor_cursor(&document, start)?,
        positions::editor_cursor(&document, end)?,
    );
    let commit = event.commit.replace("\r\n", "\n").replace('\r', "\n");
    document.replace_range(start..end, &commit);
    // 2. 【输入法】【活光标位置】光标在替换之前保持位置，在替换之后随长度变化移动，范围内落在确认文字末尾
    let next = if cursor < start {
        cursor
    } else if cursor > end {
        cursor.checked_sub(end - start)?.checked_add(commit.len())?
    } else {
        start.checked_add(commit.len())?
    };
    Some(Plan {
        visible_cursor: positions::editor_cursor(&document, next)?,
        document,
        range,
        editing,
    })
}

/// 【输入法】【确认执行】将 event 与已验证 plan 应用于 state，dirty 返回动态范围；每次实际文档修改独立撤销，不复制候选到正文
pub(super) fn confirm(state: &mut EditorState, event: &Edit, plan: Plan, dirty: &mut u32) {
    if !plan.editing {
        return;
    }
    let Some(session) = state.text_session.as_mut() else {
        return;
    };
    let Some(editor) = state.text.editors.get_mut(&PREVIEW_ID) else {
        return;
    };
    let before = (editor.cursor(), Selection::None);
    editor.start_change();
    editor.delete_selection();
    if !event.commit.is_empty()
        || event
            .replacement
            .is_some_and(|replacement| replacement.length != 0)
    {
        editor.delete_range(plan.range.0, plan.range.1);
        editor.set_cursor(plan.range.0);
        let commit = event.commit.replace("\r\n", "\n").replace('\r', "\n");
        if !commit.is_empty() {
            editor.insert_string(&commit, None);
        }
    }
    let after = (editor.cursor(), Selection::None);
    let change = editor.finish_change();
    session.history.record(
        change,
        Font::read(&session.working),
        before,
        after,
        Kind::Standalone,
    );
    editor.set_cursor(plan.visible_cursor);
    editor.set_selection(Selection::None);
    super::preview::damage(&session.working, &mut state.damage_rects);
    if let AnnotationShape::Text { content, .. } = &mut session.working.shape {
        *content = plan.document;
    }
    crate::tools::text::reflow_text_editor(
        &mut session.working,
        editor,
        &mut state.text.font_system,
    );
    session.caret.activity();
    super::preview::damage(&session.working, &mut state.damage_rects);
    crate::editor::dirty::apply_damage_rects(state, dirty);
}
