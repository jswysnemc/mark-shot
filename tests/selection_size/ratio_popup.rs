//! 【选区尺寸测试】【比例弹层】确认真实控件的释放选择和键盘独占规则

use mark_shot::{
    keys::Key,
    types::MouseButton,
    ui::selection_size::ratio_popup::{Event, Popup},
};

/// 【选区尺寸测试】【弹层构造】按 ratio 返回固定六行比例弹层，不申请原生窗口角色
fn popup(ratio: f64) -> Popup {
    Popup::new(
        1,
        0,
        tiny_skia::Rect::from_xywh(20.0, 30.0, 128.0, 164.0).unwrap(),
        ratio,
    )
}

/// 【选区尺寸测试】【释放选择】右键及左键按下不能选择，左键释放按指针行而非旧活动行返回比例
#[test]
fn only_left_release_chooses_the_pointed_row() {
    let mut popup = popup(4.13448275862069);
    assert_eq!(popup.current, None);
    assert_eq!(popup.active, 0);
    let point = (20.0, 4.0 + 2.0 * 26.0 + 13.0);
    assert!(matches!(
        popup.button(point, MouseButton::Right, false),
        Event::None
    ));
    assert!(matches!(
        popup.button(point, MouseButton::Left, true),
        Event::None
    ));
    assert!(
        matches!(popup.button(point, MouseButton::Left, false), Event::Choose(value) if value == 4.0 / 3.0)
    );
    assert!(popup.pointer(point));
    assert_eq!(popup.active, 2);
    assert!(!popup.pointer((0.0, 0.0)));
    assert_eq!(popup.active, 2);
}

/// 【选区尺寸测试】【键盘导航】上下和正向 Tab 循环，反向 Tab 保持当前行，确认与退出返回明确动作
#[test]
fn popup_navigation_wraps_and_backtab_is_ignored() {
    let mut popup = popup(0.0);
    assert!(matches!(popup.key(Key::Up, false), Event::None));
    assert_eq!(popup.active, 5);
    assert!(matches!(popup.key(Key::Enter, false), Event::Choose(value) if value == 21.0 / 9.0));
    popup.key(Key::Down, false);
    assert_eq!(popup.active, 0);
    popup.key(Key::Tab, true);
    assert_eq!(popup.active, 0);
    popup.key(Key::Tab, false);
    assert_eq!(popup.active, 1);
    assert!(matches!(popup.key(Key::Space, false), Event::Choose(1.0)));
    assert!(matches!(popup.key(Key::Escape, false), Event::Close));
}
