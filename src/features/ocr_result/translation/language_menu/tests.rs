//! 【OCR 翻译测试】【原生列表】对照原版列表字体、视区、滚动条与九项导航，独立帧静止时不重复绘制

use super::*;
use serde_json::{Value, json};

/// 【OCR 翻译测试】【私有列表】按 width 创建私有窗口并展开语言列表，返回临时目录和窗口，不发起翻译任务
fn opened(width: f32) -> (tempfile::TempDir, ResultWindow) {
    let (root, mut window) = super::super::controller::tests::window("exit 8");
    window.layout.size = (width, 502.0);
    window.translation.state.visible = true;
    window.refresh_layout();
    open(&mut window);
    (root, window)
}

/// 【OCR 翻译测试】【矩形记录】按 rect 返回原版整数矩形形式，不改变被测几何
fn rect(rect: Rect) -> Value {
    json!([
        rect.x() as i32,
        rect.y() as i32,
        rect.width() as i32,
        rect.height() as i32
    ])
}

/// 【OCR 翻译测试】【六宽度列表】检查实际 Qt 补充探针全部宽度，字体、文字视区和滚动区域与原版一致，无参数、无返回值
#[test]
fn language_popup_matches_actual_qt_font_viewport_and_scrollbar() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/qt-ocr-translation-focus-popup.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let (_root, window) = opened(case["width"].as_f64().unwrap() as f32);
        let menu = window.translation.language_menu.as_ref().unwrap();
        let popup = &case["language_popup"];
        assert_eq!(
            json!([menu.size().0 as i32, menu.size().1 as i32]),
            popup["size"]
        );
        assert_eq!(rect(menu.viewport()), popup["viewport"]);
        assert_eq!(rect(menu.scrollbar()), popup["scrollbar"]);
        assert_eq!(menu.labels.len(), popup["rows"].as_array().unwrap().len());
        for states in &menu.labels {
            for label in states {
                assert_eq!(
                    label.metrics().font_size,
                    popup["font_pixels"].as_f64().unwrap() as f32
                );
                assert_eq!(
                    label.metrics().line_height,
                    popup["font_height"].as_f64().unwrap() as f32
                );
                assert!(
                    label.lines[0].attrs_list().defaults().family
                        == cosmic_text::Family::Name(popup["font_family"].as_str().unwrap())
                );
            }
        }
        for (index, row) in popup["rows"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                row,
                &json!([0, index * 30, menu.viewport().width() as usize, 30])
            );
        }
        assert_eq!(popup["max_visible"], 8);
        assert_eq!(popup["scroll_max"], 1);
        assert_eq!(popup["scroll_page"], 8);
    }
}

/// 【OCR 翻译测试】【滚轮与右键】滚轮能显示第九项，右键不选择语言，左键必须完整释放才交付，无参数、无返回值
#[test]
fn ninth_language_requires_scroll_and_left_release_not_a_right_click() {
    let (_root, mut window) = opened(760.0);
    let id = window.translation.language_menu.as_ref().unwrap().id;
    assert_eq!(
        event(&mut window, id, PopupEvent::Scroll { delta: (0.0, 15.0) }),
        WindowPopupAction::Redraw
    );
    assert_eq!(
        window
            .translation
            .language_menu
            .as_ref()
            .unwrap()
            .item(20.0, 230.0),
        Some(8)
    );
    for pressed in [true, false] {
        assert_eq!(
            event(
                &mut window,
                id,
                PopupEvent::Button {
                    x: 20.0,
                    y: 230.0,
                    button: MouseButton::Right,
                    pressed
                }
            ),
            WindowPopupAction::None
        );
    }
    assert_eq!(window.translation.target(), "English");
    assert_eq!(
        event(
            &mut window,
            id,
            PopupEvent::Button {
                x: 20.0,
                y: 230.0,
                button: MouseButton::Left,
                pressed: true
            }
        ),
        WindowPopupAction::None
    );
    assert_eq!(
        event(
            &mut window,
            id,
            PopupEvent::Button {
                x: 20.0,
                y: 230.0,
                button: MouseButton::Left,
                pressed: false
            }
        ),
        WindowPopupAction::Activate(8)
    );
}

/// 【OCR 翻译测试】【帧缓存】语言列表首次提交后静止帧为空，边框与滚动条不命中文字，无参数、无返回值
#[test]
fn unchanged_language_popup_has_no_new_frame_and_nontext_regions_do_not_select() {
    let (_root, mut window) = opened(760.0);
    assert!(frame(&mut window, 1.2).is_some());
    assert!(frame(&mut window, 1.2).is_none());
    let menu = window.translation.language_menu.as_ref().unwrap();
    for point in [
        (4.0, 20.0),
        (20.0, 4.0),
        (20.0, 245.0),
        (menu.size().0 as f64 - 8.0, 20.0),
    ] {
        assert!(menu.item(point.0, point.1).is_none());
    }
}
