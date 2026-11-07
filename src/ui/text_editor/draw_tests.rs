//! 【编辑器绘制测试】【独立视区】验证额外裁切不改变字形位置、候选或单行长度限制

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Pixmap, Rect};

use super::{EditorStyle, TextEditor};
use crate::theme::Rgba;

/// 【编辑器绘制测试】【固定样式】用 fonts 返回可比较的单行编辑器，选中全文，颜色不依赖用户主题
fn editor(fonts: &mut FontSystem) -> TextEditor {
    let mut editor = TextEditor::new_single_line_with_style(
        "abcdefghij",
        fonts,
        EditorStyle {
            size: 13.0,
            line_height: 18.0,
            colors: Some([
                Rgba(220, 220, 220, 255),
                Rgba(255, 255, 255, 255),
                Rgba(50, 160, 120, 255),
                Rgba(10, 10, 10, 255),
            ]),
        },
    );
    editor.select_all(fonts);
    editor
}

/// 【编辑器绘制测试】【分数裁切】对照完整字段与额外视区，视区内像素及偏移一致，视区外完全不变，无参数、无返回值
#[test]
fn clipped_single_line_keeps_original_glyph_coordinates_and_outside_pixels() {
    for scale in [1.0, 1.25, 2.0] {
        let mut fonts = crate::fonts::bundled();
        let mut cache = SwashCache::new();
        let mut full = Pixmap::new((100.0 * scale) as u32, (50.0 * scale) as u32).unwrap();
        full.fill(tiny_skia::Color::from_rgba8(1, 2, 3, 255));
        let before = full.clone();
        let mut clipped = full.clone();
        let rect = Rect::from_xywh(4.0, 3.0, 80.0, 35.0).unwrap();
        let viewport = Rect::from_xywh(25.0, 14.0, 40.0, 20.0).unwrap();
        let mut full_offset = 0.0;
        editor(&mut fonts).draw_single_line(
            &mut fonts,
            &mut cache,
            &mut full,
            rect,
            scale,
            true,
            &mut full_offset,
        );
        let mut clipped_offset = 0.0;
        editor(&mut fonts).draw_single_line_clipped(
            &mut fonts,
            &mut cache,
            &mut clipped,
            rect,
            viewport,
            scale,
            true,
            &mut clipped_offset,
        );
        assert_eq!(full_offset, clipped_offset);
        let mut changed = 0;
        for y in 0..clipped.height() {
            for x in 0..clipped.width() {
                let inside = x >= (viewport.x() * scale).round() as u32
                    && x < (viewport.right() * scale).ceil() as u32
                    && y >= (viewport.y() * scale).round() as u32
                    && y < (viewport.bottom() * scale).ceil() as u32;
                let actual = clipped.pixel(x, y).unwrap();
                assert_eq!(
                    actual,
                    if inside {
                        full.pixel(x, y).unwrap()
                    } else {
                        before.pixel(x, y).unwrap()
                    },
                    "{scale} ({x}, {y})"
                );
                changed += usize::from(actual != before.pixel(x, y).unwrap());
            }
        }
        assert!(changed > 20);
    }
}

/// 【编辑器绘制测试】【样式长度限制】局部字号不取消 Qt 单行 UTF-16 上限，补充字符不截断到半个字符，无参数、无返回值
#[test]
fn styled_single_line_retains_original_utf16_limit() {
    let mut fonts = crate::fonts::bundled();
    let text = format!("{}𠮷", "a".repeat(32766));
    let editor = TextEditor::new_single_line_with_style(
        &text,
        &mut fonts,
        EditorStyle {
            size: 13.0,
            line_height: 18.0,
            colors: None,
        },
    );
    assert_eq!(editor.text(), "a".repeat(32766));
}
