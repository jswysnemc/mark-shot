use crate::{
    backend::window::{
        KeyInput, LEFT_BUTTON, RIGHT_BUTTON, WindowAction, WindowContent, WindowEvent,
    },
    features::ocr_result::{
        document::OcrDocument,
        layout::Control,
        window::{ResultWindow, View},
    },
    keys::{Key, Mods},
};

/// 【结果按钮测试】【固定文档】按 text 构造未连接窗口，不启动模型或写入配置，返回窗口状态
fn window(text: &str) -> ResultWindow {
    ResultWindow::new(OcrDocument {
        text: text.into(),
        png: crate::utils::encode_png(&tiny_skia::Pixmap::new(2, 2).unwrap()),
        lines: vec![],
    })
    .unwrap()
}

/// 【结果按钮测试】【真实坐标】返回 state 中 control 的内部点，使用当前布局，不复制 Qt 几何作为原生假象
fn point(state: &ResultWindow, control: Control) -> (f64, f64) {
    let rect = match control {
        Control::Close => state.layout.close,
        Control::Copy => state.layout.copy,
        Control::Undo => state.layout.undo.unwrap(),
        _ => unreachable!(),
    };
    (f64::from(rect.x() + 10.0), f64::from(rect.y() + 10.0))
}

/// 【结果按钮测试】【鼠标事件】为 position、button、pressed 创建表面事件，返回事件，不访问桌面
fn mouse(position: (f64, f64), button: u32, pressed: bool) -> WindowEvent {
    WindowEvent::PointerButton {
        position,
        button,
        pressed,
    }
}

/// 【结果按钮测试】【按键事件】按 key、mods、pressed 返回按下或释放事件，不生成正文字符
fn key(key: Key, mods: Mods, pressed: bool) -> WindowEvent {
    let input = KeyInput {
        key: Some(key),
        text: String::new(),
        mods,
    };
    if pressed {
        WindowEvent::Key(input)
    } else {
        WindowEvent::KeyRelease(input)
    }
}

/// 【结果按钮测试】【释放执行】关闭按钮左键按下只更新状态，内部释放才关闭，未按下的释放无动作
#[test]
fn close_waits_for_a_matching_left_release() {
    let mut state = window("alpha");
    let at = point(&state, Control::Close);
    assert_eq!(
        state.event(mouse(at, LEFT_BUTTON, false)),
        WindowAction::None
    );
    assert_eq!(
        state.event(mouse(at, LEFT_BUTTON, true)),
        WindowAction::Redraw
    );
    assert_eq!(
        state.event(mouse(at, LEFT_BUTTON, false)),
        WindowAction::Close
    );
}

/// 【结果按钮测试】【移出取消】按下后移出并释放或失焦后释放，不能执行已取消的按钮
#[test]
fn outside_release_and_native_focus_loss_cancel_actions() {
    let mut state = window("alpha");
    let at = point(&state, Control::Close);
    state.event(mouse(at, LEFT_BUTTON, true));
    assert_eq!(
        state.event(mouse((2.0, 100.0), LEFT_BUTTON, false)),
        WindowAction::Redraw
    );
    state.event(mouse(at, LEFT_BUTTON, true));
    state.event(WindowEvent::Focused(false));
    assert_eq!(
        state.event(mouse(at, LEFT_BUTTON, false)),
        WindowAction::None
    );
}

/// 【结果按钮测试】【右键焦点】右键只聚焦关闭按钮，不执行；普通 Enter 不把非默认按钮当作默认确认
#[test]
fn right_click_focuses_without_activation_and_return_is_not_a_button_shortcut() {
    let mut state = window("alpha");
    let at = point(&state, Control::Close);
    state.event(mouse(at, RIGHT_BUTTON, true));
    assert!(state.text_input_target().is_none());
    assert_eq!(
        state.event(key(Key::Enter, Mods::default(), true)),
        WindowAction::None
    );
    assert_eq!(
        state.event(mouse(at, RIGHT_BUTTON, false)),
        WindowAction::None
    );
}

/// 【结果按钮测试】【空格时机】Space 包括修饰键组合都等待释放，不能在按下或自动重复时执行关闭
#[test]
fn modified_space_activates_only_on_release() {
    for mods in [
        Mods::default(),
        Mods {
            ctrl: true,
            ..Mods::default()
        },
        Mods {
            alt: true,
            ..Mods::default()
        },
        Mods {
            shift: true,
            ..Mods::default()
        },
        Mods {
            logo: true,
            ..Mods::default()
        },
    ] {
        let mut state = window("alpha");
        let at = point(&state, Control::Close);
        state.event(mouse(at, RIGHT_BUTTON, true));
        assert_eq!(
            state.event(key(Key::Space, mods, true)),
            WindowAction::Redraw
        );
        state.event(key(Key::Space, mods, true));
        assert_eq!(
            state.event(key(Key::Space, mods, false)),
            WindowAction::Close
        );
        assert_eq!(
            state.event(key(Key::Space, mods, false)),
            WindowAction::None
        );
    }
}

