//! 【选区尺寸】【主选择编辑】真实释放时复制选择，中键先取消选择再异步粘贴，过期请求不能修改新字段

use crate::{
    backend::ScreenOverlay,
    editor::EditorState,
    features::selection_size::ratio::Field,
    types::{MouseButton, OverlayEvent},
};

#[derive(Clone, Copy)]
struct Pointer {
    field: Field,
    session: u64,
}

struct Paste {
    id: u64,
    field: Field,
    identity: u64,
    revision: u64,
    session: u64,
    text: String,
    cursor: usize,
    anchor: Option<usize>,
    sent: bool,
}

#[derive(Default)]
pub(super) struct Clipboard {
    supported: Option<bool>,
    pointer: Option<Pointer>,
    copy: Option<String>,
    paste: Option<Paste>,
    cancel: Option<u64>,
}

/// 【选区尺寸】【字段按下】按 side 记录 state 已显示字段的编辑身份，无返回值，首次打开字段的释放不复制全文
pub(super) fn press(state: &mut EditorState, side: Field) {
    state.selection_size.clipboard.pointer =
        state.selection_size.panel.as_ref().and_then(|panel| {
            let field = panel.field(side);
            field.editing.then_some(Pointer {
                field: side,
                session: field.session,
            })
        });
}

/// 【选区尺寸】【字段释放】按 button 在 state 复制选择或准备中键粘贴，dirty 返回选择变化，跨输出释放仍归原编辑身份
pub(super) fn release(state: &mut EditorState, button: MouseButton, dirty: &mut u32) {
    let Some(pointer) = state.selection_size.clipboard.pointer.take() else {
        return;
    };
    if state.selection_size.clipboard.supported != Some(true) {
        return;
    }
    let Some(panel) = &mut state.selection_size.panel else {
        return;
    };
    let field = panel.field_mut(pointer.field);
    if !field.editing || field.session != pointer.session {
        return;
    }
    match button {
        MouseButton::Left => {
            state.selection_size.clipboard.copy =
                field.editor.selected_text().filter(|text| !text.is_empty());
        }
        MouseButton::Middle => {
            // 1. 【选区尺寸】【中键释放】原版先取消选择，粘贴以当前光标插入，失败仍保持取消后的选择
            field.editor.selection_anchor = None;
            field.editor.cancel_preedit();
            state.selection_size.clipboard.paste = Some(Paste {
                id: crate::features::text::input_method::focus::identity(),
                field: pointer.field,
                identity: field.id,
                revision: field.editor.revision,
                session: field.session,
                text: field.editor.text.clone(),
                cursor: field.editor.cursor,
                anchor: field.editor.selection_anchor,
                sent: false,
            });
            super::panels::damage(state, dirty);
            super::caret::activity(state, dirty);
        }
        MouseButton::Right => {}
    }
}

/// 【选区尺寸】【接收身份】按 state 与 paste 核对编辑会话、正文、光标及候选，返回请求是否仍然有效
fn valid(state: &EditorState, paste: &Paste) -> bool {
    state.selection_size.panel.as_ref().is_some_and(|panel| {
        let field = panel.field(paste.field);
        panel.rect.is_some()
            && field.editing
            && field.id == paste.identity
            && field.session == paste.session
            && field.editor.text == paste.text
            && field.editor.cursor == paste.cursor
            && field.editor.selection_anchor == paste.anchor
            && field.editor.revision == paste.revision
            && field.editor.composition.is_none()
    })
}

/// 【选区尺寸】【取消旧接收】清理 state 当前粘贴并记录待取消管道，无返回值，来源复制不受影响
fn cancel(state: &mut EditorState) {
    if let Some(paste) = state
        .selection_size
        .clipboard
        .paste
        .take()
        .filter(|paste| paste.sent)
    {
        state.selection_size.clipboard.cancel = Some(paste.id);
    }
}

/// 【选区尺寸】【接收事件】按 event 处理 state 异步结果或取消后续编辑前的旧请求，dirty 返回原子变化，返回是否消费事件
pub(in crate::app) fn event(
    state: &mut EditorState,
    event: &OverlayEvent,
    dirty: &mut u32,
) -> bool {
    if let OverlayEvent::PrimaryText { id, text } = event {
        let current = state.selection_size.clipboard.paste.as_ref();
        if current.is_some_and(|paste| paste.id == *id && valid(state, paste)) {
            let paste = state.selection_size.clipboard.paste.take().unwrap();
            if let Some(text) = text {
                state
                    .selection_size
                    .panel
                    .as_mut()
                    .unwrap()
                    .field_mut(paste.field)
                    .insert(text);
                super::panels::damage(state, dirty);
                super::caret::activity(state, dirty);
            }
        } else if current.is_some_and(|paste| paste.id == *id) {
            cancel(state);
        }
        return true;
    }
    if matches!(
        event,
        OverlayEvent::Key { .. }
            | OverlayEvent::Scroll(_)
            | OverlayEvent::PointerButton { .. }
            | OverlayEvent::FocusLost { .. }
            | OverlayEvent::TextInput(_)
            | OverlayEvent::EscapePressed
    ) {
        cancel(state);
    }
    false
}

