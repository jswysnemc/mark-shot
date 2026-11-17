//! 【微交互徽标】【共用胶囊组件】为贴图、翻译、上传、缩放及轻提示提供统一的紧凑状态胶囊绘制

use crate::{
    renderer::{Painter, paths::rounded_rect_path, text::TextStyle},
    theme::{Mode, Rgba, toolbar::Palette},
    ui::controls,
};
use cosmic_text::{FontSystem, Style, Weight};
use tiny_skia::{PathBuilder, Rect, Stroke};

pub(crate) const PILL_HEIGHT: f32 = 26.0;
pub(crate) const TWO_LINE_HEIGHT: f32 = 44.0;
const PILL_RADIUS: f32 = 9.0;
const FONT_SIZE: f32 = 11.5;
const SUB_FONT_SIZE: f32 = 10.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BadgeKind {
    Busy,
    Success,
    Notice,
    Error,
    Zoom,
    Neutral,
}

impl BadgeKind {
    /// 【微交互徽标】【前导占位】返回当前类型是否包含左侧语义状态图标，无参数
    pub fn has_indicator(self) -> bool {
        !matches!(self, Self::Zoom | Self::Neutral)
    }
}

/// 【微交互徽标】【尺寸测量】按 fonts、title、detail、kind 与 max_width 计算紧凑胶囊宽高
pub(crate) fn measure(
    fonts: &mut FontSystem,
    title: &str,
    detail: &str,
    kind: BadgeKind,
    max_width: f32,
) -> (f32, f32) {
    crate::fonts::ui_fallback::ensure_text(fonts, title);
    if !detail.is_empty() {
        crate::fonts::ui_fallback::ensure_text(fonts, detail);
    }
    let (_, title_w, _) = crate::renderer::text::shape_single_line(
        fonts,
        title,
        FONT_SIZE,
        Weight::MEDIUM,
        Style::Normal,
    );
    let lead = if kind.has_indicator() { 16.0 } else { 0.0 };
    let pad_x = if matches!(kind, BadgeKind::Zoom) {
        18.0
    } else {
        20.0
    };
    if detail.is_empty() {
        let width = (title_w.ceil() + lead + pad_x)
            .max(44.0)
            .min(max_width.max(36.0));
        (width, PILL_HEIGHT)
    } else {
        let (_, detail_w, _) = crate::renderer::text::shape_single_line(
            fonts,
            detail,
            SUB_FONT_SIZE,
            Weight::NORMAL,
            Style::Normal,
        );
        let content_w = (title_w + lead).max(detail_w);
        let width = (content_w.ceil() + 22.0).max(84.0).min(max_width.max(48.0));
        (width, TWO_LINE_HEIGHT)
    }
}

/// 【微交互徽标】【配色方案】按当前明暗主题与 opacity 返回胶囊底色、边色、主文字色与副文字色
fn colors(opacity: f32) -> (Rgba, Rgba, Rgba, Rgba) {
    let palette = Palette::current();
    let clamp = opacity.clamp(0.0, 1.0);
    let scale_alpha = |c: Rgba| c.with_alpha((f32::from(c.alpha()) * clamp).round() as u8);
    if Mode::current() == Mode::Dark {
        (
            scale_alpha(Rgba(15, 23, 42, 236)),
            scale_alpha(Rgba(148, 163, 184, 68)),
            scale_alpha(Rgba(248, 250, 252, 255)),
            scale_alpha(Rgba(148, 163, 184, 235)),
        )
    } else {
        (
            scale_alpha(Rgba(255, 255, 255, 244)),
            scale_alpha(palette.panel_border.with_alpha(44)),
            scale_alpha(palette.strong_text),
            scale_alpha(palette.muted_text),
        )
    }
}

/// 【微交互徽标】【状态点绘制】在 (cx, cy) 处绘制对应语义类型的精细矢量标记，无返回值
fn draw_indicator(p: &mut Painter<'_>, kind: BadgeKind, cx: f32, cy: f32, opacity: f32) {
    let alpha = (255.0 * opacity.clamp(0.0, 1.0)).round() as u8;
    match kind {
        BadgeKind::Busy => {
            let halo = Rgba(20, 184, 166, (68.0 * opacity.clamp(0.0, 1.0)).round() as u8);
            let core = Rgba(20, 184, 166, alpha);
            p.fill_circle(cx, cy, 4.6, halo);
            p.fill_circle(cx, cy, 2.5, core);
        }
        BadgeKind::Success => {
            let fill = Rgba(16, 185, 129, alpha);
            p.fill_circle(cx, cy, 4.4, fill);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 2.0, cy + 0.1);
            pb.line_to(cx - 0.5, cy + 1.6);
            pb.line_to(cx + 2.2, cy - 1.4);
            if let Some(path) = pb.finish() {
                p.stroke_path(
                    &path,
                    Rgba(255, 255, 255, alpha),
                    &Stroke {
                        width: 1.25,
                        line_cap: tiny_skia::LineCap::Round,
                        line_join: tiny_skia::LineJoin::Round,
                        ..Default::default()
                    },
                );
            }
        }
        BadgeKind::Notice => {
            p.fill_circle(cx, cy, 3.2, Rgba(245, 158, 11, alpha));
        }
        BadgeKind::Error => {
            p.fill_circle(cx, cy, 3.4, Rgba(239, 68, 68, alpha));
        }
        BadgeKind::Zoom | BadgeKind::Neutral => {}
    }
}