/// 【结果按钮测试】【撤销时机】撤销按钮保持正文直到释放，最后一项撤销后焦点转到复制按钮
#[test]
fn undo_waits_for_release_and_focuses_copy_when_the_button_disappears() {
    let mut state = window("alpha");
    state.source.insert(&mut state.fonts, "!");
    state.refresh_layout();
    let before = state.source.text();
    let at = point(&state, Control::Undo);
    state.event(mouse(at, LEFT_BUTTON, true));
    assert_eq!(state.source.text(), before);
    state.event(mouse(at, LEFT_BUTTON, false));
    assert_eq!(state.source.text(), "alpha");
    assert!(state.text_input_target().is_none());
}

/// 【结果按钮测试】【禁用焦点】空白原文复制按钮不执行，点击后父窗口焦点也不能继续向正文写字
#[test]
fn disabled_copy_never_grants_editor_input_or_copies_whitespace() {
    let mut state = window("  ");
    let at = point(&state, Control::Copy);
    state.event(mouse(at, RIGHT_BUTTON, true));
    assert!(state.text_input_target().is_none());
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Char('x')),
        text: "x".into(),
        mods: Mods::default(),
    }));
    assert_eq!(state.source.text(), "  ");
    assert_eq!(
        state.event(key(
            Key::Char('c'),
            Mods {
                ctrl: true,
                shift: true,
                ..Mods::default()
            },
            true
        )),
        WindowAction::None
    );
}

/// 【结果按钮测试】【页签职责】左键页签仍在按下时换页，右键仅聚焦页签，不按按钮释放规则处理页签
#[test]
fn tabs_keep_the_qt_press_behavior_and_right_focus() {
    let mut state = window("alpha");
    let image = state.layout.titlebar.tabs[1].1;
    let at = (f64::from(image.x() + 10.0), f64::from(image.y() + 10.0));
    state.event(mouse(at, RIGHT_BUTTON, true));
    assert_eq!(state.view, View::Text);
    assert!(state.text_input_target().is_none());
    assert_eq!(
        state.event(mouse(at, LEFT_BUTTON, true)),
        WindowAction::Redraw
    );
    assert_eq!(state.view, View::Image);
}

/// 【结果按钮测试】【关闭快捷键】只有无修饰键 Escape 关闭普通结果窗口，带修饰键不执行关闭
#[test]
fn modified_escape_is_not_the_window_close_shortcut() {
    for mods in [
        Mods {
            ctrl: true,
            ..Mods::default()
        },
        Mods {
            alt: true,
            ..Mods::default()
        },
        Mods {
            shift: true,
            ..Mods::default()
        },
        Mods {
            logo: true,
            ..Mods::default()
        },
    ] {
        assert_eq!(
            window("alpha").event(key(Key::Escape, mods, true)),
            WindowAction::None
        );
    }
    assert_eq!(
        window("alpha").event(key(Key::Escape, Mods::default(), true)),
        WindowAction::Close
    );
}

/// 【结果反馈测试】【事件保留】切页、编辑和撤销只修改正文，不撤销其他分区的独立复制反馈，无参数、无返回值
#[test]
fn view_edit_and_undo_events_keep_the_independent_feedback() {
    use crate::features::ocr_result::feedback::Pane;
    let mut state = window("alpha");
    let now = std::time::Instant::now();
    state.feedback.show(Pane::Source, true, now);
    state.feedback.show(Pane::Table, false, now);
    state.control(Control::View(View::Image));
    assert_eq!(state.feedback.label(Pane::Source), "Copied");
    state.control(Control::View(View::Text));
    state.source.insert(&mut state.fonts, "!");
    state.edit_event(crate::ui::text_editor::EditEvent::Changed);
    state.control(Control::Undo);
    assert_eq!(state.feedback.label(Pane::Source), "Copied");
    assert_eq!(state.feedback.label(Pane::Table), "Copy failed");
    assert_eq!(state.source.text(), "alpha");
}

/// 【结果反馈测试】【真实唤醒】到期 Wake 只重绘一次并清除最近期限，后续无期限 Wake 没有周期工作，无参数、无返回值
#[test]
fn expired_feedback_wake_is_consumed_once() {
    use crate::features::ocr_result::feedback::Pane;
    let mut state = window("alpha");
    let before = std::time::Instant::now() - std::time::Duration::from_secs(3);
    state.feedback.show(Pane::Source, true, before);
    state.feedback.show(Pane::Table, true, before);
    assert!(state.next_deadline().is_some());
    assert_eq!(state.event(WindowEvent::Wake), WindowAction::Redraw);
    assert_eq!(state.next_deadline(), None);
    assert_eq!(state.event(WindowEvent::Wake), WindowAction::None);
}

