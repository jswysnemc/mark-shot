//! 【输入法测试】【生产路由】验证正文上下文、字段切换和原生焦点边界，不用缓存身份接收候选

use super::*;
use crate::{
    app::{event_loop, text_input},
    editor::text_session::{self, PREVIEW_ID},
    types::{
        AnnotationShape, OverlayEvent,
        text_input::{Batch, Focus, Preedit, Snapshot, Update},
    },
};
use cosmic_text::{Cursor, Edit as _, Selection};

/// 【输入法测试】【原生准备】按 text 创建真实编辑器与原生焦点，返回状态；不连接显示服务器
fn prepared(text: &str) -> EditorState {
    let mut state = editor();
    state.selected_tool = crate::tools::Tool::Text;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.selection.prev_zone = state.selection.zone;
    text_session::begin_new(&mut state, (100.25, 120.5), 1.0, &mut 0);
    crate::tools::text::session_input::text(&mut state, text, &mut 0);
    text_input::event(
        &mut state,
        OverlayEvent::TextInputFocus(Focus {
            id: 9000,
            monitor_idx: 0,
        }),
        &mut 0,
    );
    state
}

/// 【输入法测试】【快照收集】同步 state 并返回实际发送的快照，空调用不记录停用
fn sync(state: &mut EditorState) -> Vec<Snapshot> {
    let mut sent = Vec::new();
    text_input::sync_with(
        state,
        |snapshot| {
            if let Some(snapshot) = snapshot {
                sent.push(snapshot.clone());
            }
            Ok(())
        },
        &mut 0,
    )
    .unwrap();
    sent
}

/// 【输入法测试】【实际交付】按 state 当前目标交付 batch，matching 控制提交序号许可，无返回值
fn deliver(state: &mut EditorState, batch: Batch, matching: bool) {
    let target_id = state.input.text_input.id.unwrap();
    event_loop::handle_event(
        state,
        OverlayEvent::TextInput(Update {
            target_id,
            serial: 1,
            sync_allowed: matching,
            batch,
        }),
        &mut 0,
    );
}

/// 【输入法测试】【正文读取】返回 state 已确认文本，不读取候选缓存
fn content(state: &EditorState) -> &str {
    let AnnotationShape::Text { content, .. } = &state.text_session.as_ref().unwrap().working.shape
    else {
        unreachable!()
    };
    content
}

/// 【输入法测试】【候选上下文】候选进入真实显示但不进入周边正文，静止和闪烁同步不复制快照，无参数、无返回值
#[test]
fn native_routes_display_preedit_without_leaking_it_into_surrounding_text() {
    let mut state = prepared("one\ntwo");
    let first = sync(&mut state).pop().unwrap();
    assert_ne!(first.target_id, PREVIEW_ID);
    assert_ne!(first.target_id, state.text_session.as_ref().unwrap().id);
    assert_eq!(first.surrounding.as_ref().unwrap().text, "two");
    assert_eq!(first.surrounding.as_ref().unwrap().document_start, 4);
    deliver(
        &mut state,
        Batch {
            preedit: Preedit {
                text: "ni".into(),
                cursor: Some((2, 2)),
            },
            ..Default::default()
        },
        true,
    );
    assert_eq!(content(&state), "one\ntwo");
    assert!(state.text_session.as_ref().unwrap().composition.is_some());
    let candidate = sync(&mut state).pop().unwrap();
    assert_eq!(candidate.surrounding, first.surrounding);
    assert_ne!(candidate.cursor_rect, first.cursor_rect);
    let pointer = state
        .input
        .text_input
        .snapshot
        .as_ref()
        .unwrap()
        .surrounding
        .as_ref()
        .unwrap()
        .text
        .as_ptr();
    for _ in 0..5 {
        assert!(sync(&mut state).is_empty());
    }
    assert_eq!(
        pointer,
        state
            .input
            .text_input
            .snapshot
            .as_ref()
            .unwrap()
            .surrounding
            .as_ref()
            .unwrap()
            .text
            .as_ptr()
    );
}

/// 【输入法测试】【序号等待】旧序号确认仍写正文但不同步上下文，后续匹配序号恢复同步，无参数、无返回值
#[test]
fn mismatched_done_updates_content_but_waits_before_protocol_resynchronization() {
    let mut state = prepared("");
    sync(&mut state);
    deliver(
        &mut state,
        Batch {
            commit: "你".into(),
            preedit: Preedit {
                text: "hao".into(),
                cursor: Some((3, 3)),
            },
            ..Default::default()
        },
        false,
    );
    assert_eq!(content(&state), "你");
    assert!(sync(&mut state).is_empty());
    assert_eq!(
        state
            .input
            .text_input
            .snapshot
            .as_ref()
            .unwrap()
            .surrounding
            .as_ref()
            .unwrap()
            .text,
        ""
    );
    deliver(
        &mut state,
        Batch {
            commit: "好".into(),
            ..Default::default()
        },
        true,
    );
    assert_eq!(content(&state), "你好");
    assert_eq!(
        sync(&mut state)[0].surrounding.as_ref().unwrap().text,
        "你好"
    );
}

