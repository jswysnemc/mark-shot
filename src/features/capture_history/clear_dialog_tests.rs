//! 【截图历史测试】【确认回归】真实 Qt 几何、右键焦点及按下释放事务

use super::*;
use crate::backend::window::KeyInput;

/// 【截图历史测试】【按键事件】为 key 和 mods 创建原生事件，返回事件，不发送真实桌面输入
fn key(key: Key, mods: Mods) -> WindowEvent {
    WindowEvent::Key(KeyInput {
        key: Some(key),
        text: String::new(),
        mods,
    })
}

/// 【截图历史测试】【鼠标事件】按 button、pressed 和 position 创建按钮事件，不使用显示服务器，返回事件
fn mouse(button: u32, pressed: bool, position: (f64, f64)) -> WindowEvent {
    WindowEvent::PointerButton {
        button,
        pressed,
        position,
    }
}

/// 【截图历史测试】【按钮中心】返回 state 中 button 的逻辑中心，不复用额外窗口内确认层坐标
fn center(state: &ClearDialog, button: Button) -> (f64, f64) {
    let rect = state
        .layout
        .buttons
        .iter()
        .find(|(kind, _)| *kind == button)
        .unwrap()
        .1;
    (
        (rect.x() + rect.width() / 2.0) as f64,
        (rect.y() + rect.height() / 2.0) as f64,
    )
}

/// 【截图历史测试】【Qt 冻结样本】返回只读原版探针的布局及真实事件结果，缺少证据即失败，无参数
fn reference() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-history-dialog.json"
    ))
    .unwrap()
}

/// 【截图历史测试】【精确矩形】比较 actual 和 Qt expected 的全部坐标，不舍入或放宽误差，无返回值
fn same_rect(actual: tiny_skia::Rect, expected: &serde_json::Value) {
    let expected: Vec<f32> = expected
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_f64().unwrap() as f32)
        .collect();
    assert_eq!(
        [actual.x(), actual.y(), actual.width(), actual.height()].as_slice(),
        expected
    );
}

/// 【截图历史测试】【独立几何】窗口尺寸、正文、按钮和默认焦点严格匹配真实 Qt，不放宽矩形断言，无参数、无返回值
#[test]
fn qt_clear_dialog_geometry_and_default_focus_match() {
    let fixture = reference();
    let state = ClearDialog::new();
    let request = request(91);
    assert_eq!(request.spec.size, (315, 96));
    assert_eq!(request.spec.minimum, (223, 77));
    assert_eq!(fixture["layout"]["size"], serde_json::json!([315, 96]));
    let label = state.layout.label;
    same_rect(label, &fixture["layout"]["labels"][0]["rect"]);
    assert_eq!(fixture["layout"]["labels"][0]["text"], MESSAGE);
    assert_eq!(fixture["layout"]["modality"], 1);
    for (index, (button, rect)) in state.layout.buttons.iter().enumerate() {
        same_rect(*rect, &fixture["layout"]["buttons"][index]["rect"]);
        assert_eq!(fixture["layout"]["buttons"][index]["text"], button.label());
    }
    assert_eq!(state.focus, Button::Cancel);
    assert!(!state.dialog_accepted());
}