/// 【微交互徽标】【胶囊绘制】在 p 的 rect 范围内绘制带语义指示器的紧凑圆角状态胶囊，无返回值
pub(crate) fn draw(
    p: &mut Painter<'_>,
    rect: Rect,
    kind: BadgeKind,
    title: &str,
    detail: &str,
    opacity: f32,
) {
    if opacity <= 0.0 || rect.width() < 16.0 || rect.height() < 14.0 {
        return;
    }
    p.ensure_ui_text(title);
    if !detail.is_empty() {
        p.ensure_ui_text(detail);
    }

    let (fill, border, ink, sub_ink) = colors(opacity);
    let radius = PILL_RADIUS.min(rect.height() * 0.5);
    // 1. 绘制圆角半透明胶囊底座与一像素内描边
    if let Some(frame) = Rect::from_xywh(
        rect.left() + 0.5,
        rect.top() + 0.5,
        (rect.width() - 1.0).max(1.0),
        (rect.height() - 1.0).max(1.0),
    ) {
        p.fill_rounded(frame, radius, fill);
        if let Some(path) = rounded_rect_path(&frame, radius, true, true, true, true) {
            p.stroke_path(
                &path,
                border,
                &Stroke {
                    width: 1.0,
                    ..Default::default()
                },
            );
        }
    }

    // 2. 绘制左侧语义状态点并排布单行或双行自适应截断文本
    let has_dot = kind.has_indicator() && rect.width() >= 54.0;
    if detail.is_empty() || rect.height() < 36.0 {
        if has_dot {
            let dot_x = rect.left() + 12.5;
            let dot_y = rect.top() + rect.height() * 0.5;
            draw_indicator(p, kind, dot_x, dot_y, opacity);
            if let Some(text_rect) = Rect::from_xywh(
                rect.left() + 22.0,
                rect.top(),
                (rect.width() - 31.0).max(1.0),
                rect.height(),
            ) {
                let fitted = controls::fit_label(p, title, FONT_SIZE, text_rect.width());
                p.text(
                    text_rect,
                    &fitted,
                    TextStyle::new(FONT_SIZE, ink.color()).weight(Weight::MEDIUM),
                );
            }
        } else {
            let fitted = controls::fit_label(p, title, FONT_SIZE, (rect.width() - 12.0).max(1.0));
            let weight = if kind == BadgeKind::Zoom {
                Weight::SEMIBOLD
            } else {
                Weight::MEDIUM
            };
            p.text(
                rect,
                &fitted,
                TextStyle::new(FONT_SIZE, ink.color())
                    .weight(weight)
                    .center(),
            );
        }
    } else {
        let title_y = rect.top() + 4.0;
        if has_dot {
            draw_indicator(p, kind, rect.left() + 12.5, title_y + 9.5, opacity);
        }
        let title_left = if has_dot {
            rect.left() + 22.0
        } else {
            rect.left() + 10.0
        };
        if let Some(title_rect) = Rect::from_xywh(
            title_left,
            title_y,
            (rect.right() - 10.0 - title_left).max(1.0),
            19.0,
        ) {
            let fitted = controls::fit_label(p, title, FONT_SIZE, title_rect.width());
            p.text(
                title_rect,
                &fitted,
                TextStyle::new(FONT_SIZE, ink.color()).weight(Weight::MEDIUM),
            );
        }
        if let Some(sub_rect) = Rect::from_xywh(
            rect.left() + 10.0,
            rect.top() + 22.0,
            (rect.width() - 20.0).max(1.0),
            17.0,
        ) {
            let fitted = controls::fit_label(p, detail, SUB_FONT_SIZE, sub_rect.width());
            p.text(
                sub_rect,
                &fitted,
                TextStyle::new(SUB_FONT_SIZE, sub_ink.color()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::SwashCache;
    use tiny_skia::Pixmap;

    /// 【微交互徽标测试】【紧凑尺寸与透明外角】验证缩放倍率与忙碌胶囊宽度自适应且四角外部保持透明
    #[test]
    fn micro_badge_measures_compactly_and_keeps_corners_transparent() {
        let mut fonts = crate::fonts::bundled();
        let mut cache = SwashCache::new();
        let (zoom_w, zoom_h) = measure(&mut fonts, "125%", "", BadgeKind::Zoom, 240.0);
        assert_eq!(zoom_h, PILL_HEIGHT);
        assert!((44.0..96.0).contains(&zoom_w));

        let mut canvas = Pixmap::new(120, 48).unwrap();
        let mut painter = Painter::new(&mut canvas).with_text(&mut fonts, &mut cache);
        let rect = Rect::from_xywh(10.0, 10.0, zoom_w, zoom_h).unwrap();
        draw(&mut painter, rect, BadgeKind::Zoom, "125%", "", 1.0);

        assert_eq!(canvas.pixel(10, 10).unwrap().alpha(), 0);
        assert!(
            canvas
                .pixel((10.0 + zoom_w * 0.5) as u32, 23)
                .unwrap()
                .alpha()
                > 200
        );
    }
}
