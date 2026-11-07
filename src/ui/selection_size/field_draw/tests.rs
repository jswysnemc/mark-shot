//! 【选区尺寸测试】【鼠标字形】用原版已绘制字段的指针样本核对字符命中与双击选择范围

use super::*;
use crate::ui::text_field::LineEditState;
use serde_json::Value;

/// 【选区尺寸测试】【鼠标参考】返回生产 Qt 已完成正文绘制的鼠标样本，不连接显示服务器
fn reference() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-selection-size/field-mouse.json"
    ))
    .unwrap()
}

/// 【选区尺寸测试】【输入布局】按 text 返回与原版初始化相同的数字字段，正文光标位于末尾
fn field(text: &str) -> NumberField {
    let mut field = NumberField::new(12345, 10000000);
    field.begin();
    field.editor = LineEditState::new(text.into());
    field
}

/// 【选区尺寸测试】【真实单击】65 个 Qt 字段坐标在当前排版下命中相同光标，不从字符宽度推算预期
#[test]
fn pointer_positions_match_production_line_edit() {
    let reference = reference();
    let mut fonts = crate::fonts::selection_ui::new();
    let mut cache = SwashCache::new();
    let rect = Rect::from_xywh(0.0, 0.0, 47.0, 20.0).unwrap();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        if sample["action"] != "click" {
            continue;
        }
        let field = field(sample["text"].as_str().unwrap());
        let cursor = index(
            &field,
            rect,
            &mut fonts,
            &mut cache,
            sample["x"].as_f64().unwrap() as f32,
        );
        assert_eq!(
            cursor,
            sample["hit"]["cursor"].as_u64().unwrap() as usize,
            "{sample}"
        );
        count += 1;
    }
    assert_eq!(count, 65);
}

/// 【选区尺寸测试】【生产度量】65个字段状态的定点前进量、自然宽度和整数滚动与 Qt 内部实际值相等
#[test]
fn glyph_metrics_and_scroll_match_production_line_edit() {
    let reference = reference();
    let mut fonts = crate::fonts::selection_ui::new();
    let mut cache = SwashCache::new();
    let rect = Rect::from_xywh(0.0, 0.0, 47.0, 20.0).unwrap();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        if sample["action"] != "click" {
            continue;
        }
        let field = field(sample["text"].as_str().unwrap());
        let mut shaped = shape(&field, rect, &mut fonts, &mut cache);
        let metrics = &sample["initial_metrics"];
        let natural =
            super::super::metrics::fixed(&mut shaped.text, &field.editor, &mut fonts, &mut cache);
        assert_eq!(
            f64::from(natural),
            metrics["natural_width"].as_f64().unwrap(),
            "{sample}"
        );
        assert_eq!(
            f64::from(shaped.x),
            5.0 - metrics["hscroll"].as_f64().unwrap(),
            "{sample}"
        );
        let run = shaped.text.runs().next().unwrap();
        assert_eq!(
            f64::from(run.line_y - run.line_top),
            metrics["ascent"].as_f64().unwrap(),
            "{sample}"
        );
        assert_eq!(
            f64::from(run.line_height),
            metrics["line_height"].as_f64().unwrap(),
            "{sample}"
        );
        assert_eq!(
            f64::from(shaped.text.height),
            metrics["line_height"].as_f64().unwrap(),
            "{sample}"
        );
        for (index, expected) in metrics["positions"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                run.cursor_position(&cosmic_text::Cursor::new(0, index))
                    .map(f64::from),
                expected.as_f64(),
                "{sample}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 65);
}

/// 【选区尺寸测试】【原生光标】32个实际 Qt 输入法矩形与当前字段协议矩形相等，保留输入框十像素宽的光标范围
#[test]
fn native_caret_rectangles_match_production_line_edit() {
    let reference: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-selection-size/field-navigation-index.json"
    ))
    .unwrap();
    let mut fonts = crate::fonts::selection_ui::new();
    let mut cache = SwashCache::new();
    let rect = Rect::from_xywh(0.0, 0.0, 47.0, 20.0).unwrap();
    let mut count = 0;
    for sample in reference["carets"].as_array().unwrap() {
        let mut field = field(sample["text"].as_str().unwrap());
        for (index, expected) in sample["rects"].as_array().unwrap().iter().enumerate() {
            field.editor.move_to(index, false);
            let shape = shape(&field, rect, &mut fonts, &mut cache);
            let actual = caret(&shape, rect);
            let expected = (
                expected[0].as_i64().unwrap() as i32,
                expected[1].as_i64().unwrap() as i32,
                expected[2].as_i64().unwrap() as i32,
                expected[3].as_i64().unwrap() as i32,
            );
            assert_eq!(actual, expected, "{} cursor={index}", field.editor.text);
            count += 1;
        }
    }
    assert_eq!(count, 32);
}

