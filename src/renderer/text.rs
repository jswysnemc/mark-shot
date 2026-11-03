use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Style, SwashContent, Weight};
use tiny_skia::{Color, Paint, Pixmap, PixmapPaint, PremultipliedColorU8, Rect, Transform};

use super::Painter;
use crate::theme::{color, font};

#[derive(Clone, Copy)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

pub fn shape_single_line(
    font_system: &mut FontSystem,
    text: &str,
    font_size: f32,
    weight: cosmic_text::Weight,
    style: cosmic_text::Style,
) -> (Buffer, f32, f32) {
    let line_height = font_size * font::line_height();
    let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));

    buffer.set_size(None, None);
    buffer.set_text(
        text,
        &Attrs::new().weight(weight).style(style),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);

    let mut text_width: f32 = 0.0;
    let mut text_height: f32 = 0.0;
    for run in buffer.layout_runs() {
        text_width = text_width.max(run.line_w);
        text_height += run.line_height;
    }

    (buffer, text_width, text_height)
}

pub fn measure_line_width(text: &str, font_size: f32, font_system: &mut FontSystem) -> f32 {
    shape_single_line(
        font_system,
        text,
        font_size,
        cosmic_text::Weight::NORMAL,
        cosmic_text::Style::Normal,
    )
    .1
}

#[derive(Clone, Copy)]
pub struct TextStyle {
    pub size: f32,
    pub color: Color,
    pub align: HAlign,
    pub weight: Weight,
    pub style: Style,
}

impl TextStyle {
    pub fn new(size: f32, color: impl Into<Color>) -> Self {
        Self {
            size,
            color: color.into(),
            align: HAlign::Left,
            weight: Weight::NORMAL,
            style: Style::Normal,
        }
    }

    pub fn center(self) -> Self {
        Self {
            align: HAlign::Center,
            ..self
        }
    }

    pub fn weight(self, weight: Weight) -> Self {
        Self { weight, ..self }
    }

    pub fn style(self, style: Style) -> Self {
        Self { style, ..self }
    }
}

pub fn measure_text_prefix_width(
    text: &str,
    upto_byte: usize,
    font_size: f32,
    font_system: &mut FontSystem,
) -> f32 {
    if upto_byte == 0 || text.is_empty() {
        return 0.0;
    }

    let metrics = Metrics::new(font_size, font_size * font::line_height());
    let mut buffer = Buffer::new_empty(metrics);
    buffer.set_size(None, None);
    buffer.set_text(
        text,
        &Attrs::new().family(cosmic_text::Family::SansSerif),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);

    if let Some(run) = buffer.layout_runs().next() {
        for glyph in run.glyphs.iter() {
            if glyph.start >= upto_byte {
                return glyph.x;
            }
        }
        return run.line_w;
    }
    0.0
}

impl Painter<'_> {
    pub fn text_buffer(&mut self, buffer: &Buffer, pos: (f32, f32), color: Color) {
        let s = self.scale();
        let Some(fonts) = self.fonts.as_mut() else {
            return;
        };
        let (x_start, y_start) = pos;
        let offset = self.origin;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let color = glyph.color_opt.map_or(color, |color| {
                    Color::from_rgba8(color.r(), color.g(), color.b(), color.a())
                });
                let physical = glyph.physical((0., 0.), s);

                if let Some(image) = fonts.cache.get_image(fonts.system, physical.cache_key) {
                    let width = image.placement.width;
                    let height = image.placement.height;

                    if width == 0 || height == 0 {
                        continue;
                    }

                    let draw_x = (x_start * s + physical.x as f32 + image.placement.left as f32
                        - offset.0 * s) as i32;
                    let draw_y = ((y_start + run.line_y) * s
                        - image.placement.top as f32
                        - offset.1 * s) as i32;

                    if image.content == SwashContent::Mask
                        && let Some(mut glyph_pixmap) = Pixmap::new(width, height)
                    {
                        let pixels = glyph_pixmap.pixels_mut();

                        for (i, mask_alpha) in image.data.iter().enumerate() {
                            let a_f32 = (*mask_alpha as f32 / 255.0) * color.alpha();
                            let a_u8 = (a_f32 * 255.0) as u8;

                            let pr = (color.red() * a_f32 * 255.0) as u8;
                            let pg = (color.green() * a_f32 * 255.0) as u8;
                            let pb = (color.blue() * a_f32 * 255.0) as u8;

                            pixels[i] = PremultipliedColorU8::from_rgba(pr, pg, pb, a_u8)
                                .unwrap_or(PremultipliedColorU8::TRANSPARENT);
                        }

                        self.canvas.draw_pixmap(
                            draw_x,
                            draw_y,
                            glyph_pixmap.as_ref(),
                            &PixmapPaint::default(),
                            Transform::identity(),
                            None,
                        );
                    }
                }
            }
        }
    }

    pub fn text(&mut self, rect: Rect, text: &str, style: TextStyle) {
        if text.is_empty() {
            return;
        }
        let Some(fonts) = self.fonts.as_mut() else {
            return;
        };

        let (buffer, text_w, text_h) =
            shape_single_line(fonts.system, text, style.size, style.weight, style.style);

        let px = match style.align {
            HAlign::Left => rect.left(),
            HAlign::Center => rect.left() + (rect.width() - text_w) / 2.0,
            HAlign::Right => rect.right() - text_w,
        };
        let py = rect.top() + (rect.height() - text_h) / 2.0;

        self.text_buffer(&buffer, (px, py), style.color);
    }

    pub fn measure(&mut self, text: &str, font_size: f32) -> f32 {
        self.fonts.as_mut().map_or(0.0, |fonts| {
            measure_line_width(text, font_size, fonts.system)
        })
    }

    /// 【界面文字】【样式测量】按 style 的字号、字重与斜体测量 text，返回真实字形宽度，不使用常规字重替代
    pub fn measure_styled(&mut self, text: &str, style: TextStyle) -> f32 {
        self.fonts.as_mut().map_or(0.0, |fonts| {
            shape_single_line(fonts.system, text, style.size, style.weight, style.style).1
        })
    }

    // ── common input field stuff ─────────────────────────────────────
    pub fn input_box(&mut self, rect: Rect, radius: f32) {
        self.fill_rounded(rect, radius, color::field());
    }

    pub(super) fn text_selection(&mut self, rect: Rect, start_x: f32, end_x: f32) {
        let sel_h = rect.height() * crate::config::get().theme.field_text_height;
        let sel_y = rect.top() + (rect.height() - sel_h) / 2.0;
        let Some(sel_rect) = Rect::from_xywh(
            rect.left() + start_x,
            sel_y,
            (end_x - start_x).max(1.0),
            sel_h,
        ) else {
            return;
        };

        let mut paint = Paint::default();
        paint.set_color(color::text_selection().color());
        paint.anti_alias = true;
        let transform = self.transform();
        self.canvas.fill_rect(sel_rect, &paint, transform, None);
    }

    pub(super) fn text_caret(&mut self, rect: Rect, cursor_x: f32) {
        let theme = &crate::config::get().theme;
        let cur_h = rect.height() * theme.field_text_height;
        let cur_y = rect.top() + (rect.height() - cur_h) / 2.0;
        let Some(cur_rect) = Rect::from_xywh(
            (rect.left() + cursor_x).round(),
            cur_y,
            theme.caret_width,
            cur_h,
        ) else {
            return;
        };

        let mut paint = Paint::default();
        paint.set_color(color::caret().color());
        paint.anti_alias = false;
        let transform = self.transform();
        self.canvas.fill_rect(cur_rect, &paint, transform, None);
    }
}
