//! 【表格绘制测试】【字形存在】通过真实绘制入口验证嵌套裁切中的每格文字

use super::*;

/// 【表格绘制测试】【单元格字形】真实表格绘制入口必须在正文每格绘出字形，不能只验证选区与边线，无参数、无返回值
#[test]
fn every_cell_has_real_text_pixels_inside_nested_clipping() {
    use super::super::table_view::TableView;
    let mut fonts = crate::fonts::ocr_ui::new();
    let mut cache = cosmic_text::SwashCache::new();
    let table = Table {
        rows: vec![
            vec!["Item".into(), "Count".into(), "Total".into()],
            vec!["Paper".into(), "12".into(), "36".into()],
            vec!["Ink".into(), "2".into(), "24".into()],
        ],
    };
    let mut view = TableView::new(table, &mut fonts);
    let body = Rect::from_xywh(0.0, 0.0, 744.0, 359.0).unwrap();
    let grid = view.grid(body);
    let mut canvas = tiny_skia::Pixmap::new(744, 359).unwrap();
    view.draw(&mut fonts, &mut cache, &mut canvas, body, 1.0, false);
    let base = Palette::current().base.color().premultiply().to_color_u8();
    for row in 0..3 {
        for column in 0..3 {
            let (left, width) = grid.column(column);
            let (x, y) = (
                grid.viewport.x() + left,
                grid.viewport.y() + row as f32 * 28.0,
            );
            let mut glyph_pixels = 0;
            for py in (y + 2.0) as u32..(y + 25.0) as u32 {
                for px in (x + 3.0) as u32..(x + width - 4.0) as u32 {
                    glyph_pixels += usize::from(canvas.pixel(px, py).unwrap() != base);
                }
            }
            assert!(
                glyph_pixels > 10,
                "cell ({row}, {column}) has only {glyph_pixels} text pixels"
            );
        }
    }
}
