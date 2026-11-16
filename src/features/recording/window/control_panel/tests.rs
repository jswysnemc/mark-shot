//! 【录制控制面板测试】【会话边界】验证隐藏不停止录制、真实状态确认与静止界面不重绘

use super::{layout::Control, state::Model, *};
use crate::{
    backend::window::{KeyInput, LEFT_BUTTON, WindowAction, WindowContent, WindowEvent},
    features::recording::{Command, Target, controller::Controller},
    keys::{Key, Mods},
};

/// 【录制控制面板测试】【状态样本】按 phase 和 elapsed_ms 返回测试会话状态，不创建录制设备
fn status(phase: &str, elapsed_ms: u64) -> Status {
    Status {
        phase: phase.into(),
        elapsed_ms,
        ..Default::default()
    }
}

/// 【录制控制面板测试】【隐藏操作】分别关闭、按 Esc 和点击隐藏；面板销毁后实际工作线程仍在等待控制
#[test]
fn hiding_panel_leaves_recording_worker_alive() {
    let directory = tempfile::tempdir().unwrap();
    let controller = Rc::new(
        Controller::start(
            crate::config::recording::Recording {
                countdown: 10,
                directory: directory.path().to_string_lossy().into(),
                ..Default::default()
            },
            Target::default(),
        )
        .unwrap(),
    );
    for action in 0..3 {
        let hidden = Rc::new(Cell::new(false));
        let shared = controller.clone();
        let mut panel = Panel::new(
            fonts::shared(),
            move || (!shared.finished(), shared.snapshot()),
            hidden.clone(),
        );
        match action {
            0 => assert!(panel.close_requested()),
            1 => assert_eq!(
                panel.event(WindowEvent::Key(KeyInput {
                    key: Some(Key::Escape),
                    text: String::new(),
                    mods: Mods::default()
                })),
                WindowAction::Close
            ),
            _ => {
                let rect = Control::Hide.bounds();
                let position = (f64::from(rect.x() + 5.0), f64::from(rect.y() + 5.0));
                panel.event(WindowEvent::PointerButton {
                    position,
                    button: LEFT_BUTTON,
                    pressed: true,
                });
                assert_eq!(
                    panel.event(WindowEvent::PointerButton {
                        position,
                        button: LEFT_BUTTON,
                        pressed: false
                    }),
                    WindowAction::Close
                );
            }
        }
        assert!(hidden.get());
        assert!(panel.next_deadline().is_none());
        drop(panel);
        assert!(
            !controller.finished(),
            "hiding controls stopped the recording worker"
        );
        assert_eq!(Rc::strong_count(&controller), 1);
    }
    controller.send(Command::Stop).unwrap();
}

/// 【录制控制面板测试】【会话结束】工作线程结束自动关闭窗口，无隐藏提示且不保留刷新期限
#[test]
fn completed_session_closes_panel_without_hidden_notice() {
    let hidden = Rc::new(Cell::new(false));
    let mut panel = Panel::new(
        fonts::shared(),
        || (false, status("finished", 2000)),
        hidden.clone(),
    );
    assert_eq!(panel.event(WindowEvent::Wake), WindowAction::Close);
    assert!(!hidden.get());
    assert!(panel.next_deadline().is_none());
}

/// 【录制控制面板测试】【命令确认】暂停与继续等待后端确认阶段后恢复按钮，服务即时回复不解锁
#[test]
fn commands_wait_for_real_phase_before_allowing_repeat() {
    let now = Instant::now();
    let mut model = Model::new(status("recording", 1800));
    assert_eq!(model.request(Control::Toggle, now), Some("pause"));
    assert!(!model.enabled(Control::Toggle));
    assert!(model.enabled(Control::Hide));
    assert!(!model.update(status("recording", 1900), now + POLL));
    assert!(model.update(status("paused", 1900), now + POLL * 2));
    assert_eq!(model.request(Control::Toggle, now), Some("resume"));
    assert!(!model.enabled(Control::Stop));
    assert!(model.update(status("recording", 1900), now + POLL));
    assert_eq!(model.request(Control::Stop, now), Some("stop"));
    assert!(model.update(status("saving", 2200), now + POLL));
    assert!(!model.enabled(Control::Stop));
    assert!(!model.enabled(Control::Toggle));
    assert!(model.enabled(Control::Hide));
}

/// 【录制控制面板测试】【失败恢复】请求失败和后端无响应都恢复可用操作，并保留可显示反馈
#[test]
fn failed_or_unconfirmed_commands_can_be_retried() {
    let now = Instant::now();
    let mut model = Model::new(status("recording", 1000));
    model.request(Control::Toggle, now);
    model.failed("connection closed".into());
    assert!(model.enabled(Control::Toggle));
    assert_eq!(model.error.as_deref(), Some("connection closed"));
    assert_eq!(model.request(Control::Toggle, now), Some("pause"));
    assert!(model.error.is_none());
    assert!(model.update(status("recording", 1000), now + Duration::from_secs(4)));
    assert!(model.error.is_some());
    assert!(model.enabled(Control::Toggle));
}

/// 【录制控制面板测试】【静止内容】毫秒变化、重复状态与暂停状态不重绘，秒数和阶段变化才更新
#[test]
fn unchanged_display_does_not_redraw() {
    let now = Instant::now();
    let mut model = Model::new(status("recording", 1500));
    assert!(!model.update(status("recording", 1999), now));
    assert!(model.update(status("recording", 2000), now));
    assert!(model.update(status("paused", 2100), now));
    assert!(!model.update(status("paused", 2100), now + POLL));
    assert_eq!(model.elapsed(), "00:02");
    model.update(status("recording", 3_661_000), now);
    assert_eq!(model.elapsed(), "1:01:01");
}

/// 【录制控制面板测试】【操作边界】三个按钮互不重叠且位于紧凑尺寸内，空白区域只请求拖动
#[test]
fn button_bounds_are_disjoint_and_background_moves_window() {
    for control in layout::CONTROLS {
        let rect = control.bounds();
        assert!(rect.right() <= layout::SIZE.0 as f32);
        assert!(rect.bottom() <= layout::SIZE.1 as f32);
        assert_eq!(
            layout::hit((f64::from(rect.x() + 1.0), f64::from(rect.y() + 1.0))),
            Some(control)
        );
        for other in layout::CONTROLS
            .into_iter()
            .filter(|candidate| *candidate != control)
        {
            assert!(rect.intersect(&other.bounds()).is_none());
        }
    }
    let mut panel = Panel::new(
        fonts::shared(),
        || (true, status("recording", 1000)),
        Rc::new(Cell::new(false)),
    );
    assert_eq!(
        panel.event(WindowEvent::PointerButton {
            position: (10.0, 10.0),
            button: LEFT_BUTTON,
            pressed: true
        }),
        WindowAction::Move
    );
    assert!(panel.model.error.is_none());
}
