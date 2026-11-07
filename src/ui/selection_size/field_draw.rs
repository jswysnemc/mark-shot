//! 【选区尺寸】【字段绘制】居中输入、选区背景和原生候选位置共用字形排版

use super::{field::NumberField, layout::FONT_SIZE, palette::Colors};
use crate::{
    renderer::{Painter, line_edit},
    ui::controls,
};
use cosmic_text::{FontSystem, SwashCache, Weight};
use tiny_skia::Rect;

#[cfg(test)]
mod tests;

pub(crate) struct Shape {
    text: line_edit::Shape,
    x: f32,
    y: f32,
    height: f32,
}

/// 【选区尺寸】【字段字形】按 field、rect、fonts 和 cache 返回定点正文与候选布局，长文字保持光标可见
pub(crate) fn shape(
    field: &NumberField,
    rect: Rect,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) -> Shape {
    crate::fonts::selection_ui::prepare(
        fonts,
        field
            .editor
            .composition
            .as_ref()
            .map_or(field.editor.text.as_str(), |composition| {
                &composition.display
            }),
    );
    let mut text = line_edit::shape(&field.editor, fonts, FONT_SIZE, Weight(600), None);
    let width = super::metrics::fixed(&mut text, &field.editor, fonts, cache);
    let x = field.scroll.x(width, text.caret, rect.width());
    let height = 18.0_f32.min(rect.height() - 2.0);
    Shape {
        y: 1.0 + ((rect.height() - 2.0 - text.height.round()) / 2.0).trunc(),
        text,
        x,
        height,
    }
}

/// 【选区尺寸】【字符命中】按 field、rect、fonts、cache 和 x 返回定点正文对应字符位置，候选在调用前取消
pub(crate) fn index(
    field: &NumberField,
    rect: Rect,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
    x: f32,
) -> usize {
    let shape = shape(field, rect, fonts, cache);
    let local = (x - rect.left()).round() - shape.x;
    shape.text.hit(local, shape.text.height / 2.0).map_or_else(
        || {
            if local < 0.0 {
                0
            } else {
                field.editor.text.chars().count()
            }
        },
        |cursor| {
            field.editor.text[..cursor.index.min(field.editor.text.len())]
                .chars()
                .count()
        },
    )
}

/// 【选区尺寸】【词语命中】按 field、rect、fonts、cache 和 x 返回最近光标对应字符，末尾取最后正文字符
pub(crate) fn character(
    field: &NumberField,
    rect: Rect,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
    x: f32,
) -> usize {
    index(field, rect, fonts, cache, x).min(field.editor.text.chars().count().saturating_sub(1))
}

/// 【选区尺寸】【光标矩形】按 shape 和 rect 返回原版输入框的扩展逻辑光标范围，包含两侧留白及额外一个像素高度
pub(crate) fn caret(shape: &Shape, rect: Rect) -> (i32, i32, i32, i32) {
    let height = shape.text.height.round().max(1.0) + 1.0;
    (
        (rect.left() + shape.x + shape.text.caret).round() as i32 - 5,
        (rect.top() + 1.0 + ((rect.height() - 2.0 - height) / 2.0).trunc()).round() as i32,
        10,
        height as i32,
    )
}

/// 【选区尺寸】【输入绘制】在 p 的 rect 按 field、shape、colors 和 caret_visible 绘制编辑字段，无返回值
pub(crate) fn draw(
    p: &mut Painter,
    rect: Rect,
    shape: &Shape,
    colors: Colors,
    caret_visible: bool,
) {
    controls::frame(p, rect, colors.editor_fill, colors.editor_border, 5.0);
    let mut cache = None;
    p.panel_unframed(
        &mut cache,
        (rect.left() + 1.0, rect.top() + 1.0),
        (rect.width() - 2.0, rect.height() - 2.0),
        1.0,
        true,
        |p| {
            let x = shape.x - 1.0;
            let top = (rect.height() - shape.height) / 2.0 - 1.0;
            for &(start, width) in &shape.text.selections {
                p.fill_rounded(
                    Rect::from_xywh(x + start, top, width.max(1.0), shape.height).unwrap(),
                    0.0,
                    colors.editor_selection,
                );
            }
            line_edit::text(
                p,
                &shape.text,
                (x, shape.y - 1.0),
                colors.editor_text.color(),
            );
            if caret_visible && shape.text.caret_visible {
                p.fill_rounded(
                    Rect::from_xywh((x + shape.text.caret).round(), top, 1.0, shape.height)
                        .unwrap(),
                    0.0,
                    colors.editor_text,
                );
            }
        },
    );
}
