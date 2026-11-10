//! 【框选翻译测试】【原版控件】生产 Qt 控件和避让模块直接采样，逐项比较几何与输入

use mark_shot::{
    features::selection_translation::placement,
    keys::Key,
    types::MouseButton,
    ui::selection_translation::{
        language_popup::{Event, Popup},
        result_bar::{Action, Bar},
    },
};
use serde_json::Value;
use tiny_skia::Rect;

/// 【框选翻译测试】【矩形读取】按 value 返回原版整数矩形，坐标不换算
fn rect(value: &Value) -> Rect {
    let r = value.as_array().unwrap();
    Rect::from_xywh(
        r[0].as_f64().unwrap() as f32,
        r[1].as_f64().unwrap() as f32,
        r[2].as_f64().unwrap() as f32,
        r[3].as_f64().unwrap() as f32,
    )
    .unwrap()
}

/// 【框选翻译测试】【尺寸读取】按 value 返回逻辑像素宽高
fn size(value: &Value) -> (f32, f32) {
    (
        value[0].as_f64().unwrap() as f32,
        value[1].as_f64().unwrap() as f32,
    )
}

/// 【框选翻译测试】【真实位置】逐项核对原版八个语言弹层和二十个浮条避让结果，无误差容许
#[test]
fn popup_and_result_bar_placement_match_production_qt() {
    let reference: Value = serde_json::from_str(include_str!(
        "fixtures/qt-selection-translation/reference.json"
    ))
    .unwrap();
    for case in reference["languages"].as_array().unwrap() {
        assert_eq!(
            placement::language(
                rect(&case["anchor"]),
                case["vertical"].as_bool().unwrap(),
                size(&case["size"]),
                rect(&case["bounds"])
            ),
            rect(&case["result"]),
            "{case}"
        );
    }
    for case in reference["positions"].as_array().unwrap() {
        let obstacles = case["obstacles"]
            .as_array()
            .unwrap()
            .iter()
            .map(rect)
            .collect::<Vec<_>>();
        assert_eq!(
            placement::result(
                rect(&case["region"]),
                size(&case["size"]),
                rect(&case["bounds"]),
                &obstacles
            ),
            rect(&case["result"]),
            "{case}"
        );
    }
}

/// 【框选翻译测试】【字体上下文】返回本机原版 Noto Sans 家族，仅用于有条件数值对照
fn fonts() -> cosmic_text::FontSystem {
    let mut fonts = mark_shot::fonts::bundled();
    let family = mark_shot::fonts::text_faces::family(&mut fonts, "Noto Sans", false, false);
    fonts.db_mut().set_sans_serif_family(family);
    fonts
}

/// 【框选翻译测试】【生产尺寸】保留原版按钮尺寸，新浮窗按钮插入复制与退出之间，整体宽度增加一个按钮间距
#[test]
fn language_list_and_every_bar_button_match_qt_geometry() {
    let reference: Value = serde_json::from_str(include_str!(
        "fixtures/qt-selection-translation/reference.json"
    ))
    .unwrap();
    let mut fonts = fonts();
    let popup = Popup::new(1, 0, "English", &mut fonts);
    assert_eq!(
        (popup.rect.width(), popup.rect.height()),
        size(&reference["popup_size"])
    );
    let popup = Popup::new(2, 0, "Esperanto", &mut fonts);
    assert_eq!(
        (popup.rect.width(), popup.rect.height()),
        size(&reference["custom_popup_size"])
    );
    for case in reference["bars"].as_array().unwrap() {
        let bar = Bar::new(case["label"].as_str().unwrap().into(), true, &mut fonts);
        let (width, height) = size(&case["size"]);
        assert_eq!(
            (bar.rect.width(), bar.rect.height()),
            (width + 33.0, height),
            "{case}; actual cells: {:?}",
            bar.cells
        );
        for ((action, actual), expected) in bar
            .cells
            .iter()
            .filter(|(action, _)| *action != Action::OpenWindow)
            .zip(case["buttons"].as_array().unwrap())
        {
            let expected_rect = rect(&expected["rect"]);
            let expected_rect = if *action == Action::Exit {
                Rect::from_xywh(
                    expected_rect.left() + 33.0,
                    expected_rect.top(),
                    expected_rect.width(),
                    expected_rect.height(),
                )
                .unwrap()
            } else {
                expected_rect
            };
            assert_eq!(*actual, expected_rect, "{expected}");
        }
        assert_eq!(bar.cells[4].0, Action::OpenWindow);
        assert_eq!(bar.cells[4].1.width(), 31.0);
    }
}

/// 【框选翻译测试】【左右键语义】实际 Qt 右键保持弹层，当前目标回车沿用；Rust 同时隔离不匹配左键序列
#[test]
fn right_button_and_keyboard_preserve_current_language_semantics() {
    let mut fonts = fonts();
    let mut popup = Popup::new(1, 0, "English", &mut fonts);
    for pressed in [true, false] {
        assert!(matches!(
            popup.button(
                (20.0, 24.0 + 3.0 * 26.0 + 13.0),
                MouseButton::Right,
                pressed
            ),
            Event::None
        ));
    }
    assert!(matches!(popup.key(Key::Enter, false), Event::Choose(value) if value == "English"));
    assert!(matches!(
        popup.button((20.0, 24.0 + 13.0), MouseButton::Left, false),
        Event::None
    ));
    popup.key(Key::Up, false);
    assert!(
        matches!(popup.key(Key::Enter, false), Event::Choose(value) if value == "Traditional Chinese")
    );
    popup.key(Key::End, false);
    popup.key(Key::Down, false);
    assert!(
        matches!(popup.key(Key::Enter, false), Event::Choose(value) if value == "Simplified Chinese")
    );
}
