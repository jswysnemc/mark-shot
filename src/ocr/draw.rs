// Overlay for the OCR tool: the shaded scan region, a plate behind every
// recognized line, the selection on top, and the progress badge that stands in
// for all of it while recognition is still running.
//
// Everything here is translucent and everything here is transient - it is
// repainted from the dim layer on every pointer move. Two rules keep that cheap
// and keep the washes from stacking up:
//
//   * `clip` is the area the caller has just restored from the dim layer, and
//     everything is painted into a scratch pixmap covering exactly that area,
//     which is then composited once. Shapes keep their true geometry - the
//     pixmap's own bounds do the cutting - and nothing lands outside the damage
//     the tool reported.
//   * the scratch pixmap starts transparent, so the washes composite against
//     each other exactly as they would drawn straight onto the canvas.

use std::collections::HashMap;
use tiny_skia::{FillRule, Paint, Pixmap, Rect, Transform};
use usvg::Tree;

use crate::config::OcrLook;
use crate::ocr::OcrView;
use crate::renderer::paths::{rect_bounds, rounded_rect_path, scale_rect};
use crate::theme::{Rgba, color};
use crate::ui::icons;

fn look() -> &'static OcrLook {
    &crate::config::get().ocr.look
}

pub fn draw_ocr_overlay(
    canvas: &mut Pixmap,
    view: &OcrView,
    offset: (f32, f32),
    clip: Option<&Rect>,
    scale: f32,
) {
    let Some(mut painter) = Painter::new(canvas, offset, clip, scale) else {
        return;
    };
    let look = look();

    // Wash over the whole scanned area. Deliberately paired with the plate below: the
    // shade pushes the screenshot back and the plate lifts the text roughly back to
    // where it started, so the lines read as the foreground without the region ever
    // getting as dark as the overlay dim outside it.
    if let Some(region) = view.region() {
        painter.fill(region, 0.0, look.region_shade);
    }

    // Plate behind a block of text, grown around the block's own bounds so it
    // reads as a box around the text rather than a tight box on it.
    let plates = view
        .block_bounds()
        .filter_map(|bounds| pad(bounds, look.plate_pad_x, look.plate_pad_y))
        .filter_map(|plate| inside(plate, view.region()));
    painter.fill_union(plates, look.plate_radius, look.plate);

    // At least a hairline wide, so an empty span still reads as a caret.
    let spans = (0..view.lines.len()).filter_map(|i| {
        let sel = view.line_selection(i)?;
        let (mut top, mut bottom) = (sel.y.0 - look.plate_pad_y, sel.y.1 + look.plate_pad_y);
        // (cutting the sides sometimes causes cutting a letter, and there is no need in that really)
        if let Some(region) = view.region() {
            top = top.max(region.top());
            bottom = bottom.min(region.bottom());
        }
        Rect::from_ltrb(sel.x.0, top, sel.x.1.max(sel.x.0 + 1.0), bottom)
    });
    painter.fill_union(spans, look.selection_radius, color::text_selection());

    painter.finish(canvas);
}

/// Progress badge, centred in the region being scanned: the tool's own icon with
/// a bar sweeping back and forth across it. Only the badge animates - the shade
/// underneath it is painted once, when the scan starts, and simply survives on
/// the canvas because nothing damages it until results land.
pub fn draw_ocr_scan(
    canvas: &mut Pixmap,
    region: Rect,
    phase: f32,
    offset: (f32, f32),
    icons_cache: &HashMap<&'static str, Tree>,
    clip: Option<&Rect>,
    scale: f32,
) {
    let Some(mut painter) = Painter::new(canvas, offset, clip, scale) else {
        return;
    };
    let look = look();
    let (size, icon) = (look.badge_size, look.badge_icon_size);
    painter.fill(region, 0.0, look.region_shade);

    let badge = scan_badge_rect(region);
    painter.fill(badge, look.badge_radius, look.badge_background.get());

    let left = badge.left() - offset.0;
    let top = badge.top() - offset.1;
    let mut p = painter.shared().with_icons(icons_cache);
    p.panel_border(left, top, size, size, look.badge_radius, 1.0);
    p.icon(
        icons::OCR,
        icon,
        left + (size - icon) / 2.0,
        top + (size - icon) / 2.0,
        look.badge_icon.get().usvg(),
    );

    // Ping-pong, eased at both ends so the bar decelerates into each turn
    // instead of snapping back. `scan_rate` is sweeps per second, counting
    // there and back as two.
    let swing = (phase * look.scan_rate).rem_euclid(2.0);
    let t = if swing > 1.0 { 2.0 - swing } else { swing };
    let t = t * t * (3.0 - 2.0 * t);

    let track = (icon + 6.0).min(size - look.scan_width).max(0.0);
    let x = left + (size - track) / 2.0 + t * track;
    let bar_top = top + (size - track) / 2.0;

    let half = look.scan_width / 2.0;
    if let Some(bar) = Rect::from_ltrb(x - half, bar_top, x + half, bar_top + track) {
        painter.fill_local(bar, half, look.scan.get());
    }

    painter.finish(canvas);
}

