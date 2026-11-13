//! 【系统托盘测试】【Windows动态菜单】验证原生右键回调读取最新状态、交付录制操作并释放菜单

#[path = "windows_menu_driver.rs"]
mod driver;

use crate::{
    features::desktop::{
        Action, Event, Reporter,
        tray::{menu, recording::State as Recording},
    },
    ui::language::Language,
};
use driver::{Expected, Window};
use tokio::sync::{mpsc, watch};

/// 【系统托盘测试】【原生环境】无参数，返回测试窗口、菜单快照发送端和生产动作接收端
fn fixture() -> (
    Window,
    watch::Sender<menu::State>,
    mpsc::UnboundedReceiver<Event>,
) {
    let (sender, actions) = mpsc::unbounded_channel();
    let reporter = Reporter {
        sender,
        generation: 1,
        hotkey: false,
    };
    let (states, receiver) = watch::channel(menu::State {
        language: Language::Chinese,
        delayed: false,
        recording: Recording::Idle,
    });
    let window = Window::new(reporter, receiver).expect("create native tray test window");
    (window, states, actions)
}

/// 【系统托盘测试】【动作接收】按 actions 返回唯一生产动作；无动作时返回空值，其他事件或重复交付直接失败
fn action(actions: &mut mpsc::UnboundedReceiver<Event>) -> Option<Action> {
    let result = match actions.try_recv() {
        Ok(Event::Action(action, None)) => Some(action),
        Err(mpsc::error::TryRecvError::Empty) => None,
        _ => panic!("unexpected native tray event"),
    };
    assert!(matches!(
        actions.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    result
}

/// 【系统托盘测试】【实际录制切换】无参数，同一原生窗口连续打开最新菜单，按键选择对应开始、暂停、继续和停止动作
#[test]
fn native_popup_uses_latest_recording_snapshot_and_delivers_commands() {
    let (window, states, mut actions) = fixture();
    for (recording, title, items, selected, expected_action) in [
        (
            Recording::Idle,
            "屏幕录制",
            vec![("开始录制", true)],
            0,
            Some(Action::Record),
        ),
        (
            Recording::Recording,
            "录制中",
            vec![("暂停录制", true), ("停止并保存", true)],
            0,
            Some(Action::PauseRecording),
        ),
        (
            Recording::Paused,
            "录制已暂停",
            vec![("继续录制", true), ("停止并保存", true)],
            0,
            Some(Action::ResumeRecording),
        ),
        (
            Recording::Recording,
            "录制中",
            vec![("暂停录制", true), ("停止并保存", true)],
            1,
            Some(Action::StopRecording),
        ),
        (
            Recording::Countdown,
            "录制倒计时",
            vec![("取消录制", true)],
            0,
            Some(Action::StopRecording),
        ),
        (
            Recording::Starting,
            "录制准备中",
            vec![("取消录制", true)],
            0,
            Some(Action::StopRecording),
        ),
        (
            Recording::Saving,
            "录制保存中",
            vec![("正在保存", false)],
            0,
            None,
        ),
        (
            Recording::Unavailable,
            "屏幕录制",
            vec![("录制状态暂不可用", false)],
            0,
            None,
        ),
        (
            Recording::Preparing,
            "屏幕录制",
            vec![("录制窗口已打开", false)],
            0,
            None,
        ),
        (
            Recording::Idle,
            "屏幕录制",
            vec![("开始录制", true)],
            0,
            Some(Action::Record),
        ),
    ] {
        states.send_modify(|state| state.recording = recording);
        window
            .open(Expected {
                title,
                items,
                selected: Some(selected),
            })
            .unwrap_or_else(|error| panic!("{recording:?}: {error}"));
        assert_eq!(action(&mut actions), expected_action, "{recording:?}");
    }
}

/// 【系统托盘测试】【取消与语言切换】无参数，取消原生菜单不交付操作，下一次菜单立即采用新语言并回收句柄
#[test]
fn native_popup_cancel_and_language_change_preserve_menu_lifecycle() {
    let (window, states, mut actions) = fixture();
    window
        .open(Expected {
            title: "屏幕录制",
            items: vec![("开始录制", true)],
            selected: None,
        })
        .expect("cancel native tray menu");
    assert_eq!(action(&mut actions), None);
    states.send_modify(|state| {
        state.language = Language::English;
        state.recording = Recording::Paused;
    });
    window
        .open(Expected {
            title: "Recording Paused",
            items: vec![("Resume Recording", true), ("Stop and Save", true)],
            selected: Some(0),
        })
        .expect("open current English recording menu");
    assert_eq!(action(&mut actions), Some(Action::ResumeRecording));
}
