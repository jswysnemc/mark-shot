//! 【输入法】【编辑衔接】正文焦点与协议焦点共同确定目标，候选显示位置与已确认上下文分别同步

use crate::{
    backend::ScreenOverlay,
    editor::{EditorState, text_composition, text_session::PREVIEW_ID},
    features::text::input_method::{
        positions,
        surrounding::{self, Window},
    },
    types::{
        AnnotationShape, OverlayEvent,
        text_input::{ChangeCause, Snapshot},
    },
    ui::panel::UiPanel,
};
use cosmic_text::{Edit as _, Selection};
pub(crate) mod fields;

/// 【输入法】【正文资格】按 state 返回可输入正文的会话身份；字段、菜单和识别界面拥有输入时返回空值
fn session(state: &EditorState) -> Option<u64> {
    if let Some(id) = fields::identity(state) {
        return state
            .input
            .text_input
            .focus
            .filter(|focus| fields::monitor(state) == Some(focus.monitor_idx))
            .map(|_| id);
    }
    if state.ui_hidden
        || state.text_context_menu.is_some()
        || state.font_panel.open
        || state.color_popover.is_visible()
        || state.settings_panel.is_editing()
        || state.selected_tool == crate::tools::Tool::Ocr
    {
        return None;
    }
    state.text_session.as_ref().map(|session| session.id)
}

/// 【输入法】【目标刷新】按 state 当前正文资格更新目标，dirty 返回被取消候选范围，无返回值；不把键盘失焦等同于原生 leave
fn reconcile(state: &mut EditorState, dirty: &mut u32) {
    let session = session(state);
    if state.input.text_input.reconcile(session) {
        fields::cancel(state, dirty);
        text_composition::cancel(state, dirty);
    }
}

/// 【输入法】【协议交付】在 state 中处理 event，dirty 返回显示变化；过期会话或焦点事件不修改正文，无返回值
pub(crate) fn event(state: &mut EditorState, event: OverlayEvent, dirty: &mut u32) {
    match event {
        OverlayEvent::TextInputFocus(focus) => {
            text_composition::cancel(state, dirty);
            fields::cancel(state, dirty);
            state.input.text_input.enter(focus);
            reconcile(state, dirty);
        }
        OverlayEvent::TextInputLeave { focus_id } => {
            if state.input.text_input.leave(focus_id) {
                text_composition::cancel(state, dirty);
                fields::cancel(state, dirty);
            }
        }
        OverlayEvent::TextInput(update) => {
            reconcile(state, dirty);
            if !state.input.text_input.accept(&update) {
                return;
            }
            if let Some(edit) = crate::types::text_input::Edit::from_batch(update.batch) {
                if fields::identity(state).is_some() {
                    fields::apply(state, edit, dirty);
                    state.input.text_input.cause = ChangeCause::InputMethod;
                    return;
                }
                if !text_composition::apply(state, edit, dirty) {
                    return;
                }
                state.input.text_input.cause = ChangeCause::InputMethod;
                super::input::refresh_panels(state, dirty);
                super::text_caret::activity(state, dirty);
            }
        }
        _ => {}
    }
}

/// 【输入法】【周边正文】从 state 已确认编辑器返回借用上下文；同段落使用 Qt 段落范围，跨段落选区使用合法文档位置
fn context(state: &EditorState) -> Option<Window<'_>> {
    let session = state.text_session.as_ref()?;
    let AnnotationShape::Text { content, .. } = &session.working.shape else {
        return None;
    };
    let editor = state.text.editors.get(&PREVIEW_ID)?;
    let cursor = editor.cursor();
    let anchor = match editor.selection() {
        Selection::Normal(anchor) | Selection::Word(anchor) | Selection::Line(anchor) => anchor,
        Selection::None => cursor,
    };
    let offset = positions::document_offset(content, cursor)?;
    let anchor_offset = positions::document_offset(content, anchor)?;
    if cursor.line == anchor.line {
        let paragraph = content.split('\n').nth(cursor.line)?;
        let mut window = surrounding::view(paragraph, cursor.index, anchor.index)?;
        window.document_start += offset - cursor.index;
        Some(window)
    } else {
        surrounding::view(content, offset, anchor_offset)
    }
}

/// 【输入法】【候选位置】用 state 已显示编辑器返回 monitor_idx 表面逻辑光标矩形；不使用缓存像素比例，不能定位时返回空值
fn cursor_rect(state: &EditorState, monitor_idx: usize) -> Option<(i32, i32, i32, i32)> {
    let session = state.text_session.as_ref()?;
    let AnnotationShape::Text {
        start, appearance, ..
    } = &session.presented().shape
    else {
        return None;
    };
    let padding = crate::features::text::geometry::padding(appearance.size_scale);
    let placement = state.placements.get(monitor_idx)?;
    let editor = state.text.editors.get(&session.presented().id)?;
    let cursor = editor.cursor();
    editor.with_buffer(|buffer| {
        buffer.layout_runs().find_map(|run| {
            let x = run.cursor_position(&cursor)?;
            Some((
                (start.0 + padding.0 + x - placement.position.0 as f32).floor() as i32,
                (start.1 + padding.1 + run.line_top - placement.position.1 as f32).floor() as i32,
                1,
                run.line_height.ceil().max(1.0) as i32,
            ))
        })
    })
}

/// 【输入法】【上下文同步】按 state 当前字段同步 overlay，dirty 返回候选取消范围；闪烁帧不复制文字，旧 done 序号等待匹配后才发送状态，返回协议结果
pub(crate) fn sync(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    dirty: &mut u32,
) -> crate::AppResult {
    sync_with(state, |snapshot| overlay.set_text_input(snapshot), dirty)
}

/// 【输入法】【同步执行】按 state 生成有限快照并交给 send，dirty 返回候选取消范围；传输失败不缓存未发送状态，返回发送结果
pub(crate) fn sync_with(
    state: &mut EditorState,
    mut send: impl FnMut(Option<&Snapshot>) -> crate::AppResult,
    dirty: &mut u32,
) -> crate::AppResult {
    reconcile(state, dirty);
    let target = &state.input.text_input;
    let (Some(target_id), Some(focus)) = (target.id, target.focus) else {
        return send(None);
    };
    if !target.sync_allowed {
        return Ok(());
    }
    if fields::identity(state).is_some() {
        if let Some(next) = fields::snapshot(state, focus, target_id) {
            send(Some(&next))?;
            state.input.text_input.snapshot = Some(next);
            state.input.text_input.cause = ChangeCause::Other;
        }
        return Ok(());
    }
    let context = context(state);
    let rect = cursor_rect(state, focus.monitor_idx);
    let unchanged = target.snapshot.as_ref().is_some_and(|old| {
        old.target_id == target_id
            && old.focus_id == focus.id
            && old.cursor_rect == rect
            && match (context, old.surrounding.as_ref()) {
                (None, None) => true,
                (Some(context), Some(old)) => context.matches(old),
                _ => false,
            }
    });
    if unchanged {
        return Ok(());
    }
    let next = Snapshot {
        target_id,
        focus_id: focus.id,
        monitor_idx: focus.monitor_idx,
        surrounding: context.map(Window::owned),
        cursor_rect: rect,
        cause: target.cause,
        content: crate::types::text_input::ContentType::Multiline,
    };
    send(Some(&next))?;
    state.input.text_input.snapshot = Some(next);
    state.input.text_input.cause = ChangeCause::Other;
    Ok(())
}
