//! 【OCR 置顶测试】【状态隔离】保存失败不切换角色，按钮只接受完成的左键事务，不改变文字和翻译任务

use super::*;
use crate::{
    backend::window::{LEFT_BUTTON, RIGHT_BUTTON, WindowContent, WindowEvent},
    features::ocr_result::{layout::Control, translation::controller},
};

/// 【OCR 置顶测试】【私有状态】返回临时目录与固定窗口，显式关闭置顶，不使用用户配置保存，无参数
fn window() -> (tempfile::TempDir, ResultWindow) {
    let (root, mut window) = controller::tests::window("sleep 0.6; exit 8");
    window.stacking.active = false;
    (root, window)
}

/// 【OCR 置顶测试】【原子交付】失败保持原开关和限时提示，成功仅修改独立开关，不改变正文版本，无参数、无返回值
#[test]
fn saving_changes_only_the_result_window_flag_and_failure_rolls_back() {
    let (_root, mut state) = window();
    let version = state.source.text_version();
    change(&mut state, |_| Err(io::Error::other("private failure")));
    assert!(!state.always_on_top());
    assert_eq!(state.status.as_deref(), Some("Failed to save settings"));
    let deadline = state.stacking.deadline.unwrap();
    assert!(!expire(&mut state, deadline - Duration::from_millis(1)));
    assert!(expire(&mut state, deadline));
    assert!(state.status.is_none());
    change(&mut state, |flag| {
        assert!(flag);
        Ok(())
    });
    assert!(state.always_on_top());
    assert_eq!(state.source.text_version(), version);
}

/// 【OCR 置顶测试】【任务保持】切换置顶不重新创建或取消翻译任务，不接收不支持协议的伪成功，无参数、无返回值
#[test]
fn changing_stacking_keeps_the_current_translation_task() {
    let (_root, mut state) = window();
    controller::start(&mut state);
    assert!(state.translation.state.busy());
    change(&mut state, |_| Ok(()));
    assert!(state.translation.state.busy());
    assert!(state.translation.task.is_some());
    state.stacking_supported(false);
    assert!(!state.always_on_top());
    assert!(!state.control_enabled(Control::AlwaysOnTop));
    assert_eq!(
        change(&mut state, |_| panic!(
            "unsupported stacking wrote settings"
        )),
        WindowAction::None
    );
}

/// 【OCR 置顶测试】【右键资格】真实右键及左键按下均不交付保存，布局命中与原版槽位一致，无参数、无返回值
#[test]
fn pin_button_does_not_activate_on_right_click_or_left_press() {
    let (_root, mut state) = window();
    let bounds = state.layout.titlebar.slots[2].unwrap();
    let position = (f64::from(bounds.x() + 13.0), f64::from(bounds.y() + 13.0));
    assert_eq!(
        state.layout.control(position, state.view),
        Some(Control::AlwaysOnTop)
    );
    for button in [RIGHT_BUTTON, LEFT_BUTTON] {
        state.event(WindowEvent::PointerButton {
            position,
            button,
            pressed: true,
        });
        assert!(!state.stacking.active);
    }
}