/// 【选区尺寸】【主选择同步】为 state 活动字段按需启用 overlay，发送复制和粘贴请求，无返回值，不创建额外连接或守护进程
pub(in crate::app) fn sync(state: &mut EditorState, overlay: &mut dyn ScreenOverlay) {
    if state.selection_size.clipboard.supported.is_none()
        && state
            .selection_size
            .panel
            .as_ref()
            .is_some_and(|panel| panel.editing().is_some())
    {
        state.selection_size.clipboard.supported = Some(overlay.enable_primary_selection());
    }
    if state
        .selection_size
        .clipboard
        .paste
        .as_ref()
        .is_some_and(|paste| !valid(state, paste))
    {
        cancel(state);
    }
    if let Some(id) = state.selection_size.clipboard.cancel.take() {
        overlay.cancel_primary_text(id);
    }
    if let Some(text) = state.selection_size.clipboard.copy.take() {
        overlay.set_primary_text(&text);
    }
    if let Some(paste) = &mut state.selection_size.clipboard.paste
        && !paste.sent
    {
        paste.sent = true;
        overlay.request_primary_text(paste.id, 32767 * 4 + 3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【主选择编辑测试】【真实字段】创建有实际选区与编辑身份的尺寸字段，按 supported 配置原生能力，返回编辑器
    fn editing(supported: bool) -> EditorState {
        let mut state = crate::app::input::tests::editor();
        state.selection.zone = tiny_skia::Rect::from_xywh(30.0, 40.0, 300.0, 180.0);
        super::super::panels::update(&mut state, &mut 0);
        super::super::keyboard::focus(&mut state, Field::Width, &mut 0);
        state.selection_size.panel.as_mut().unwrap().fields[0].editor =
            crate::ui::text_field::LineEditState::new("120".into());
        state.selection_size.clipboard.supported = Some(supported);
        state
    }

    /// 【主选择编辑测试】【粘贴身份】为 state 字段按下并释放中键，返回实际请求身份，不绕过字段接收校验
    fn request(state: &mut EditorState) -> u64 {
        press(state, Field::Width);
        release(state, MouseButton::Middle, &mut 0);
        state.selection_size.clipboard.paste.as_ref().unwrap().id
    }

    /// 【主选择编辑测试】【原子粘贴】中键先取消选择再插入正文，非法来源整体拒绝，原选区尺寸保持不变
    #[test]
    fn middle_paste_deselects_then_inserts_and_rejects_invalid_text() {
        let mut state = editing(true);
        let field = &mut state.selection_size.panel.as_mut().unwrap().fields[0];
        field.maximum = 1280;
        field.editor.cursor = 0;
        field.editor.selection_anchor = Some(3);
        let id = request(&mut state);
        assert_eq!(
            state.selection_size.panel.as_ref().unwrap().fields[0]
                .editor
                .selection_anchor,
            None
        );
        let zone = state.selection.zone;
        assert!(event(
            &mut state,
            &OverlayEvent::PrimaryText {
                id,
                text: Some("9".into())
            },
            &mut 0
        ));
        assert_eq!(
            state.selection_size.panel.as_ref().unwrap().fields[0]
                .editor
                .text,
            "9120"
        );
        assert_eq!(state.selection.zone, zone);
        let id = request(&mut state);
        event(
            &mut state,
            &OverlayEvent::PrimaryText {
                id,
                text: Some("not a number".into()),
            },
            &mut 0,
        );
        assert_eq!(
            state.selection_size.panel.as_ref().unwrap().fields[0]
                .editor
                .text,
            "9120"
        );
    }

    /// 【主选择编辑测试】【过期结果】关闭重开、程序改写或用户按键后丢弃旧接收结果，不修改新的编辑状态
    #[test]
    fn stale_paste_is_rejected_after_session_revision_or_input_changes() {
        for change in 0..3 {
            let mut state = editing(true);
            let id = request(&mut state);
            state.selection_size.clipboard.paste.as_mut().unwrap().sent = true;
            match change {
                0 => {
                    let field = &mut state.selection_size.panel.as_mut().unwrap().fields[0];
                    field.cancel();
                    field.begin();
                }
                1 => {
                    state.selection_size.panel.as_mut().unwrap().fields[0]
                        .editor
                        .select_all();
                }
                _ => {
                    event(
                        &mut state,
                        &OverlayEvent::Key {
                            chord: None,
                            text: Some("2".into()),
                        },
                        &mut 0,
                    );
                }
            }
            let before = state.selection_size.panel.as_ref().unwrap().fields[0]
                .editor
                .text
                .clone();
            event(
                &mut state,
                &OverlayEvent::PrimaryText {
                    id,
                    text: Some("9".into()),
                },
                &mut 0,
            );
            assert_eq!(
                state.selection_size.panel.as_ref().unwrap().fields[0]
                    .editor
                    .text,
                before
            );
            assert_eq!(state.selection_size.clipboard.cancel, Some(id));
        }
    }

    /// 【主选择编辑测试】【释放归属】只有已显示字段的有效左键释放复制选择，首次打开和不支持协议的中键不复制或取消选择
    #[test]
    fn release_requires_existing_session_and_supported_primary_protocol() {
        let mut state = editing(true);
        state.selection_size.panel.as_mut().unwrap().fields[0].cancel();
        press(&mut state, Field::Width);
        state.selection_size.panel.as_mut().unwrap().fields[0].begin();
        release(&mut state, MouseButton::Left, &mut 0);
        assert!(state.selection_size.clipboard.copy.is_none());
        press(&mut state, Field::Width);
        release(&mut state, MouseButton::Left, &mut 0);
        assert_eq!(state.selection_size.clipboard.copy.as_deref(), Some("300"));
        let mut state = editing(false);
        state.selection_size.panel.as_mut().unwrap().fields[0]
            .editor
            .select_all();
        press(&mut state, Field::Width);
        release(&mut state, MouseButton::Middle, &mut 0);
        assert!(state.selection_size.clipboard.paste.is_none());
        assert_eq!(
            state.selection_size.panel.as_ref().unwrap().fields[0]
                .editor
                .selected_text()
                .as_deref(),
            Some("120")
        );
    }
}
