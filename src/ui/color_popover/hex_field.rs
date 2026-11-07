//! 【属性取色】【文本绘制】使用原版字体、青色选择背景与白色光标，不继承旧主题输入样式

use super::{ColorField, ColorPickerPopover, layout, tracks};
use crate::renderer::{Painter, line_edit};
use cosmic_text::{FontSystem, Weight};
use tiny_skia::Rect;

pub(super) type HexRender = line_edit::Shape;

/// 【属性取色】【文本排版】由 picker 与 fonts 生成字段字形和加权光标坐标，返回绘制数据，不修改输入内容
pub(super) fn shape(picker: &ColorPickerPopover, fonts: &mut FontSystem) -> HexRender {
    let editing = picker
        .fields
        .editing
        .as_ref()
        .filter(|edit| edit.key == ColorField::Hex);
    if let Some(edit) = editing {
        line_edit::shape(
            &edit.field,
            fonts,
            12.0,
            Weight(600),
            Some(crate::theme::toolbar::Palette::current().hover_text),
        )
    } else {
        let field = crate::ui::text_field::LineEditState::new(picker.field_text(ColorField::Hex));
        let mut shape = line_edit::shape(&field, fonts, 12.0, Weight(600), None);
        shape.caret_visible = false;
        shape
    }
}

/// 【属性取色】【字段绘制】按 picker 与 render 绘入 p；文本只占实际输入框，光标与选区使用相同字形坐标
pub(super) fn draw(p: &mut Painter, picker: &ColorPickerPopover, render: &HexRender) {
    let rect = layout::hex((0.0, 0.0));
    let inner = Rect::from_xywh(
        rect.left() + 0.5,
        rect.top() + 0.5,
        rect.width() - 1.0,
        rect.height() - 1.0,
    )
    .unwrap();
    p.fill_rounded(
        inner,
        6.0,
        crate::theme::toolbar::Palette::current().control_fill,
    );
    let editing = picker.fields.is_editing();
    tracks::border(
        p,
        inner,
        6.0,
        if editing {
            crate::theme::toolbar::Palette::current().focus_border
        } else {
            crate::theme::toolbar::Palette::current().control_border
        },
    );
    let width = rect.width() - 22.0;
    let shift = line_edit::offset(render, width);
    let mut cache = None;
    p.panel_unframed(
        &mut cache,
        (rect.left() + 11.0, rect.top()),
        (width, rect.height()),
        1.0,
        true,
        |p| {
            for &(x, w) in &render.selections {
                p.fill_rounded(
                    Rect::from_xywh(x - shift, 5.0, w.max(1.0), 16.0).unwrap(),
                    0.0,
                    crate::theme::toolbar::Palette::current().hover_fill,
                );
            }
            line_edit::text(
                p,
                render,
                (-shift, (rect.height() - render.height) / 2.0 + 1.0),
                crate::theme::toolbar::Palette::current().text.color(),
            );
            if render.caret_visible && render.selections.is_empty() {
                p.fill_rounded(
                    Rect::from_xywh((render.caret - shift).round(), 5.0, 1.0, 16.0).unwrap(),
                    0.0,
                    crate::theme::toolbar::Palette::current().text,
                );
            }
        },
    );
}
