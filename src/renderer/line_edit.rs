//! 【字段绘制】【候选布局】字段显示、选区、装饰和原生候选位置共用真实字形，不测量候选的提交正文

use super::{
    Painter,
    text::{TextStyle, shape_single_line},
    text_preedit,
};
use crate::{theme::Rgba, ui::text_field::LineEditState};
use cosmic_text::{AttrsOwned, Buffer, Cursor, FontSystem, Style, UnderlineStyle, Weight};
use tiny_skia::{Color, Paint, Rect, Transform};

#[cfg(test)]
mod tests;

pub(crate) struct Shape {
    pub buffer: Buffer,
    pub height: f32,
    pub caret: f32,
    pub caret_visible: bool,
    pub selections: Vec<(f32, f32)>,
}

/// 【字段绘制】【布局计算】按 field、fonts、size、weight 排版显示文字，selected_color 可覆盖选择前景；返回实际光标与选择几何
pub(crate) fn shape(
    field: &LineEditState,
    fonts: &mut FontSystem,
    size: f32,
    weight: Weight,
    selected_color: Option<Rgba>,
) -> Shape {
    let displayed = field
        .composition
        .as_ref()
        .map_or(field.text.as_str(), |composition| &composition.display);
    let normalized = if displayed.contains(['\n', '\r']) {
        std::borrow::Cow::Owned(displayed.replace(['\n', '\r'], " "))
    } else {
        std::borrow::Cow::Borrowed(displayed)
    };
    let (mut buffer, _, height) =
        shape_single_line(fonts, &normalized, size, weight, Style::Normal);
    let selection = field.displayed_selection();
    let preedit = field.composition.as_ref().map(|composition| {
        (
            composition.position,
            composition.position + composition.preedit.text.len(),
        )
    });
    if preedit.is_some() || (selection.is_some() && selected_color.is_some()) {
        let line = &mut buffer.lines[0];
        let defaults = AttrsOwned::new(&line.attrs_list().defaults());
        let mut attrs = line.attrs_list().clone();
        let mut edges = vec![0, normalized.len()];
        if let Some((start, end)) = selection {
            edges.extend([start, end]);
        }
        if let Some((start, end)) = preedit {
            edges.extend([start, end]);
        }
        edges.sort_unstable();
        edges.dedup();
        for pair in edges.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            if start >= end || end > normalized.len() {
                continue;
            }
            let mut value = defaults.as_attrs();
            if preedit.is_some_and(|(from, to)| start >= from && end <= to) {
                value = value.underline(UnderlineStyle::Single);
            }
            if selection.is_some_and(|(from, to)| start >= from && end <= to)
                && let Some(Rgba(r, g, b, a)) = selected_color
            {
                value = value.color(cosmic_text::Color::rgba(r, g, b, a));
            }
            attrs.add_span(start..end, &value);
        }
        line.set_attrs_list(attrs);
        buffer.shape_until_scroll(fonts, false);
    }
    let cursor = field.displayed_cursor().unwrap_or_else(|| {
        field
            .composition
            .as_ref()
            .map_or(field.cursor_byte(), |composition| composition.position)
    });
    let caret = buffer
        .cursor_position(&Cursor::new(0, cursor))
        .map_or(0.0, |(x, _)| x);
    let selections = selection.map_or_else(Vec::new, |(start, end)| {
        buffer
            .layout_runs()
            .flat_map(|run| run.highlight(Cursor::new(0, start), Cursor::new(0, end)))
            .collect()
    });
    Shape {
        buffer,
        height,
        caret,
        caret_visible: field.displayed_cursor().is_some(),
        selections,
    }
}

/// 【字段绘制】【可见偏移】按 shape 与 width 返回保持光标可见的横向偏移，不改变编辑内容
pub(crate) fn offset(shape: &Shape, width: f32) -> f32 {
    (shape.caret - width + 2.0).max(0.0)
}

/// 【字段绘制】【协议矩形】按 shape、rect 返回与实际字段绘制相同的逻辑光标，scroll 控制横向裁切；不乘设备比例
pub(crate) fn caret_rect(cursor_x: f32, rect: Rect, scroll: bool) -> (i32, i32, i32, i32) {
    let height = rect.height() * crate::config::get().theme.field_text_height;
    let shift = if scroll {
        (cursor_x - rect.width() + 2.0).max(0.0)
    } else {
        0.0
    };
    (
        (rect.left() + cursor_x - shift).round() as i32,
        (rect.top() + (rect.height() - height) / 2.0).floor() as i32,
        crate::config::get().theme.caret_width.ceil().max(1.0) as i32,
        height.ceil().max(1.0) as i32,
    )
}

/// 【字段绘制】【字形装饰】将 shape 绘入 p 的 origin，ink 为默认前景；字形与下划线使用同一个缩放锚点，无返回值
pub(crate) fn text(p: &mut Painter, shape: &Shape, origin: (f32, f32), ink: Color) {
    p.text_buffer(&shape.buffer, origin, ink);
    let scale = p.scale();
    let origin = (origin.0 - p.origin.0, origin.1 - p.origin.1);
    let rgba = ink.to_color_u8();
    let mut paint = Paint::default();
    for run in shape.buffer.layout_runs() {
        text_preedit::draw_at(
            &run,
            scale,
            origin,
            cosmic_text::Color::rgba(rgba.red(), rgba.green(), rgba.blue(), rgba.alpha()),
            &mut |x, y, w, h, color| {
                if let Some(rect) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
                    paint.set_color_rgba8(color.r(), color.g(), color.b(), color.a());
                    paint.anti_alias = false;
                    p.canvas
                        .fill_rect(rect, &paint, Transform::identity(), None);
                }
            },
        );
    }
}

impl Painter<'_> {
    /// 【字段绘制】【单行字段】按 rect、display_text、editing、size、ink、weight 绘制文本与候选；字段裁切不覆盖相邻控件，无返回值
    pub fn line_edit(
        &mut self,
        rect: Rect,
        display_text: &str,
        editing: Option<&LineEditState>,
        size: f32,
        ink: Color,
        weight: Weight,
    ) {
        let Some(field) = editing else {
            self.text(rect, display_text, TextStyle::new(size, ink).weight(weight));
            return;
        };
        let Some(fonts) = self.fonts.as_mut() else {
            return;
        };
        let shape = shape(field, fonts.system, size, weight, None);
        let shift = offset(&shape, rect.width());
        let mut cache = None;
        self.panel_unframed(
            &mut cache,
            (rect.left(), rect.top()),
            (rect.width(), rect.height()),
            1.0,
            true,
            |p| {
                let inner =
                    Rect::from_xywh(-shift, 0.0, rect.width() + shift, rect.height()).unwrap();
                for &(x, w) in &shape.selections {
                    p.text_selection(inner, x, x + w);
                }
                if shape.caret_visible {
                    p.text_caret(inner, shape.caret);
                }
                if field.composition.is_some() || field.text.contains(['\n', '\r']) {
                    text(
                        p,
                        &shape,
                        (-shift, (rect.height() - shape.height) / 2.0),
                        ink,
                    );
                } else if !display_text.is_empty() {
                    p.text(
                        inner,
                        display_text,
                        TextStyle::new(size, ink).weight(weight),
                    );
                }
            },
        );
    }
}
