//! 【字段绘制测试】【字形契约】候选显示、选区前景、隐藏光标与原生逻辑几何采用同一排版

use super::*;
use crate::{
    types::text_input::{Edit, Preedit},
    ui::text_field::validation::Rule,
};

/// 【字段绘制测试】【候选构造】按 text 与 cursor 构造原生候选字段，返回生产状态；不确认候选
fn candidate(text: &str, cursor: Option<(usize, usize)>) -> LineEditState {
    let mut field = LineEditState::new("A".into());
    field.input_method(
        Edit {
            preedit: Preedit {
                text: text.into(),
                cursor,
            },
            ..Default::default()
        },
        Rule::Text,
    );
    field
}

/// 【字段绘制测试】【候选编码】多字节、补充平面和控制字符显示保持合法字节光标，隐藏时仍能定位候选窗口，无参数、无返回值
#[test]
fn candidate_layout_keeps_utf8_boundaries_and_hidden_caret_anchor() {
    let mut fonts = crate::fonts::bundled();
    for (text, cursor) in [
        ("你𠀀", Some((3, 3))),
        ("one\ntwo\t", Some((8, 8))),
        ("ni", None),
    ] {
        let field = candidate(text, cursor);
        let layout = shape(&field, &mut fonts, 16.0, Weight::NORMAL, None);
        assert!(layout.caret.is_finite());
        assert_eq!(layout.caret_visible, cursor.is_some());
        assert_eq!(field.text, "A");
        assert_eq!(layout.buffer.lines.len(), 1);
        assert!(
            layout
                .buffer
                .layout_runs()
                .any(|run| !run.decorations.is_empty())
        );
    }
}

/// 【字段绘制测试】【选择前景】选择覆盖使用字形颜色，而非第二次覆盖文字；未选择字形仍使用默认前景，无参数、无返回值
#[test]
fn selected_foreground_is_carried_by_actual_glyphs_and_rendered_once() {
    let mut fonts = crate::fonts::bundled();
    let mut field = LineEditState::new("MM MM".into());
    field.cursor = 2;
    field.selection_anchor = Some(0);
    let layout = shape(
        &field,
        &mut fonts,
        20.0,
        Weight(600),
        Some(Rgba(0, 255, 0, 255)),
    );
    assert!(!layout.selections.is_empty());
    let glyphs = layout
        .buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter())
        .collect::<Vec<_>>();
    assert!(
        glyphs
            .iter()
            .any(|glyph| glyph.color_opt == Some(cosmic_text::Color::rgb(0, 255, 0)))
    );
    assert!(glyphs.iter().any(|glyph| glyph.color_opt.is_none()));
    let mut image = tiny_skia::Pixmap::new(200, 50).unwrap();
    let mut cache = cosmic_text::SwashCache::new();
    text(
        &mut Painter::new(&mut image).with_text(&mut fonts, &mut cache),
        &layout,
        (0.0, 0.0),
        Color::from_rgba8(255, 0, 0, 255),
    );
    assert!(
        image
            .pixels()
            .iter()
            .any(|pixel| pixel.green() > 0 && pixel.red() == 0)
    );
    assert!(
        image
            .pixels()
            .iter()
            .any(|pixel| pixel.red() > 0 && pixel.green() == 0)
    );
}

/// 【字段绘制测试】【选择回退】无效数值候选保留原选择，实际选择边界在候选之前，不错误吞掉数字，无参数、无返回值
#[test]
fn rejected_numeric_commit_does_not_hide_the_original_selection() {
    let mut fonts = crate::fonts::bundled();
    let mut field = LineEditState::new("20 pt".into());
    field.cursor = 2;
    field.selection_anchor = Some(0);
    assert!(!field.input_method(
        Edit {
            commit: "40你".into(),
            preedit: Preedit {
                text: "ni".into(),
                cursor: Some((2, 2))
            },
            ..Default::default()
        },
        Rule::Integer {
            min: 8,
            max: 300,
            suffix: " pt"
        }
    ));
    assert_eq!(field.displayed_selection(), Some((0, 2)));
    let layout = shape(&field, &mut fonts, 16.0, Weight::NORMAL, None);
    let plain = shape(
        &LineEditState::new("20".into()),
        &mut fonts,
        16.0,
        Weight::NORMAL,
        None,
    );
    assert!(!layout.selections.is_empty());
    assert!((layout.selections[0].1 - plain.caret).abs() < 0.01);
    assert!(layout.caret > plain.caret);
}

/// 【字段绘制测试】【滚动锚点】协议矩形与实际滚动偏移共用同一规则，不随输出像素比例改变，无参数、无返回值
#[test]
fn scrolled_cursor_rect_stays_inside_the_logical_field() {
    let mut fonts = crate::fonts::bundled();
    let field = candidate("long candidate string", Some((21, 21)));
    let layout = shape(&field, &mut fonts, 16.0, Weight::BOLD, None);
    let rect = Rect::from_xywh(13.25, 17.5, 40.0, 28.0).unwrap();
    let shift = offset(&layout, rect.width());
    let cursor = caret_rect(layout.caret, rect, true);
    assert_eq!(
        cursor.0,
        (rect.left() + layout.caret - shift).round() as i32
    );
    assert!(cursor.0 >= rect.left().floor() as i32 && cursor.0 < rect.right().ceil() as i32);
}
