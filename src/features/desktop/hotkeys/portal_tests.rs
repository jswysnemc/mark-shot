//! 【全局快捷键】【门户回归】使用真实 D-Bus 快捷键响应验证状态交付和绑定恢复

use super::*;
use crate::features::desktop::{Event, status::Registration};
use std::collections::HashMap;
use zbus::zvariant::{LE, Value, serialized::Context};

/// 【全局快捷键】【响应构造】按 id、binding 编解码门户格式的快捷键列表，返回真实 Shortcut 对象
fn shortcuts(id: &str, binding: &str) -> Vec<Shortcut> {
    let values = vec![(
        id,
        HashMap::from([
            ("description", Value::from("Screenshot")),
            ("trigger_description", Value::from(binding)),
        ]),
    )];
    let bytes = zbus::zvariant::to_bytes(Context::new_dbus(LE, 0), &values).unwrap();
    bytes.deserialize().unwrap().0
}

/// 【全局快捷键】【报告通道】无参数，返回测试报告器和对应事件接收端
fn reporter() -> (Reporter, tokio::sync::mpsc::UnboundedReceiver<Event>) {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    (
        Reporter {
            sender,
            generation: 7,
            hotkey: true,
        },
        receiver,
    )
}

#[test]
/// 【全局快捷键】【空绑定回归】无参数，验证门户空按键不能交付注册成功状态，无返回值
fn empty_portal_binding_is_not_reported_as_registered() {
    let (reporter, mut receiver) = reporter();
    assert!(!report_binding(&shortcuts("screenshot", ""), &reporter));
    assert!(matches!(
        receiver.try_recv().unwrap(),
        Event::Status(7, true, Registration::Failed(_))
    ));
}

#[test]
/// 【全局快捷键】【空白绑定回归】无参数，验证全空白描述同样属于未绑定，无返回值
fn whitespace_portal_binding_is_not_reported_as_registered() {
    let (reporter, mut receiver) = reporter();
    assert!(!report_binding(&shortcuts("screenshot", " \t "), &reporter));
    assert!(matches!(
        receiver.try_recv().unwrap(),
        Event::Status(7, true, Registration::Failed(_))
    ));
}

#[test]
/// 【全局快捷键】【状态恢复回归】无参数，验证后续系统配置可恢复绑定，移除后再次报告失败，无返回值
fn changed_binding_recovers_without_restarting_the_portal_session() {
    let (reporter, mut receiver) = reporter();
    report_binding(&shortcuts("screenshot", ""), &reporter);
    receiver.try_recv().unwrap();
    assert!(report_binding(
        &shortcuts("screenshot", "Ctrl+Shift+F10"),
        &reporter
    ));
    assert!(
        matches!(receiver.try_recv().unwrap(), Event::Status(7, true, Registration::Registered(binding)) if binding == "Ctrl+Shift+F10")
    );
    assert!(!report_binding(&shortcuts("other", "F10"), &reporter));
    assert!(matches!(
        receiver.try_recv().unwrap(),
        Event::Status(7, true, Registration::Failed(_))
    ));
}

/// 【全局快捷键测试】【独立窗口动作】无参数，窗口绑定不得复用普通截图动作，空绑定报告失败，恢复后通过，无返回值
#[test]
fn active_window_binding_is_required_and_routes_to_window_capture() {
    let (reporter, mut receiver) = reporter();
    assert!(!report_window_binding(
        &shortcuts("active-window", ""),
        &reporter
    ));
    assert!(matches!(
        receiver.try_recv().unwrap(),
        Event::Status(7, true, Registration::Failed(_))
    ));
    assert!(report_window_binding(
        &shortcuts("active-window", "Alt+Print"),
        &reporter
    ));
    assert_eq!(action("active-window"), Some(Action::CaptureWindow));
    assert_eq!(action("screenshot"), Some(Action::Capture));
    assert_eq!(action("other"), None);
    assert_eq!(trigger("Alt+PrintScreen").unwrap(), "ALT+Print");
}

/// 【全局快捷键测试】【绑定完整判断】无参数，验证普通与活动窗口快捷键的绑定完整性判定逻辑
#[test]
fn has_all_bindings_requires_non_empty_triggers() {
    let main_only = shortcuts("screenshot", "Ctrl+Alt+A");
    assert!(has_all_bindings(&main_only, false));
    assert!(!has_all_bindings(&main_only, true));
    assert!(!has_all_bindings(&shortcuts("screenshot", "  "), false));
}
