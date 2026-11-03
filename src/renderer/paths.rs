use crate::theme::{Rgba, color, stroke};

/// 4/3*(sqrt(2)-1).
pub const KAPPA: f32 = 0.552_284_8;

use tiny_skia::{Color, PathBuilder, Pixmap, Rect, Stroke, Transform};

use super::Painter;

pub fn rounded_rect_path(
    rect: &Rect,
    r: f32,
    top_left: bool,
    top_right: bool,
    bottom_right: bool,
    bottom_left: bool,
) -> Option<tiny_skia::Path> {
    let r = r.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let k = 1.0 - KAPPA;

    let (l, t, ri, b) = (rect.left(), rect.top(), rect.right(), rect.bottom());

    let r_tl = if top_left { r } else { 0.0 };
    let r_tr = if top_right { r } else { 0.0 };
    let r_br = if bottom_right { r } else { 0.0 };
    let r_bl = if bottom_left { r } else { 0.0 };

    let mut pb = PathBuilder::new();

    pb.move_to(l + r_tl, t);
    pb.line_to(ri - r_tr, t);

    if top_right {
        pb.cubic_to(ri - r_tr * k, t, ri, t + r_tr * k, ri, t + r_tr);
    } else {
        pb.line_to(ri, t);
    }

    pb.line_to(ri, b - r_br);

    if bottom_right {
        pb.cubic_to(ri, b - r_br * k, ri - r_br * k, b, ri - r_br, b);
    } else {
        pb.line_to(ri, b);
    }

    pb.line_to(l + r_bl, b);

    if bottom_left {
        pb.cubic_to(l + r_bl * k, b, l, b - r_bl * k, l, b - r_bl);
    } else {
        pb.line_to(l, b);
    }

    pb.line_to(l, t + r_tl);

    if top_left {
        pb.cubic_to(l, t + r_tl * k, l + r_tl * k, t, l + r_tl, t);
    } else {
        pb.line_to(l, t);
    }

    pb.close();
    pb.finish()
}

pub fn oval_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(cx, cy - ry);
    pb.cubic_to(
        cx + rx * KAPPA,
        cy - ry,
        cx + rx,
        cy - ry * KAPPA,
        cx + rx,
        cy,
    );
    pb.cubic_to(
        cx + rx,
        cy + ry * KAPPA,
        cx + rx * KAPPA,
        cy + ry,
        cx,
        cy + ry,
    );
    pb.cubic_to(
        cx - rx * KAPPA,
        cy + ry,
        cx - rx,
        cy + ry * KAPPA,
        cx - rx,
        cy,
    );
    pb.cubic_to(
        cx - rx,
        cy - ry * KAPPA,
        cx - rx * KAPPA,
        cy - ry,
        cx,
        cy - ry,
    );
    pb.close();
    pb.finish()
}

pub fn normalized_rect(start: (f32, f32), end: (f32, f32)) -> Option<Rect> {
    Rect::from_ltrb(
        start.0.min(end.0),
        start.1.min(end.1),
        start.0.max(end.0),
        start.1.max(end.1),
    )
}

pub fn scale_rect(rect: &Rect, scale: f32) -> Option<Rect> {
    Rect::from_ltrb(
        rect.left() * scale,
        rect.top() * scale,
        rect.right() * scale,
        rect.bottom() * scale,
    )
}

pub fn rect_bounds(rect: &Rect, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let x0 = rect.left().floor().max(0.0) as i32;
    let y0 = rect.top().floor().max(0.0) as i32;
    let x1 = rect.right().ceil().min(width as f32) as i32;
    let y1 = rect.bottom().ceil().min(height as f32) as i32;

    if x1 <= x0 || y1 <= y0 {
        return None;
    }

    Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
}

