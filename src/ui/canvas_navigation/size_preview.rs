//! 【滚轮提示】【窗口绘制】原版颜色示意、热点十字和可用尺寸轮廓，不参与原图导出

use crate::{
    features::wheel_preview::model::{Kind, Sample},
    renderer::{Painter, text::TextStyle},
    theme::{Mode, Rgba},
};
use cosmic_text::Weight;
use tiny_skia::{PathBuilder, Rect, Stroke};

#[cfg(test)]
mod tests;

/// 【滚轮提示】【热点轮廓】按painter、point和sample绘制固定十字及真实窗口尺寸轮廓，无返回值
fn footprint(painter: &mut Painter<'_>, point: (f32, f32), sample: Sample) {
    let center = (point.0.round() + 0.5, point.1.round() + 0.5);
    let mut path = PathBuilder::new();
    if let Some(extent) = sample.footprint() {
        let rect = Rect::from_xywh(
            center.0 - extent / 2.0,
            center.1 - extent / 2.0,
            extent,
            extent,
        )
        .unwrap();
        if sample.kind == Kind::Mosaic {
            path.push_rect(rect);
        } else {
            path.push_oval(rect);
        }
    }
    for (start, end) in [
        ((-9.0, 0.0), (-3.0, 0.0)),
        ((3.0, 0.0), (9.0, 0.0)),
        ((0.0, -9.0), (0.0, -3.0)),
        ((0.0, 3.0), (0.0, 9.0)),
    ] {
        path.move_to(center.0 + start.0, center.1 + start.1);
        path.line_to(center.0 + end.0, center.1 + end.1);
    }
    if let Some(path) = path.finish() {
        for (width, ink) in [
            (3.0, Rgba(17, 24, 39, 235)),
            (1.0, Rgba(243, 244, 246, 255)),
        ] {
            painter.stroke_path(
                &path,
                ink,
                &Stroke {
                    width,
                    line_cap: tiny_skia::LineCap::Round,
                    ..Default::default()
                },
            );
        }
    }
}

/// 【滚轮提示】【颜色样本】按painter、slot、sample与border绘制原版工具示意，无返回值
fn color_sample(painter: &mut Painter<'_>, slot: Rect, sample: Sample, border: Rgba) {
    if sample.kind == Kind::Text {
        painter.text(
            slot,
            "T",
            TextStyle::new(13.0, sample.color)
                .center()
                .weight(Weight::SEMIBOLD),
        );
        return;
    }
    let extent = (sample.value * sample.image_scale).clamp(2.0, 12.0);
    let cx = slot.left() + slot.width() / 2.0;
    let cy = slot.top() + slot.height() / 2.0;
    let rect = match sample.kind {
        Kind::Mosaic => {
            Rect::from_xywh(cx - extent / 2.0, cy - extent / 2.0, extent, extent).unwrap()
        }
        Kind::Number => Rect::from_xywh(cx - 6.0, cy - 6.0, 12.0, 12.0).unwrap(),
        _ => Rect::from_xywh(slot.left(), cy - extent / 2.0, slot.width(), extent).unwrap(),
    };
    let mut builder = PathBuilder::new();
    let path = if sample.kind == Kind::Number {
        builder.push_oval(rect);
        builder.finish()
    } else if sample.kind == Kind::Mosaic {
        builder.push_rect(rect);
        builder.finish()
    } else {
        crate::renderer::rounded_rect_path(&rect, (extent / 2.0).min(3.0), true, true, true, true)
    };
    if let Some(path) = path {
        painter.fill_path(&path, sample.color);
        painter.stroke_path(
            &path,
            border,
            &Stroke {
                width: 0.75,
                ..Default::default()
            },
        );
    }
}

/// 【滚轮提示】【完整气泡】按painter、rect、point、text和sample绘制完整原版提示，无返回值，调用方负责固定网格
pub(crate) fn draw(
    painter: &mut Painter<'_>,
    rect: Rect,
    point: (f32, f32),
    text: &str,
    sample: Sample,
) {
    footprint(painter, point, sample);
    let palette = crate::theme::toolbar::Palette::current();
    let (fill, border, ink, muted) = if Mode::current() == Mode::Dark {
        (
            Rgba(17, 24, 39, 242),
            Rgba(148, 163, 184, 90),
            Rgba(243, 244, 246, 255),
            Rgba(226, 232, 240, 150),
        )
    } else {
        (
            palette.panel_fill,
            palette.panel_border,
            palette.strong_text,
            palette.muted_text,
        )
    };
    painter.fill_rounded(rect, 6.0, fill);
    if let Some(path) = crate::renderer::rounded_rect_path(&rect, 6.0, true, true, true, true) {
        painter.stroke_path(
            &path,
            border,
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
        );
    }
    color_sample(
        painter,
        Rect::from_xywh(
            rect.left() + 9.0,
            rect.top() + rect.height() / 2.0 - 8.0,
            16.0,
            16.0,
        )
        .unwrap(),
        sample,
        muted,
    );
    painter.text(
        Rect::from_xywh(
            rect.left() + 33.0,
            rect.top(),
            rect.width() - 43.0,
            rect.height(),
        )
        .unwrap(),
        text,
        TextStyle::new(13.0, ink).weight(Weight::SEMIBOLD),
    );
}