/// 【输入法测试】【原生离开】普通键盘失焦不取消候选，原生 leave 才重置；旧焦点离开不能清理新候选，无参数、无返回值
#[test]
fn only_the_matching_native_leave_cancels_composition() {
    let mut state = prepared("");
    deliver(
        &mut state,
        Batch {
            preedit: Preedit {
                text: "ni".into(),
                cursor: Some((2, 2)),
            },
            ..Default::default()
        },
        true,
    );
    event_loop::handle_event(
        &mut state,
        OverlayEvent::FocusLost { monitor_idx: 0 },
        &mut 0,
    );
    assert!(state.text_session.as_ref().unwrap().composition.is_some());
    text_input::event(
        &mut state,
        OverlayEvent::TextInputLeave { focus_id: 9000 },
        &mut 0,
    );
    assert!(state.text_session.as_ref().unwrap().composition.is_none());
    text_input::event(
        &mut state,
        OverlayEvent::TextInputFocus(Focus {
            id: 9001,
            monitor_idx: 0,
        }),
        &mut 0,
    );
    deliver(
        &mut state,
        Batch {
            preedit: Preedit {
                text: "hao".into(),
                cursor: Some((3, 3)),
            },
            ..Default::default()
        },
        true,
    );
    text_input::event(
        &mut state,
        OverlayEvent::TextInputLeave { focus_id: 9000 },
        &mut 0,
    );
    assert!(state.text_session.as_ref().unwrap().composition.is_some());
}

/// 【输入法测试】【旧会话隔离】排队候选不能进入复用缓存编号的新会话，也不能穿透字体字段，无参数、无返回值
#[test]
fn delayed_payloads_do_not_cross_session_or_field_boundaries() {
    let mut state = prepared("old");
    let old = state.input.text_input.id.unwrap();
    text_session::begin_new(&mut state, (200.0, 220.0), 1.0, &mut 0);
    let stale = OverlayEvent::TextInput(Update {
        target_id: old,
        serial: 1,
        sync_allowed: true,
        batch: Batch {
            commit: "bad".into(),
            ..Default::default()
        },
    });
    text_input::event(&mut state, stale.clone(), &mut 0);
    assert_eq!(content(&state), "");
    assert_ne!(state.input.text_input.id, Some(old));
    let field_target = state.input.text_input.id.unwrap();
    state.font_panel.open = true;
    text_input::event(
        &mut state,
        OverlayEvent::TextInput(Update {
            target_id: field_target,
            serial: 1,
            sync_allowed: true,
            batch: Batch {
                commit: "wrong".into(),
                ..Default::default()
            },
        }),
        &mut 0,
    );
    assert_eq!(content(&state), "");
    assert!(state.input.text_input.id.is_none());
    state.font_panel.open = false;
    sync(&mut state);
    assert_ne!(state.input.text_input.id, Some(field_target));
    text_input::event(&mut state, stale, &mut 0);
    assert_eq!(content(&state), "");
}

/// 【输入法测试】【跨段落选区】Qt 负数段落锚点规范化为协议完整文档字节，光标矩形采用表面逻辑坐标，无参数、无返回值
#[test]
fn cross_paragraph_context_and_fractional_surface_cursor_use_valid_logical_offsets() {
    let mut state = prepared("one\ntwo\nthree");
    state.placements[0].position = (50, 80);
    state.placements[0].scale = 1.2;
    let editor = state.text.editors.get_mut(&PREVIEW_ID).unwrap();
    editor.set_cursor(Cursor::new(2, 2));
    editor.set_selection(Selection::Normal(Cursor::new(0, 2)));
    let value = sync(&mut state).pop().unwrap();
    let context = value.surrounding.unwrap();
    assert_eq!(context.text, "one\ntwo\nthree");
    assert_eq!((context.cursor, context.anchor), (10, 2));
    let rect = value.cursor_rect.unwrap();
    assert!(rect.0 >= 50 && rect.0 < 200 && rect.1 >= 40 && rect.1 < 200);
    let old = rect;
    state.placements[0].scale = 2.0;
    assert!(sync(&mut state).is_empty());
    assert_eq!(
        state
            .input
            .text_input
            .snapshot
            .as_ref()
            .unwrap()
            .cursor_rect,
        Some(old)
    );
}
