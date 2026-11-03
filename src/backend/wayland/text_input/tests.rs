//! 【输入法测试】【原生请求】核验实际协议请求规划、上下文重置与对象级提交序号

use super::state::{Request, State};
use crate::types::text_input::{ChangeCause, ContentType, Snapshot, Surrounding};

/// 【输入法测试】【快照构造】返回 target_id 的最小协议状态，不连接合成器
fn snapshot(target_id: u64) -> Snapshot {
    Snapshot {
        target_id,
        focus_id: 20,
        monitor_idx: 0,
        surrounding: Some(Surrounding {
            text: "text".into(),
            cursor: 4,
            anchor: 4,
            document_start: 0,
        }),
        cursor_rect: Some((10, 20, 1, 24)),
        cause: ChangeCause::Other,
        content: ContentType::Multiline,
    }
}

/// 【输入法测试】【初次启用】核验启用后完整发送上下文，重复快照不提交，不影响序号，无参数、无返回值
#[test]
fn native_state_enables_once_and_identical_frames_send_no_requests() {
    let mut state = State::default();
    let value = snapshot(1);
    assert_eq!(
        state.sync(Some(&value)),
        vec![
            Request::Enable,
            Request::ContentType(ContentType::Multiline),
            Request::Surrounding(value.surrounding.as_ref().unwrap()),
            Request::Cause(ChangeCause::Other),
            Request::Cursor((10, 20, 1, 24)),
            Request::Commit
        ]
    );
    assert_eq!(state.commit_count, 1);
    assert!(state.sync(Some(&value)).is_empty());
    assert_eq!(state.commit_count, 1);
}

/// 【输入法测试】【能力变更】上下文支持变为不支持时先停用再启用，旧上下文和候选不能残留，无参数、无返回值
#[test]
fn native_context_support_changes_reset_previous_protocol_state() {
    let mut state = State::default();
    let mut value = snapshot(1);
    state.sync(Some(&value));
    state.pending.commit(Some("old".into()));
    value.surrounding = None;
    assert_eq!(
        state.sync(Some(&value)),
        vec![
            Request::Disable,
            Request::Commit,
            Request::Enable,
            Request::ContentType(ContentType::Multiline),
            Request::Cause(ChangeCause::Other),
            Request::Cursor((10, 20, 1, 24)),
            Request::Commit
        ]
    );
    assert_eq!(state.commit_count, 3);
    assert!(state.pending.done(1, 3, 3).batch.commit.is_empty());
    value.surrounding = snapshot(1).surrounding;
    let requests = state.sync(Some(&value));
    assert_eq!(
        &requests[..4],
        &[
            Request::Disable,
            Request::Commit,
            Request::Enable,
            Request::ContentType(ContentType::Multiline)
        ]
    );
    assert_eq!(state.commit_count, 5);
}

/// 【输入法测试】【目标隔离】新字段先关闭旧字段，leave 重置不重置对象序号，溢出使用协议整数回绕，无参数、无返回值
#[test]
fn new_targets_clear_pending_content_but_preserve_object_serials() {
    let mut state = State::default();
    state.sync(Some(&snapshot(1)));
    state.pending.commit(Some("old".into()));
    let next = snapshot(2);
    assert_eq!(
        &state.sync(Some(&next))[..4],
        &[
            Request::Disable,
            Request::Commit,
            Request::Enable,
            Request::ContentType(ContentType::Multiline)
        ]
    );
    assert_eq!(state.commit_count, 3);
    assert!(state.pending.done(2, 3, 3).batch.commit.is_empty());
    state.reset();
    assert_eq!(state.commit_count, 3);
    assert!(state.sync(None).is_empty());
    state.commit_count = u32::MAX;
    state.sync(Some(&next));
    assert_eq!(state.commit_count, 1);
}

/// 【输入法测试】【光标同步】仅光标几何变化不重发周边文字，停用单独提交且不重复，无参数、无返回值
#[test]
fn cursor_only_changes_do_not_copy_or_resend_surrounding_text() {
    let mut state = State::default();
    let mut value = snapshot(1);
    state.sync(Some(&value));
    value.cursor_rect = Some((11, 22, 1, 24));
    assert_eq!(
        state.sync(Some(&value)),
        vec![
            Request::Cause(ChangeCause::Other),
            Request::Cursor((11, 22, 1, 24)),
            Request::Commit
        ]
    );
    assert_eq!(state.sync(None), vec![Request::Disable, Request::Commit]);
    assert!(state.sync(None).is_empty());
}

/// 【输入法测试】【排队旧批次】done 使用它所知提交序号定位原目标，不按收到事件时的新目标分派，无参数、无返回值
#[test]
fn done_serial_keeps_the_original_target_after_a_new_session_is_enabled() {
    let mut state = State::default();
    state.sync(Some(&snapshot(1)));
    state.sync(Some(&snapshot(2)));
    assert_eq!(state.target_for(1), Some(1));
    assert_eq!(state.target_for(2), None);
    assert_eq!(state.target_for(3), Some(2));
    let mut next = snapshot(2);
    for _ in 0..500 {
        next.cursor_rect.as_mut().unwrap().0 += 1;
        state.sync(Some(&next));
    }
    assert_eq!(state.target_for(3), Some(2));
    state.reset();
    state.sync(Some(&snapshot(4)));
    assert_eq!(state.target_for(503), Some(2));
    assert_eq!(state.target_for(504), None);
    assert_eq!(state.target_for(505), Some(4));
}

/// 【输入法测试】【序号回绕】对象提交计数跨过整数上限仍保留原目标与停用边界，无参数、无返回值
#[test]
fn target_epoch_lookup_survives_commit_serial_wraparound() {
    let mut state = State::default();
    state.commit_count = u32::MAX - 2;
    let mut first = snapshot(1);
    state.sync(Some(&first));
    first.cursor_rect.as_mut().unwrap().0 += 1;
    state.sync(Some(&first));
    state.sync(Some(&snapshot(2)));
    assert_eq!(state.commit_count, 1);
    assert_eq!(state.target_for(u32::MAX - 1), Some(1));
    assert_eq!(state.target_for(u32::MAX), Some(1));
    assert_eq!(state.target_for(0), None);
    assert_eq!(state.target_for(1), Some(2));
}

/// 【输入法测试】【离开恢复】原生 leave 后重新 enter 先停用旧状态，再启用新字段；不依赖合成器自动清除启用标志，无参数、无返回值
#[test]
fn native_focus_return_disables_the_old_enabled_object_before_reenabling() {
    let mut state = State::default();
    state.sync(Some(&snapshot(1)));
    state.reset();
    assert!(state.sync(None).is_empty());
    state.reset();
    let next = snapshot(2);
    assert_eq!(
        &state.sync(Some(&next))[..4],
        &[
            Request::Disable,
            Request::Commit,
            Request::Enable,
            Request::ContentType(ContentType::Multiline)
        ]
    );
    assert_eq!(state.commit_count, 3);
    assert_eq!(state.target_for(1), Some(1));
    assert_eq!(state.target_for(2), None);
    assert_eq!(state.target_for(3), Some(2));
}
