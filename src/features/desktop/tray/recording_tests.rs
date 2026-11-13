//! 【系统托盘测试】【录制菜单】验证共用菜单仅展示当前阶段可执行的录制操作

use super::menu::{self, Item, State};
use super::recording::State as Recording;
use crate::{
    features::{desktop::Action, recording::Status},
    ui::language::Language,
};

/// 【系统托盘测试】【录制操作】按 state 返回录制子菜单中的操作、文字和启用状态
fn recording_actions(state: State) -> Vec<(Action, String, bool)> {
    let entries = menu::entries(state);
    let Item::Submenu { items, .. } = &entries[4] else {
        panic!("missing recording submenu")
    };
    items
        .iter()
        .map(|item| match item {
            Item::Action {
                action,
                label,
                enabled,
            } => (*action, label.clone(), *enabled),
            _ => panic!("unexpected recording submenu item"),
        })
        .collect()
}

/// 【系统托盘测试】【空闲录制】无参数，空闲时只显示开始录制，不提供暂停、继续或停止操作
#[test]
fn idle_recording_menu_only_offers_start() {
    for (language, label) in [
        (Language::Chinese, "开始录制"),
        (Language::English, "Start Recording"),
    ] {
        assert_eq!(
            recording_actions(State {
                language,
                delayed: false,
                recording: Recording::Idle,
            }),
            [(Action::Record, label.to_owned(), true)]
        );
    }
}

/// 【系统托盘测试】【阶段切换】无参数，连续切换录制阶段时替换操作与标题，中英文菜单保持一致
#[test]
fn recording_lifecycle_replaces_actions_and_labels() {
    for language in [Language::Chinese, Language::English] {
        let chinese = language == Language::Chinese;
        for (recording, title, expected) in [
            (
                Recording::Countdown,
                ["录制倒计时", "Recording Countdown"],
                vec![(
                    Action::StopRecording,
                    ["取消录制", "Cancel Recording"],
                    true,
                )],
            ),
            (
                Recording::Starting,
                ["录制准备中", "Starting Recording"],
                vec![(
                    Action::StopRecording,
                    ["取消录制", "Cancel Recording"],
                    true,
                )],
            ),
            (
                Recording::Recording,
                ["录制中", "Recording in Progress"],
                vec![
                    (
                        Action::PauseRecording,
                        ["暂停录制", "Pause Recording"],
                        true,
                    ),
                    (Action::StopRecording, ["停止并保存", "Stop and Save"], true),
                ],
            ),
            (
                Recording::Paused,
                ["录制已暂停", "Recording Paused"],
                vec![
                    (
                        Action::ResumeRecording,
                        ["继续录制", "Resume Recording"],
                        true,
                    ),
                    (Action::StopRecording, ["停止并保存", "Stop and Save"], true),
                ],
            ),
            (
                Recording::Saving,
                ["录制保存中", "Saving Recording"],
                vec![(Action::StopRecording, ["正在保存", "Saving"], false)],
            ),
            (
                Recording::Idle,
                ["屏幕录制", "Recording"],
                vec![(Action::Record, ["开始录制", "Start Recording"], true)],
            ),
        ] {
            let state = State {
                language,
                delayed: true,
                recording,
            };
            let entries = menu::entries(state);
            let Item::Submenu { label, .. } = &entries[4] else {
                panic!("missing recording submenu")
            };
            assert_eq!(label, title[usize::from(!chinese)]);
            assert_eq!(
                recording_actions(state),
                expected
                    .into_iter()
                    .map(|(action, labels, enabled)| {
                        (action, labels[usize::from(!chinese)].to_owned(), enabled)
                    })
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// 【系统托盘测试】【不可用操作】无参数，已有准备窗口或查询失败时禁用开始录制，避免启动重复会话
#[test]
fn occupied_or_unavailable_recording_cannot_start() {
    for language in [Language::Chinese, Language::English] {
        for recording in [Recording::Preparing, Recording::Unavailable] {
            let actions = recording_actions(State {
                language,
                delayed: false,
                recording,
            });
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0].0, Action::Record);
            assert!(!actions[0].2);
        }
    }
}

/// 【系统托盘测试】【状态契约】无参数，实际录制状态正确转换，窗口占用与结束状态不会错误开放重复录制
#[test]
fn recording_status_maps_phases_and_window_ownership() {
    for (phase, expected) in [
        ("countdown", Recording::Countdown),
        ("starting", Recording::Starting),
        ("recording", Recording::Recording),
        ("paused", Recording::Paused),
        ("saving", Recording::Saving),
        ("unknown", Recording::Unavailable),
    ] {
        let status = Status {
            phase: phase.into(),
            window_open: true,
            ..Default::default()
        };
        assert_eq!(Recording::from_status(&status), expected);
    }
    for phase in ["idle", "finished", "cancelled", "failed"] {
        let mut status = Status {
            phase: phase.into(),
            ..Default::default()
        };
        assert_eq!(Recording::from_status(&status), Recording::Idle);
        status.window_open = true;
        assert_eq!(Recording::from_status(&status), Recording::Preparing);
    }
}
