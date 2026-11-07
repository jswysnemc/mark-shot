//! 【单实例回归】【调度边界】待截图期限替换、活动窗口保护与明确拒绝未提供功能

use super::super::{request::Request, state::State};
use std::time::{Duration, Instant};

/// 【单实例回归】【延迟替换】新的请求替换原期限及全屏、多输出参数，旧期限不触发捕获，无参数、无返回值
#[test]
fn pending_capture_uses_latest_parameters_and_deadline() {
    let now = Instant::now();
    let initial = Request {
        capture: true,
        capture_delay_ms: 100,
        ..Request::default()
    };
    let mut state = State::new(initial.plan(now).unwrap(), now);
    let replacement = Request {
        capture: true,
        fullscreen: true,
        all_outputs: true,
        capture_delay_ms: 200,
        ..Request::default()
    };
    let (reply, changed) = state.request(replacement, now);
    assert!(
        changed && serde_json::from_slice::<serde_json::Value>(&reply).unwrap()["handled"] == true
    );
    assert!(state.begin(now + Duration::from_millis(100)).is_none());
    let plan = state.begin(now + Duration::from_millis(200)).unwrap();
    assert!(plan.fullscreen && plan.all_outputs);
    assert!(state.pending().is_none());
}

/// 【单实例回归】【即时替换】即时请求取消尚未开始的延迟，开始后重复请求不改变当前截图，无参数、无返回值
#[test]
fn immediate_request_replaces_waiting_delay_without_restarting_active_capture() {
    let now = Instant::now();
    let initial = Request {
        capture: true,
        capture_delay_ms: 100,
        ..Request::default()
    };
    let mut state = State::new(initial.plan(now).unwrap(), now);
    assert!(
        state
            .request(
                Request {
                    capture: true,
                    ..Request::default()
                },
                now
            )
            .1
    );
    assert!(state.begin(now).is_some());
    state.active();
    assert!(
        !state
            .request(
                Request {
                    capture: true,
                    fullscreen: true,
                    ..Request::default()
                },
                now
            )
            .1
    );
    assert!(state.begin(now).is_none());
}

/// 【单实例回归】【活动期限】重复延迟仅保留必要到期事件，窗口仍活动时清理请求，不创建后续截图，无参数、无返回值
#[test]
fn active_capture_discards_due_duplicate_and_immediate_request_cancels_it() {
    let now = Instant::now();
    let (_, plan) = super::capture();
    let mut state = State::new(plan, now);
    state.active();
    let delayed = Request {
        capture: true,
        capture_delay_ms: 50,
        ..Request::default()
    };
    state.request(delayed.clone(), now);
    assert_eq!(state.deadline(), Some(now + Duration::from_millis(50)));
    state.expire(now + Duration::from_millis(50));
    assert!(state.deadline().is_none());
    state.request(delayed, now);
    state.request(
        Request {
            capture: true,
            ..Request::default()
        },
        now,
    );
    assert!(state.deadline().is_none());
    state.close();
    state.active();
    assert!(state.begin(now).is_none());
}

/// 【单实例回归】【明确错误】超过一分钟或录制请求返回未处理，普通查询保留待截图计划，无参数、无返回值
#[test]
fn unsupported_requests_do_not_change_pending_capture() {
    let now = Instant::now();
    let initial = Request {
        capture: true,
        capture_delay_ms: 100,
        ..Request::default()
    };
    let mut state = State::new(initial.plan(now).unwrap(), now);
    let before = state.pending();
    for request in [
        Request {
            capture: true,
            capture_delay_ms: 60_001,
            ..Request::default()
        },
        Request {
            recording_status: true,
            ..Request::default()
        },
        Request {
            stop_recording: true,
            ..Request::default()
        },
        Request {
            pause_recording: true,
            ..Request::default()
        },
        Request {
            resume_recording: true,
            ..Request::default()
        },
        Request {
            toggle_pause_recording: true,
            ..Request::default()
        },
    ] {
        let (reply, changed) = state.request(request, now);
        assert!(!changed);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&reply).unwrap()["handled"],
            false
        );
        assert_eq!(state.pending(), before);
    }
    assert!(!state.request(Request::default(), now).1);
    assert_eq!(state.pending(), before);
}

/// 【单实例回归】【原版字段】换行 JSON 使用实际 Qt 字段名称，负延迟归零，无参数、无返回值
#[test]
fn request_uses_qt_field_names_and_normalizes_negative_delay() {
    let request: Request = serde_json::from_str(
        r#"{"capture":true,"fullscreen":true,"allOutputs":true,"captureDelayMs":-2}"#,
    )
    .unwrap();
    let plan = request.plan(Instant::now()).unwrap();
    assert!(plan.fullscreen && plan.all_outputs && plan.deadline.is_none());
    let bytes = request.encode().unwrap();
    assert!(bytes.ends_with(b"\n"));
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["allOutputs"], true);
    assert_eq!(value["captureDelayMs"], -2);
}