/// 【结果反馈测试】【几何刷新】复制反馈收窄后重新核对实际指针位置，原按钮之外不保留悬停，无参数、无返回值
#[test]
fn feedback_expiry_rechecks_hover_against_the_new_copy_geometry() {
    use crate::features::ocr_result::feedback::Pane;
    let mut state = window("alpha");
    let normal_left = state.layout.copy.left();
    state.feedback.show(
        Pane::Source,
        true,
        std::time::Instant::now() - std::time::Duration::from_secs(3),
    );
    state.refresh_layout();
    state.event(WindowEvent::PointerMove {
        position: (f64::from(normal_left - 4.0), 19.0),
    });
    assert!(state.hovered == Some(Control::Copy));
    state.event(WindowEvent::Wake);
    assert!(state.hovered.is_none());
    assert_eq!(state.layout.copy.left(), normal_left);
}

/// 【结果按钮测试】【原版回放】回放冻结 Qt 窗口的关闭、Enter 和 Escape 事件，比较焦点、按下状态与窗口关闭时机
#[test]
fn actual_qt_control_sequences_match_native_focus_and_delivery() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-controls.json"
    ))
    .unwrap();
    for name in [
        "close-left",
        "close-right",
        "close-space",
        "close-return",
        "copy-return",
        "copy-shift-return",
        "copy-alt-return",
        "copy-logo-return",
        "copy-escape",
        "copy-ctrl-escape",
        "copy-shift-escape",
        "copy-alt-escape",
        "copy-logo-escape",
    ] {
        let qt = fixture["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap();
        let control = if name.starts_with("close") {
            Control::Close
        } else {
            Control::Copy
        };
        let mut state = window("alpha beta\ngamma");
        let at = point(&state, control);
        let mods = Mods {
            ctrl: name.contains("ctrl"),
            shift: name.contains("shift"),
            alt: name.contains("alt"),
            logo: name.contains("logo"),
        };
        let mut closed = false;
        let mut actions = 0;
        for (index, expected) in qt["states"].as_array().unwrap().iter().enumerate() {
            if index > 0 {
                let pressed = index == 1;
                let event = if name.ends_with("left") || name.ends_with("right") {
                    mouse(
                        at,
                        if name.ends_with("left") {
                            LEFT_BUTTON
                        } else {
                            RIGHT_BUTTON
                        },
                        pressed,
                    )
                } else {
                    state.focus_control(super::FocusTarget::Button(control));
                    key(
                        if name.contains("space") {
                            Key::Space
                        } else if name.contains("return") {
                            Key::Enter
                        } else {
                            Key::Escape
                        },
                        mods,
                        pressed,
                    )
                };
                if state.event(event) == WindowAction::Close {
                    closed = true;
                }
                if closed && !name.contains("escape") && actions == 0 {
                    actions += 1;
                }
            }
            let focus = if closed {
                ""
            } else {
                match state.focus_target {
                    super::FocusTarget::Content => "ocrEditor",
                    super::FocusTarget::Window => "ocrResultWindow",
                    super::FocusTarget::Tabs => "ocrViewTabs",
                    super::FocusTarget::Button(Control::Close) => "ocrCloseButton",
                    super::FocusTarget::Button(Control::Copy) => "ocrCopyButton",
                    _ => unreachable!(),
                }
            };
            assert_eq!(
                focus,
                expected["focus"].as_str().unwrap(),
                "{name} step {index}"
            );
            assert_eq!(
                !closed,
                expected["visible"].as_bool().unwrap(),
                "{name} step {index}"
            );
            assert_eq!(
                state.control_down(control),
                expected["down"].as_bool().unwrap(),
                "{name} step {index}"
            );
            assert_eq!(
                actions,
                expected["clicks"].as_u64().unwrap(),
                "{name} step {index}"
            );
        }
    }
}

/// 【结果按钮测试】【空白资格】多行和 Unicode 空白不启用原文复制，正文中存在非空白字符时才启用
#[test]
fn copy_availability_borrows_lines_without_a_full_document_copy() {
    for (text, enabled) in [
        ("", false),
        (" \n\t", false),
        ("\u{a0}\u{3000}", false),
        (" \nalpha ", true),
        ("你", true),
    ] {
        let state = window(text);
        assert_eq!(state.control_enabled(Control::Copy), enabled);
        assert_eq!(state.source.has_non_whitespace(), enabled);
    }
}
