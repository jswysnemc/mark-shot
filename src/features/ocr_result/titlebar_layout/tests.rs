use super::*;
use crate::features::ocr_result::layout::{Control, Layout};
use serde_json::Value;

/// 【标题布局测试】【矩形对照】将 actual 逻辑矩形与 expected 原版记录逐项比较，无返回值
fn compare(actual: Rect, expected: &Value) {
    let actual = [actual.x(), actual.y(), actual.width(), actual.height()];
    let expected: Vec<_> = expected
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    assert_eq!(actual.as_slice(), expected);
}

/// 【标题布局测试】【完整窗口记录】按 name 对照对应语言全部 117 个真实 Qt 状态，不比较隐藏控件的过期几何，无返回值
fn compare_language(name: &str) {
    let ui_language = if name == "zh" {
        crate::ui::language::Language::Chinese
    } else {
        crate::ui::language::Language::English
    };
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-titlebar.json"
    ))
    .unwrap();
    let language = fixture["languages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == name)
        .unwrap();
    let mut count = 0;
    for case in language["cases"].as_array().unwrap() {
        let has_table = case["table"].as_bool().unwrap();
        let has_image = case["image"].as_bool().unwrap();
        let views: Vec<_> = [View::Text, View::Table, View::Image]
            .into_iter()
            .filter(|v| {
                *v == View::Text || *v == View::Table && has_table || *v == View::Image && has_image
            })
            .collect();
        for state in case["states"]
            .as_array()
            .unwrap()
            .iter()
            .chain(std::iter::once(&case["with_undo"]))
        {
            let widgets = &state["widgets"];
            let view = views[state["tab"].as_u64().unwrap() as usize];
            let layout = Layout::localized(
                (
                    state["size"][0].as_u64().unwrap() as u32,
                    state["size"][1].as_u64().unwrap() as u32,
                ),
                has_table,
                has_image,
                view,
                widgets["source_copy"]["text"].as_str().unwrap(),
                "Copy",
                widgets["source_undo"]["visible"].as_bool().unwrap(),
                false,
                ui_language,
            );
            compare(layout.titlebar.bounds, &widgets["ocrTitleBar"]["rect"]);
            compare(layout.titlebar.viewport, &widgets["ocrViewTabs"]["rect"]);
            compare(layout.copy, &widgets["source_copy"]["rect"]);
            compare(layout.close, &widgets["ocrCloseButton"]["rect"]);
            compare(layout.grip, &widgets["ocrResizeGrip"]["rect"]);
            compare(layout.body, &widgets["ocrContentViews"]["rect"]);
            if let Some(icon) = layout.titlebar.icon {
                compare(icon, &widgets["title_icon"]["rect"]);
            }
            assert_eq!(
                layout.titlebar.icon.is_some(),
                widgets["title_icon"]["visible"].as_bool().unwrap()
            );
            for (index, key) in [
                "ocrTranslationToggle",
                "ocrMoreButton",
                "ocrPinButton",
                "ocrCloseButton",
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(rect) = layout.titlebar.slots[index] {
                    compare(rect, &widgets[key]["rect"]);
                }
            }
            for ((view, rect), tab) in layout
                .titlebar
                .tabs
                .iter()
                .zip(state["tabs"].as_array().unwrap())
            {
                compare(*rect, &tab["rect"]);
                assert_eq!(
                    ui_language.text(labels::view(*view)),
                    tab["text"].as_str().unwrap()
                );
            }
            if view == View::Text {
                compare(layout.editor_frame, &widgets["ocrEditor"]["rect"]);
                compare(layout.editor, &widgets["editor_viewport"]["rect"]);
                if let Some(undo) = layout.undo {
                    compare(undo, &widgets["source_undo"]["rect"]);
                }
            } else if view == View::Table {
                compare(layout.table_copy, &widgets["ocrTableCopyButton"]["rect"]);
            }
            count += 1;
        }
    }
    assert_eq!(count, 117);
}

/// 【标题布局测试】【英文状态】保留全部英文原版几何与文案断言，无参数、无返回值
#[test]
fn titlebar_and_body_match_all_english_qt_window_states() {
    compare_language("en");
}

/// 【标题布局测试】【中文状态】中文文案必须同时采用原版宽度和裁切位置，不能沿用英文布局，无参数、无返回值
#[test]
fn titlebar_and_body_match_all_chinese_qt_window_states() {
    compare_language("zh");
}

/// 【标题布局测试】【两种语言反馈】对照两种语言、五种宽度的真实复制状态，中文失败宽度必须采用实测值，无参数、无返回值
#[test]
fn localized_copy_and_table_feedback_match_the_actual_qt_window() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ui-language.json"
    ))
    .unwrap();
    for entry in fixture["languages"].as_array().unwrap() {
        let language = if entry["name"] == "zh" {
            crate::ui::language::Language::Chinese
        } else {
            crate::ui::language::Language::English
        };
        for case in entry["cases"].as_array().unwrap() {
            for state in case["states"].as_array().unwrap() {
                let is_table = state["name"].as_str().unwrap().starts_with("table");
                let widgets = &state["widgets"];
                let layout = Layout::localized(
                    (case["width"].as_u64().unwrap() as u32, 520),
                    true,
                    true,
                    if is_table { View::Table } else { View::Text },
                    widgets["source_copy"]["text"].as_str().unwrap(),
                    widgets["ocrTableCopyButton"]["text"].as_str().unwrap(),
                    false,
                    false,
                    language,
                );
                compare(layout.copy, &widgets["source_copy"]["rect"]);
                if is_table {
                    compare(layout.table_copy, &widgets["ocrTableCopyButton"]["rect"]);
                }
            }
        }
    }
}

/// 【标题布局测试】【反馈宽度】对照真实反馈标签及切页时的复制位置，无参数、无返回值
#[test]
fn feedback_changes_the_copy_width_without_reflowing_tabs() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-titlebar.json"
    ))
    .unwrap();
    for entry in fixture["feedback"]["states"].as_array().unwrap() {
        let state = &entry["state"];
        let view = if state["tab"] == 0 {
            View::Text
        } else {
            View::Table
        };
        let title = Titlebar::new(
            760.0,
            true,
            false,
            view,
            state["widgets"]["source_copy"]["text"].as_str().unwrap(),
        );
        compare(title.copy, &state["widgets"]["source_copy"]["rect"]);
        compare(title.viewport, &state["widgets"]["ocrViewTabs"]["rect"]);
    }
}

/// 【标题布局测试】【裁切命中】窄窗口页签溢出部分不能抢占复制按钮或标题空白，右下角不再假装是页脚入口，无参数、无返回值
#[test]
fn clipped_tabs_and_centered_grip_have_matching_hit_regions() {
    let layout = Layout::new((380, 520), true);
    assert!(layout.titlebar.tabs[2].1.right() > layout.titlebar.viewport.right());
    assert_eq!(
        layout.control((169.0, 19.0), View::Text),
        Some(Control::View(View::Image))
    );
    assert_eq!(layout.control((173.0, 19.0), View::Text), None);
    assert_eq!(
        layout.control((180.0, 19.0), View::Text),
        Some(Control::Copy)
    );
    assert!(layout.resize_at((190.0, 505.0)));
    assert!(!layout.resize_at((371.0, 505.0)));
}
