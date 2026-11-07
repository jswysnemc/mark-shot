//! 【OCR 翻译测试】【实际几何】语言行、分栏、译文正文和提示逐项比较真实 Qt 整数矩形

use super::*;
use serde_json::{Value, json};

/// 【OCR 翻译测试】【矩形记录】按 rect 返回整数位置与尺寸，不改变原始逻辑几何
fn record(rect: Rect) -> Value {
    json!([
        rect.x() as i32,
        rect.y() as i32,
        rect.width() as i32,
        rect.height() as i32
    ])
}

/// 【OCR 翻译测试】【六宽度几何】对照全部可见译文记录，包含忙碌、编辑、反馈和一行/两行过期提示，无参数、无返回值
#[test]
fn layout_matches_visible_actual_qt_translation_geometry() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/qt-ocr-translation.json"
    ))
    .unwrap();
    let mut fonts = crate::fonts::ocr_ui::new();
    for case in fixture["cases"].as_array().unwrap() {
        for step in case["states"].as_array().unwrap() {
            let widgets = &step["state"]["widgets"];
            if !widgets["ocrTranslationPane"]["visible"].as_bool().unwrap() {
                continue;
            }
            let size = &step["state"]["size"];
            let notice = widgets["translation:ocrPaneNotice"]["text"]
                .as_str()
                .unwrap();
            let layout = Layout::new(
                (
                    size[0].as_f64().unwrap() as f32,
                    size[1].as_f64().unwrap() as f32,
                ),
                0.6,
                widgets["translation:ocrUndoButton"]["visible"]
                    .as_bool()
                    .unwrap(),
                widgets["translation:ocrCopyButton"]["text"]
                    .as_str()
                    .unwrap(),
                notice,
                widgets["ocrTranslationEditor"]["read_only"]
                    .as_bool()
                    .unwrap(),
                &mut fonts,
            );
            for (name, rect) in [
                ("ocrTranslationActions", layout.actions),
                ("ocrLanguageCombo", layout.language),
                ("language:line_edit", layout.language_text),
                ("ocrTranslateButton", layout.translate),
                ("ocrSourcePane", layout.source),
                ("ocrTranslationPane", layout.pane),
                ("translation:ocrPaneHeader", layout.header),
                ("translation:ocrCopyButton", layout.copy),
                ("ocrTranslationEditor", layout.frame),
            ] {
                assert_eq!(
                    record(rect),
                    widgets[name]["rect"],
                    "{} {} {name}",
                    case["width"],
                    step["name"]
                );
            }
            for (name, rect) in [
                ("language:caption", layout.caption),
                ("translation:ocrUndoButton", layout.undo),
                ("translation:ocrPaneNotice", layout.notice),
                ("translation:ocrTranslationProgress", layout.progress),
            ] {
                assert_eq!(
                    rect.is_some(),
                    widgets[name]["visible"].as_bool().unwrap(),
                    "{name} {step}"
                );
                if let Some(rect) = rect {
                    assert_eq!(record(rect), widgets[name]["rect"], "{name} {step}");
                }
            }
        }
    }
}

/// 【OCR 翻译测试】【语言几何】对照原版两种语言和五种宽度的实际目标行、复制和提示，不能把英文标签宽度用于中文，无参数、无返回值
#[test]
fn localized_actions_and_notices_match_actual_qt_geometry() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/qt-ui-language.json"
    ))
    .unwrap();
    let mut fonts = crate::fonts::ocr_ui::new();
    for entry in fixture["languages"].as_array().unwrap() {
        let language = if entry["name"] == "zh" {
            crate::ui::language::Language::Chinese
        } else {
            crate::ui::language::Language::English
        };
        crate::fonts::ocr_ui::ensure_text(&mut fonts, language.font_sample());
        for case in entry["cases"].as_array().unwrap() {
            for state in case["states"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|state| matches!(state["name"].as_str(), Some("busy" | "canceled")))
            {
                let widgets = &state["widgets"];
                let layout = Layout::localized(
                    (case["width"].as_f64().unwrap() as f32, 520.0),
                    0.6,
                    false,
                    widgets["translation:ocrCopyButton"]["text"]
                        .as_str()
                        .unwrap(),
                    widgets["translation:ocrPaneNotice"]["text"]
                        .as_str()
                        .unwrap(),
                    state["name"] == "busy",
                    &mut fonts,
                    language,
                );
                for (name, rect) in [
                    ("ocrTranslationActions", layout.actions),
                    ("ocrLanguageCombo", layout.language),
                    ("ocrTranslateButton", layout.translate),
                    ("translation:ocrCopyButton", layout.copy),
                    ("translation:ocrPaneNotice", layout.notice.unwrap()),
                ] {
                    assert_eq!(
                        record(rect),
                        widgets[name]["rect"],
                        "{} {} {} {name}",
                        entry["name"],
                        case["width"],
                        state["name"]
                    );
                }
                if let Some(caption) = layout.caption {
                    assert_eq!(record(caption), widgets["caption"]["rect"]);
                }
            }
        }
    }
}