/// 【截图历史测试】【真实语义】Qt 的取消、Tab、右键焦点和按钮释放结果逐项比较，未关闭场景不得误确认，无参数、无返回值
#[test]
fn qt_clear_dialog_results_match_real_event_scenarios() {
    for case in reference()["scenarios"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mut state = ClearDialog::new();
        let clear = center(&state, Button::Clear);
        let cancel = center(&state, Button::Cancel);
        let action = match name {
            "layout" => {
                for _ in 0..4 {
                    state.event(key(Key::Tab, Mods::default()));
                }
                WindowAction::None
            }
            "default-enter" => state.event(key(Key::Enter, Mods::default())),
            "escape" | "wm-close" => state.event(key(Key::Escape, Mods::default())),
            "ctrl-enter" | "shift-enter" | "alt-enter" | "logo-enter" => state.event(key(
                Key::Enter,
                Mods {
                    ctrl: name == "ctrl-enter",
                    shift: name == "shift-enter",
                    alt: name == "alt-enter",
                    logo: name == "logo-enter",
                },
            )),
            "ctrl-escape" | "shift-escape" | "logo-escape" => state.event(key(
                Key::Escape,
                Mods {
                    ctrl: name == "ctrl-escape",
                    shift: name == "shift-escape",
                    logo: name == "logo-escape",
                    ..Mods::default()
                },
            )),
            "ctrl-tab-enter" => {
                state.event(key(
                    Key::Tab,
                    Mods {
                        ctrl: true,
                        ..Mods::default()
                    },
                ));
                state.event(key(Key::Enter, Mods::default()))
            }
            "tab-ctrl-enter" | "tab-alt-enter" | "tab-shift-enter" => {
                state.event(key(Key::Tab, Mods::default()));
                state.event(key(
                    Key::Enter,
                    Mods {
                        ctrl: name == "tab-ctrl-enter",
                        alt: name == "tab-alt-enter",
                        shift: name == "tab-shift-enter",
                        ..Mods::default()
                    },
                ))
            }
            "tab-enter" | "right-enter" | "shift-tab-enter" | "up-enter" | "down-enter" => {
                state.event(key(
                    match name {
                        "right-enter" => Key::Right,
                        "up-enter" => Key::Up,
                        "down-enter" => Key::Down,
                        _ => Key::Tab,
                    },
                    Mods {
                        shift: name == "shift-tab-enter",
                        ..Mods::default()
                    },
                ));
                state.event(key(Key::Enter, Mods::default()))
            }
            "right-clear" => state.event(mouse(RIGHT_BUTTON, true, clear)),
            "right-clear-enter" => {
                state.event(mouse(RIGHT_BUTTON, true, clear));
                state.event(key(Key::Enter, Mods::default()))
            }
            "left-clear-press" => state.event(mouse(LEFT_BUTTON, true, clear)),
            "left-clear-release-outside" => {
                state.event(mouse(LEFT_BUTTON, true, clear));
                state.event(mouse(LEFT_BUTTON, false, (3.0, 3.0)))
            }
            "left-clear" | "left-cancel" => {
                let point = if name == "left-clear" { clear } else { cancel };
                state.event(mouse(LEFT_BUTTON, true, point));
                state.event(mouse(LEFT_BUTTON, false, point))
            }
            "space-clear" | "ctrl-space-clear" | "shift-space-clear" | "alt-space-clear" => {
                state.event(key(Key::Tab, Mods::default()));
                let mods = Mods {
                    ctrl: name == "ctrl-space-clear",
                    shift: name == "shift-space-clear",
                    alt: name == "alt-space-clear",
                    ..Mods::default()
                };
                assert_eq!(state.event(key(Key::Space, mods)), WindowAction::Redraw);
                assert!(!state.dialog_accepted());
                assert_eq!(state.pressed, Some(Button::Clear));
                state.event(WindowEvent::KeyRelease(KeyInput {
                    key: Some(Key::Space),
                    text: String::new(),
                    mods,
                }))
            }
            _ => panic!("unhandled real Qt scenario: {name}"),
        };
        assert_eq!(
            action == WindowAction::Close,
            !case["visible"].as_bool().unwrap(),
            "{name}"
        );
        assert_eq!(
            state.dialog_accepted(),
            case["accepted"].as_u64().unwrap() == 1,
            "{name}"
        );
    }
}

/// 【截图历史测试】【失焦取消】失焦或焦点切换清理旧 Space 事务，后续释放不能执行清空，无参数、无返回值
#[test]
fn clear_dialog_cancels_space_transaction_on_focus_loss_or_navigation() {
    let release = || {
        WindowEvent::KeyRelease(KeyInput {
            key: Some(Key::Space),
            text: String::new(),
            mods: Mods::default(),
        })
    };
    for event in [WindowEvent::Focused(false), key(Key::Tab, Mods::default())] {
        let mut state = ClearDialog::new();
        state.event(key(Key::Tab, Mods::default()));
        state.event(key(Key::Space, Mods::default()));
        state.event(event);
        assert_eq!(state.event(release()), WindowAction::None);
        assert!(!state.dialog_accepted());
    }
}