/// 【选区尺寸测试】【真实双击】先完成第一次定位再选择词语，验证65个 Qt 数字段、正号与分组符选择
#[test]
fn double_click_words_match_production_line_edit() {
    let reference = reference();
    let mut fonts = crate::fonts::selection_ui::new();
    let mut cache = SwashCache::new();
    let rect = Rect::from_xywh(0.0, 0.0, 47.0, 20.0).unwrap();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        if sample["action"] != "double" {
            continue;
        }
        let mut field = field(sample["text"].as_str().unwrap());
        let x = sample["x"].as_f64().unwrap() as f32;
        let cursor = index(&field, rect, &mut fonts, &mut cache, x);
        field.editor.move_to(cursor, false);
        let character = character(&field, rect, &mut fonts, &mut cache, x);
        let (start, end) =
            crate::ui::selection_size::navigation::word(&field.editor.text, character);
        field.editor.move_to(start, false);
        field.editor.move_to(end, true);
        assert_eq!(
            field.editor.selected_text().unwrap_or_default(),
            sample["after"]["selected"].as_str().unwrap(),
            "{sample}"
        );
        assert_eq!(
            field.editor.cursor,
            sample["after"]["cursor"].as_u64().unwrap() as usize,
            "{sample}"
        );
        count += 1;
    }
    assert_eq!(count, 65);
}

/// 【选区尺寸测试】【改写拖选】130个原版上下键与滚轮改写样本继续使用当前光标锚点和新正文排版
#[test]
fn pointer_after_program_replacement_matches_production_line_edit() {
    let reference = reference();
    let mut fonts = crate::fonts::selection_ui::new();
    let mut cache = SwashCache::new();
    let rect = Rect::from_xywh(0.0, 0.0, 47.0, 20.0).unwrap();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        let action = sample["action"].as_str().unwrap();
        if !matches!(action, "up_drag" | "wheel_drag") {
            continue;
        }
        let mut field = field(sample["text"].as_str().unwrap());
        let x = sample["x"].as_f64().unwrap() as f32;
        let cursor = index(&field, rect, &mut fonts, &mut cache, x);
        field.editor.move_to(cursor, false);
        let shaped = shape(&field, rect, &mut fonts, &mut cache);
        assert_eq!(
            f64::from(shaped.x),
            5.0 - sample["hit_metrics"]["hscroll"].as_f64().unwrap(),
            "{sample}"
        );
        // 1. 【选区尺寸测试】【程序微调】按生产事件改写正文，保留同一控件的鼠标拖选
        if action == "up_drag" {
            field.arrow(1);
        } else {
            field.wheel(1);
        }
        assert_eq!(
            field.editor.text,
            sample["program"]["text"].as_str().unwrap(),
            "{sample}"
        );
        let shaped = shape(&field, rect, &mut fonts, &mut cache);
        assert_eq!(
            f64::from(shaped.x),
            5.0 - sample["program_metrics"]["hscroll"].as_f64().unwrap(),
            "{sample}"
        );
        let cursor = index(&field, rect, &mut fonts, &mut cache, x - 20.0);
        field.editor.move_to(cursor, true);
        assert_eq!(
            field.editor.cursor,
            sample["after"]["cursor"].as_u64().unwrap() as usize,
            "{sample}"
        );
        assert_eq!(
            field.editor.selected_text().unwrap_or_default(),
            sample["after"]["selected"].as_str().unwrap(),
            "{sample}"
        );
        count += 1;
    }
    assert_eq!(count, 130);
}
