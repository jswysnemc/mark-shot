//! 【文字光标测试】【颜色与选区】真实编辑绘制使用正文颜色，字符选区保留原版插入光标

use std::collections::HashMap;

use cosmic_text::{Action, Edit, Motion, Selection, SwashCache};
use mark_shot::{
    features::text::TextAppearance,
    tools::text::{ensure_text_editor, render_text_annotation, update_text_bbox_inline},
    types::{Annotation, AnnotationShape},
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【文字光标测试】【标注构造】按 content 与 alpha 返回普通二十点测试文字，使用内置字体，不连接显示服务器
fn annotation(content: &str, alpha: u8) -> Annotation {
    Annotation {
        id: 7,
        shape: AnnotationShape::Text {
            start: (10.0, 10.0),
            content: content.into(),
            font_size: 20.0,
            bold: false,
            italic: false,
            outline: None,
            appearance: TextAppearance {
                family: "Inter".into(),
                ..Default::default()
            },
        },
        color: Color::from_rgba8(255, 77, 77, alpha),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 0.0,
        bbox: Rect::from_xywh(10.0, 10.0, 1.0, 1.0).unwrap(),
    }
}

/// 【文字光标测试】【空文档绘制】按 alpha 生成真实编辑像素，返回非透明像素；空文档没有字形或背景
fn empty_pixels(alpha: u8) -> Vec<tiny_skia::PremultipliedColorU8> {
    let mut ann = annotation("", alpha);
    let mut fonts = mark_shot::fonts::bundled();
    let mut editors = HashMap::new();
    let editor = ensure_text_editor(&ann, &mut editors, &mut fonts);
    update_text_bbox_inline(&mut ann, editor, &mut fonts);
    let mut canvas = Pixmap::new(160, 100).unwrap();
    render_text_annotation(
        &ann,
        editor,
        &mut fonts,
        &mut SwashCache::new(),
        &mut canvas.as_mut(),
        (0.0, 0.0),
        1.0,
        true,
    );
    canvas
        .pixels()
        .iter()
        .copied()
        .filter(|pixel| pixel.alpha() != 0)
        .collect()
}

/// 【文字光标测试】【正文颜色】空文档插入光标采用原版文字前景红色，而不是共用字段白色，无参数、无返回值
#[test]
fn annotation_caret_uses_its_actual_text_color() {
    let pixels = empty_pixels(255);
    assert!(!pixels.is_empty());
    assert!(pixels.iter().all(|pixel| pixel.red() == 255
        && pixel.green() == 77
        && pixel.blue() == 77
        && pixel.alpha() == 255));
}

/// 【文字光标测试】【前景透明度】光标与原版前景透明度一致，只做一次预乘合成，无参数、无返回值
#[test]
fn annotation_caret_preserves_text_alpha() {
    let pixels = empty_pixels(128);
    assert!(!pixels.is_empty());
    assert!(pixels.iter().all(|pixel| pixel.red() == 128
        && pixel.green() == 38
        && pixel.blue() == 38
        && pixel.alpha() == 128));
}

/// 【文字光标测试】【选区共存】原版选中尾部空格时仍绘制字符选区与红色插入光标，无参数、无返回值
#[test]
fn selected_text_keeps_the_insertion_caret() {
    let mut ann = annotation(" ", 255);
    let mut fonts = mark_shot::fonts::bundled();
    let mut editors = HashMap::new();
    let editor = ensure_text_editor(&ann, &mut editors, &mut fonts);
    update_text_bbox_inline(&mut ann, editor, &mut fonts);
    editor.action(&mut fonts, Action::Motion(Motion::BufferStart));
    editor.set_selection(Selection::Normal(editor.cursor()));
    editor.action(&mut fonts, Action::Motion(Motion::BufferEnd));
    let mut canvas = Pixmap::new(160, 100).unwrap();
    render_text_annotation(
        &ann,
        editor,
        &mut fonts,
        &mut SwashCache::new(),
        &mut canvas.as_mut(),
        (0.0, 0.0),
        1.0,
        true,
    );
    assert!(canvas.pixels().iter().any(|pixel| pixel.alpha() != 0));
    assert!(
        canvas
            .pixels()
            .iter()
            .any(|pixel| pixel.red() == 255 && pixel.green() == 77 && pixel.blue() == 77)
    );
}
