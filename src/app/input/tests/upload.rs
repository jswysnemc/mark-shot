use super::*;
use crate::{
    app::{actions, event_loop},
    keys::Action,
    types::{MouseButton, OverlayEvent},
    ui::toolbar::{Integration, ToolbarButton, ToolbarItem},
};

/// 【上传输入测试】【按钮可用】上传按钮已经映射真实动作，原版右键不能启动，左键同位置释放才请求上传
#[test]
fn upload_toolbar_is_enabled_and_requires_left_release() {
    let mut state = editor();
    state.selection.zone = tiny_skia::Rect::from_xywh(30.0, 40.0, 200.0, 180.0);
    let mut dirty = 0;
    crate::app::panels::toolbar::update_toolbar(&mut state, &mut dirty);
    let index = state
        .action_toolbar
        .items
        .iter()
        .position(|item| {
            *item == ToolbarItem::Button(ToolbarButton::Integration(Integration::Upload))
        })
        .unwrap();
    assert!(!state.action_toolbar.disabled.contains(&index));
    let cell = state.action_toolbar.cells[index];
    let point = (
        state.action_toolbar.render_pos.0 + cell.left() + cell.width() / 2.0,
        state.action_toolbar.render_pos.1 + cell.top() + cell.height() / 2.0,
    );
    pointer(
        &mut state,
        f64::from(point.0),
        f64::from(point.1),
        &mut dirty,
    );
    super::super::handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    super::super::handle_pointer_button(&mut state, MouseButton::Right, false, &mut dirty);
    assert!(!state.upload.requested);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert!(!state.upload.requested);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.upload.requested);
}

/// 【上传输入测试】【忙碌隔离】实际事件分派阻止上传中画布、滚轮和快捷键修改，取消恢复编辑且不完成常规导出
#[test]
fn upload_busy_blocks_actual_editor_events_and_cancel_preserves_canvas() {
    let mut state = editor();
    state.selection.zone = tiny_skia::Rect::from_xywh(30.0, 40.0, 200.0, 180.0);
    let selection = state.selection.zone;
    let mut dirty = 0;
    actions::run(&mut state, Action::Upload, &mut dirty);
    assert!(state.upload.busy());
    assert_eq!(
        super::super::compute_cursor(&state),
        crate::types::CursorIcon::Progress
    );
    event_loop::handle_event(
        &mut state,
        OverlayEvent::PointerMove {
            monitor_idx: 0,
            x: 400.0,
            y: 400.0,
            history: Vec::new(),
        },
        &mut dirty,
    );
    event_loop::handle_event(
        &mut state,
        OverlayEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            serial: None,
            timestamp: None,
        },
        &mut dirty,
    );
    event_loop::handle_event(
        &mut state,
        OverlayEvent::Key {
            chord: Some(crate::keys::parse("Ctrl+S").unwrap()),
            text: None,
        },
        &mut dirty,
    );
    assert_eq!(state.selection.zone, selection);
    assert!(!state.input.mouse_down);
    assert!(state.finish.is_none() && state.annotations.is_empty() && state.pending.is_none());
    assert!(
        crate::app::event_loop::poll_timeout(&state).is_none(),
        "waiting for upload must not start periodic polling"
    );
    actions::run(&mut state, Action::Cancel, &mut dirty);
    assert!(!state.upload.busy() && !state.cancel);
}

/// 【上传输入测试】【原版结果】按实际 Qt 样本验证成功、失败、复制失败与超时的提示、关闭期限和冻结选区
#[test]
fn upload_completion_matches_original_qt_window_contract() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../../tests/fixtures/qt-upload.json")).unwrap();
    for case in fixture["results"].as_array().unwrap() {
        let mut state = editor();
        state.selection.zone = tiny_skia::Rect::from_xywh(30.0, 40.0, 200.0, 180.0);
        let selection = state.selection.zone;
        let kind = case["kind"].as_str().unwrap();
        let result = match kind {
            "empty" | "failed" => Err(crate::upload::Error {
                kind: crate::upload::ErrorKind::Protocol,
                detail: "Upload failed".into(),
            }),
            "timeout" => Err(crate::upload::Error {
                kind: crate::upload::ErrorKind::Timeout,
                detail: "Upload timed out".into(),
            }),
            _ => Ok(crate::upload::Output {
                url: case["url"].as_str().unwrap().into(),
                delete_url: String::new(),
            }),
        };
        let mut copied = String::new();
        crate::app::upload::complete(&mut state, result, &mut 0, |text| {
            copied = text.to_owned();
            if kind == "copy-failed" {
                Err("isolated copy failure".into())
            } else {
                Ok(())
            }
        });
        assert_eq!(copied, case["url"].as_str().unwrap());
        assert_eq!(state.upload.copied, case["closed"].as_bool().unwrap());
        assert_eq!(
            state.upload.deadline().is_some(),
            case["closed"].as_bool().unwrap()
        );
        assert_eq!(
            state.toasts.items.last().unwrap().text,
            case["toast"].as_str().unwrap()
        );
        assert_eq!(state.selection.zone, selection);
        assert!(state.finish.is_none() && state.annotations.is_empty() && !state.cancel);
        assert!(case["input_removed"].as_bool().unwrap() && !case["busy"].as_bool().unwrap());
    }
}

/// 【上传输入测试】【提示停留】完整显示的失败提示只等待四秒到期，不在稳定画面上维持每帧轮询
#[test]
fn stable_upload_feedback_waits_for_expiration_instead_of_frame_polling() {
    let mut state = editor();
    crate::app::upload::complete(
        &mut state,
        Err(crate::upload::Error {
            kind: crate::upload::ErrorKind::Protocol,
            detail: "Upload failed".into(),
        }),
        &mut 0,
        |_| Ok(()),
    );
    state.toasts.items[0].progress = 1.0;
    assert!(
        !state.toasts.is_animating(),
        "visible stable toast must not request animation frames"
    );
    let wait =
        crate::app::event_loop::poll_timeout(&state).expect("timed toast must have a deadline");
    assert!(
        wait > std::time::Duration::from_secs(3) && wait <= std::time::Duration::from_secs(4),
        "stable toast must block until expiration: {wait:?}"
    );
    state.toasts.items[0].shown_at -= std::time::Duration::from_secs(5);
    assert_eq!(
        crate::app::event_loop::poll_timeout(&state),
        Some(std::time::Duration::ZERO)
    );
    crate::app::event_loop::tick_panels(&mut state, &mut 0);
    assert!(state.toasts.items[0].closing && state.toasts.is_animating());
    assert!(state.toasts.deadline().is_none());
    state.toasts.items[0].progress = 0.0;
    crate::app::event_loop::tick_panels(&mut state, &mut 0);
    assert!(state.toasts.items.is_empty());
    assert!(crate::app::event_loop::poll_timeout(&state).is_none());
}