/// Where the badge sits, in global coordinates. The tool damages exactly this
/// rectangle each frame, so it has to agree with what is drawn.
pub fn scan_badge_rect(region: Rect) -> Rect {
    let cx = region.left() + region.width() / 2.0;
    let cy = region.top() + region.height() / 2.0;
    let size = look().badge_size;
    Rect::from_xywh(cx - size / 2.0, cy - size / 2.0, size, size).unwrap_or(region)
}

// ── clipped painting ────────────────────────────────────────────────────────

struct Painter {
    buf: Pixmap,
    origin: (f32, f32),
    offset: (f32, f32),
    scale: f32,
}

impl Painter {
    fn new(canvas: &Pixmap, offset: (f32, f32), clip: Option<&Rect>, scale: f32) -> Option<Self> {
        let (w, h) = (canvas.width(), canvas.height());
        let (x, y, cw, ch) = match clip {
            Some(c) => rect_bounds(&scale_rect(c, scale)?, w, h)?,
            None => (0, 0, w, h),
        };
        Some(Self {
            buf: Pixmap::new(cw, ch)?,
            origin: (x as f32, y as f32),
            offset,
            scale,
        })
    }

    fn shared(&mut self) -> crate::renderer::Painter<'_> {
        let s = self.scale;
        crate::renderer::Painter::new(&mut self.buf)
            .with_scale(s)
            .with_origin((self.origin.0 / s, self.origin.1 / s))
    }

    fn buf_rect(&self, rect: Rect) -> Option<Rect> {
        let s = self.scale;
        Rect::from_ltrb(
            rect.left() * s - self.origin.0,
            rect.top() * s - self.origin.1,
            rect.right() * s - self.origin.0,
            rect.bottom() * s - self.origin.1,
        )
    }

    /// Fills every rectangle in one pass
    fn fill_union(&mut self, rects: impl IntoIterator<Item = Rect>, radius: f32, color: Rgba) {
        let mut pb = tiny_skia::PathBuilder::new();
        let mut any = false;
        for rect in rects {
            let Some(rect) = self.to_buf_rect(rect) else {
                continue;
            };
            let sub = if radius > 0.0 {
                rounded_rect_path(&rect, radius * self.scale, true, true, true, true)
            } else {
                Some(tiny_skia::PathBuilder::from_rect(rect))
            };
            if let Some(sub) = sub {
                pb.push_path(&sub);
                any = true;
            }
        }
        if !any {
            return;
        }
        let Some(path) = pb.finish() else { return };

        let mut paint = Paint::default();
        paint.set_color(color.color());
        paint.anti_alias = radius > 0.0;
        self.buf.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    fn to_buf_rect(&self, rect: Rect) -> Option<Rect> {
        let (dx, dy) = self.offset;
        let rect = self.buf_rect(Rect::from_ltrb(
            rect.left() - dx,
            rect.top() - dy,
            rect.right() - dx,
            rect.bottom() - dy,
        )?)?;
        let outside = rect.right() <= 0.0
            || rect.bottom() <= 0.0
            || rect.left() >= self.buf.width() as f32
            || rect.top() >= self.buf.height() as f32;
        (!outside).then_some(rect)
    }

    fn fill(&mut self, rect: Rect, radius: f32, color: Rgba) {
        let Some(local) = Rect::from_ltrb(
            rect.left() - self.offset.0,
            rect.top() - self.offset.1,
            rect.right() - self.offset.0,
            rect.bottom() - self.offset.1,
        ) else {
            return;
        };
        self.fill_local(local, radius, color);
    }

    fn fill_local(&mut self, rect: Rect, radius: f32, color: Rgba) {
        let Some(rect) = self.buf_rect(rect) else {
            return;
        };
        if rect.right() <= 0.0
            || rect.bottom() <= 0.0
            || rect.left() >= self.buf.width() as f32
            || rect.top() >= self.buf.height() as f32
        {
            return;
        }

        let mut paint = Paint::default();
        paint.set_color(color.color());
        paint.anti_alias = radius > 0.0;

        let path = if radius > 0.0 {
            rounded_rect_path(&rect, radius * self.scale, true, true, true, true)
        } else {
            Some(tiny_skia::PathBuilder::from_rect(rect))
        };
        let Some(path) = path else { return };

        self.buf.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    fn finish(self, canvas: &mut Pixmap) {
        canvas.draw_pixmap(
            self.origin.0 as i32,
            self.origin.1 as i32,
            self.buf.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}

fn inside(rect: Rect, region: Option<Rect>) -> Option<Rect> {
    match region {
        Some(region) => rect.intersect(&region),
        None => Some(rect),
    }
}

fn pad(rect: Rect, x: f32, y: f32) -> Option<Rect> {
    Rect::from_ltrb(
        rect.left() - x,
        rect.top() - y,
        rect.right() + x,
        rect.bottom() + y,
    )
}