pub fn tint_pixmap(pixmap: &mut tiny_skia::Pixmap, color: Rgba) {
    let Rgba(red, green, blue, alpha) = color;
    for pixel in pixmap.pixels_mut() {
        if pixel.alpha() == 0 {
            continue;
        }
        let a = (pixel.alpha() as u16 * alpha as u16 / 255) as u8;
        let r = (red as u16 * a as u16 / 255) as u8;
        let g = (green as u16 * a as u16 / 255) as u8;
        let b = (blue as u16 * a as u16 / 255) as u8;
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a).unwrap();
    }
}

/// Perceived luminance (ITU-R BT.601)
pub fn luminance(color: Color) -> f32 {
    0.299 * color.red() + 0.587 * color.green() + 0.114 * color.blue()
}

/// Above this a panel is light enough to need a dark border.
const BORDER_FLIP: f32 = 0.5;

pub fn panel_border_color(bg: Color) -> Color {
    if luminance(bg) > BORDER_FLIP {
        color::border_on_light().color()
    } else {
        color::border().color()
    }
}

impl Painter<'_> {
    pub fn progress_bar(&mut self, x: f32, y: f32, w: f32, percent: u8) {
        let h = stroke::progress();
        let filled = match percent {
            0 => 0.0,
            _ => (w * f32::from(percent.min(100)) / 100.0).max(h),
        };
        for (width, fill) in [(w, color::track()), (filled, color::active())] {
            if width <= 0.0 {
                continue;
            }
            if let Some(rect) = Rect::from_xywh(x, y, width, h) {
                self.fill_rounded(rect, h / 2.0, fill);
            }
        }
    }

    pub fn item_border(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        stroke_width: f32,
        is_hovered: bool,
        is_selected: bool,
    ) {
        let color = if is_selected {
            color::active().color()
        } else if is_hovered {
            color::hover().color()
        } else {
            panel_border_color(color::panel().color())
        };
        self.inset_border(x, y, w, h, radius, stroke_width, color);
    }

    pub fn panel_border(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, opacity: f32) {
        let mut color = panel_border_color(color::panel().color());
        color.set_alpha(color.alpha() * opacity);
        self.inset_border(x, y, w, h, radius, stroke::border(), color);
    }

    fn inset_border(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        stroke_width: f32,
        color: Color,
    ) {
        let inset = stroke_width / 2.0;
        let Some(inset_rect) = Rect::from_xywh(
            x + inset,
            y + inset,
            (w - stroke_width).max(0.1),
            (h - stroke_width).max(0.1),
        ) else {
            return;
        };

        let r = (radius - inset)
            .max(0.0)
            .min(inset_rect.width() / 2.0)
            .min(inset_rect.height() / 2.0);
        let Some(path) = rounded_rect_path(&inset_rect, r, true, true, true, true) else {
            return;
        };

        let stroke = Stroke {
            width: stroke_width,
            ..Default::default()
        };
        self.stroke_path(&path, color, &stroke);
    }

    pub fn icon(
        &mut self,
        svg_str: &'static str,
        icon_size: f32,
        x: f32,
        y: f32,
        tint: impl Into<Rgba>,
    ) {
        let Some(rtree) = self.icons.and_then(|icons| icons.get(svg_str)) else {
            return;
        };

        let size = icon_size * self.scale();
        let scale_x = size / rtree.size().width();
        let scale_y = size / rtree.size().height();

        let px_size = size.ceil().max(1.0) as u32;
        let Some(mut icon_pixmap) = Pixmap::new(px_size, px_size) else {
            return;
        };

        resvg::render(
            rtree,
            Transform::from_scale(scale_x, scale_y),
            &mut icon_pixmap.as_mut(),
        );
        tint_pixmap(&mut icon_pixmap, tint.into());

        let (px, py) = self.to_px(x, y);
        self.draw_pixmap(px.round() as i32, py.round() as i32, &icon_pixmap);
    }
}

impl From<usvg::Color> for Rgba {
    fn from(c: usvg::Color) -> Self {
        Rgba(c.red, c.green, c.blue, 255)
    }
}
