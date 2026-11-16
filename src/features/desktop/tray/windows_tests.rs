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

/// 【系统托盘测试】【实际延时选项】无参数，原生菜单交付全部预设秒数，取消入口随最新计划状态和语言更新；无返回值
#[test]
fn native_popup_delivers_all_delays_and_updates_cancel_state() {
    let (window, states, mut actions) = fixture();
    let chinese = vec![
        ("1 秒后", true),
        ("3 秒后", true),
        ("5 秒后", true),
        ("10 秒后", true),
        ("30 秒后", true),
        ("", false),
        ("取消延时截图", false),
    ];
    // 1. 【系统托盘测试】【预设交付】通过真实右键回调逐个选择延迟，不直接调用菜单动作
    for (position, seconds) in [1, 3, 5, 10, 30].into_iter().enumerate() {
        window
            .open(Expected {
                submenu_position: 3,
                title: "延时截图",
                items: chinese.clone(),
                selected: Some(position),
            })
            .unwrap_or_else(|error| panic!("delay={seconds}: {error}"));
        assert_eq!(action(&mut actions), Some(Action::Delay(seconds)));
    }
    window
        .open(Expected {
            submenu_position: 3,
            title: "延时截图",
            items: chinese,
            selected: None,
        })
        .expect("cancel action stays disabled without a schedule");
    assert_eq!(action(&mut actions), None);
    // 2. 【系统托盘测试】【取消状态】重新打开菜单读取最新语言与待执行状态，再选择取消动作
    states.send_modify(|state| {
        state.language = Language::English;
        state.delayed = true;
    });
    let mut english = vec![
        ("After 1 second", true),
        ("After 3 seconds", true),
        ("After 5 seconds", true),
        ("After 10 seconds", true),
        ("After 30 seconds", true),
        ("", false),
        ("Cancel Delayed Capture", true),
    ];
    window
        .open(Expected {
            submenu_position: 3,
            title: "Delayed Capture",
            items: english.clone(),
            selected: Some(6),
        })
        .expect("choose enabled cancel action after the separator");
    assert_eq!(action(&mut actions), Some(Action::CancelDelay));
    states.send_modify(|state| state.delayed = false);
    english[6].1 = false;
    window
        .open(Expected {
            submenu_position: 3,
            title: "Delayed Capture",
            items: english,
            selected: None,
        })
        .expect("cancel action becomes disabled after cancellation");
    assert_eq!(action(&mut actions), None);
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
                submenu_position: 4,
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
            submenu_position: 4,
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
            submenu_position: 4,
            title: "Recording Paused",
            items: vec![("Resume Recording", true), ("Stop and Save", true)],
            selected: Some(0),
        })
        .expect("open current English recording menu");
    assert_eq!(action(&mut actions), Some(Action::ResumeRecording));
}
