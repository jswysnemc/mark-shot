//! 【尺子】【简约箭头】双头箭头居中显示测量宽度，短跨度自动将数值抬至箭头上方避免重叠

use tiny_skia::{Color, LineCap, LineJoin, PathBuilder, Rect, Stroke};

use super::geometry::{points_bounds, rect_center, rotated};
use super::{Measurement, Point, RulerGeometry};
use crate::editor::canvas_navigation::transform::ViewTransform;
use crate::features::annotation_style::window::WindowUnits;
use crate::renderer::{Painter, text::HAlign};

#[derive(Clone, Copy, Debug)]
struct SimpleMetrics {
    bounds: Rect,
    center: Point,
    rotation: f32,
    horizontal: bool,
    span_canvas: f32,
    span_pixels: u64,
    unit: f32,
    font_pixels: f32,
    head_len: f32,
    head_half: f32,
    text_width: f32,
    text_height: f32,
    inline_text: bool,
    text_rect: Rect,
    window: WindowUnits,
}

/// 【尺子】【简约度量】按 measurement、width、window 和可选 projection 计算箭头几何、字号与文字位置，非长度尺返回空值
fn metrics(
    measurement: Measurement,
    width: f32,
    window: WindowUnits,
    projection: Option<ViewTransform>,
) -> Option<SimpleMetrics> {
    let RulerGeometry::Length {
        start,
        end,
        rotation,
    } = measurement.geometry
    else {
        return None;
    };
    let bounds = measurement.unrotated_bounds()?;
    let center = rect_center(bounds);
    let raw_width = (end.0 - start.0).abs();
    let raw_height = (end.1 - start.1).abs();
    let horizontal = raw_width >= raw_height;
    let span_canvas = if horizontal { raw_width } else { raw_height };
    let span_pixels = (span_canvas * measurement.pixel_scale).round() as u64;

    // 1. 按与标准尺一致的样式单位换算字号与箭头尺寸，导出与窗口缩放保持比例稳定
    let coordinate_scale = projection.map_or(1.0, |view| view.scale);
    let unit64 = (1.0 + (f64::from(width * measurement.pixel_scale) - 3.0) / 9.0).clamp(0.75, 3.5)
        * measurement.style_scale
        * coordinate_scale;
    let unit = (unit64 / coordinate_scale) as f32;
    let paint_per_window = if projection.is_some() {
        1.0
    } else {
        f64::from(window.canvas(1.0))
    };
    let font_pixels = (13.0 * unit64 / paint_per_window).round().max(8.0) as f32;

    let label = format_span(span_pixels);
    let measured = crate::fonts::ruler::measure(&label, font_pixels);
    let text_width = window.canvas(measured.width);
    let text_height = window.canvas(measured.height);

    let head_len = (10.0 * unit).min(span_canvas * 0.42).max(3.5 * unit);
    let head_half = (head_len * 0.46).max(2.5 * unit);
    let gap_pad = if horizontal { 6.0 * unit } else { 4.0 * unit };
    let needed = if horizontal {
        text_width + gap_pad * 2.0
    } else {
        text_height + gap_pad * 2.0
    };
    let available = span_canvas - head_len * 2.0;
    let inline_text = available >= needed + 6.0 * unit;

    // 2. 跨度足够时文字置于双头箭头正中并断开主轴；跨度过短时抬至箭头外侧避免遮挡箭头
    let text_center = if inline_text {
        center
    } else if horizontal {
        (
            center.0,
            center.1 - head_half - text_height / 2.0 - 4.0 * unit,
        )
    } else {
        (
            center.0 + head_half + text_width / 2.0 + 4.0 * unit,
            center.1,
        )
    };
    let text_rect = Rect::from_xywh(
        text_center.0 - text_width / 2.0,
        text_center.1 - text_height / 2.0,
        text_width.max(1.0),
        text_height.max(1.0),
    )?;

    Some(SimpleMetrics {
        bounds,
        center,
        rotation,
        horizontal,
        span_canvas,
        span_pixels,
        unit,
        font_pixels,
        head_len,
        head_half,
        text_width,
        text_height,
        inline_text,
        text_rect,
        window,
    })
}

/// 【尺子】【跨度文案】按原图像素整数 span 返回紧凑像素文本
pub(crate) fn format_span(span: u64) -> String {
    format!("{span}px")
}

/// 【尺子】【主轴端点】返回未旋转局部空间中双头箭头起止点，非长度尺返回空值
pub(crate) fn axis_endpoints(measurement: Measurement) -> Option<(Point, Point)> {
    let RulerGeometry::Length { start, end, .. } = measurement.geometry else {
        return None;
    };
    let bounds = measurement.unrotated_bounds()?;
    let center = rect_center(bounds);
    let horizontal = (end.0 - start.0).abs() >= (end.1 - start.1).abs();
    if horizontal {
        Some(((bounds.left(), center.1), (bounds.right(), center.1)))
    } else {
        Some(((center.0, bounds.top()), (center.0, bounds.bottom())))
    }
}

/// 【尺子】【简约范围】按 measurement、width、window 和 projection 返回含箭头与居中文字的旋转外接矩形
pub(crate) fn bounds(
    measurement: Measurement,
    width: f32,
    window: WindowUnits,
    projection: ViewTransform,
) -> Option<Rect> {
    let proj = (!projection.is_identity()).then_some(projection);
    let window = if proj.is_some() {
        WindowUnits::new(1.0 / projection.scale as f32)
    } else {
        window
    };
    let info = metrics(measurement, width, window, proj)?;
    let (p0, p1) = axis_endpoints(measurement)?;
    let halo = info.unit.max(info.window.canvas(0.8));
    let arrow_pad = info.head_half + info.unit;
    let arrow_rect = Rect::from_ltrb(
        p0.0.min(p1.0) - arrow_pad,
        p0.1.min(p1.1) - arrow_pad,
        p0.0.max(p1.0) + arrow_pad,
        p0.1.max(p1.1) + arrow_pad,
    )?;
    let text_rect = info
        .text_rect
        .outset(
            halo + info.window.canvas(2.0),
            halo + info.window.canvas(2.0),
        )
        .unwrap_or(info.text_rect);
    let combined = arrow_rect.join(&text_rect)?;
    let corners = [
        (combined.left(), combined.top()),
        (combined.right(), combined.top()),
        (combined.right(), combined.bottom()),
        (combined.left(), combined.bottom()),
    ]
    .map(|point| rotated(point, info.center, info.rotation));
    points_bounds(&corners, 1.0 / measurement.pixel_scale)
}

/// 【尺子】【顶部留量】按 measurement、width、window 返回箭头或文字超出正文上边的距离，用于定位旋转把手
pub(crate) fn top_reach(measurement: Measurement, width: f32, window: WindowUnits) -> f32 {
    let Some(info) = metrics(measurement, width, window, None) else {
        return 8.0 * measurement.unit(width);
    };
    let arrow_top = if info.horizontal {
        info.center.1 - info.head_half
    } else {
        info.bounds.top()
    };
    let halo = info.unit.max(window.canvas(0.8));
    let highest = arrow_top.min(info.text_rect.top() - halo);
    let frame_top = super::simple_frame::bounds(measurement, window)
        .map_or(info.bounds.top(), |rect| rect.top());
    (frame_top - highest + 4.0 * info.unit).max(0.0)
}

/// 【尺子】【三角箭头】把以 tip 为尖端、朝 dir 指向的实心三角形加入 path，已按 center 和 rotation 旋转，无返回值
fn push_arrowhead(
    path: &mut PathBuilder,
    tip: Point,
    dir: Point,
    head_len: f32,
    head_half: f32,
    center: Point,
    rotation: f32,
) {
    let perp = (-dir.1, dir.0);
    let base = (tip.0 - dir.0 * head_len, tip.1 - dir.1 * head_len);
    let wing_a = (base.0 + perp.0 * head_half, base.1 + perp.1 * head_half);
    let wing_b = (base.0 - perp.0 * head_half, base.1 - perp.1 * head_half);
    let p_tip = rotated(tip, center, rotation);
    let p_a = rotated(wing_a, center, rotation);
    let p_b = rotated(wing_b, center, rotation);
    path.move_to(p_tip.0, p_tip.1);
    path.line_to(p_a.0, p_a.1);
    path.line_to(p_b.0, p_b.1);
    path.close();
}

/// 【尺子】【简约绘制】将双头箭头与中间宽度文本按 measurement、width、color 绘入 painter，无返回值
pub fn draw(painter: &mut Painter<'_>, measurement: Measurement, width: f32, color: Color) {
    if !measurement.measures() || color.alpha() <= 0.0 {
        return;
    }
    let projection = painter.ruler_projection();
    let proj = (!projection.is_identity()).then_some(projection);
    let window = if proj.is_some() {
        WindowUnits::new(1.0 / projection.scale as f32)
    } else {
        painter.window_units()
    };
    let Some(info) = metrics(measurement, width, window, proj) else {
        return;
    };
    let Some((p0, p1)) = axis_endpoints(measurement) else {
        return;
    };
    let dir = if info.horizontal {
        (1.0, 0.0)
    } else {
        (0.0, 1.0)
    };

    // 1. 绘制两端实心箭头
    let mut heads = PathBuilder::new();
    push_arrowhead(
        &mut heads,
        p0,
        (-dir.0, -dir.1),
        info.head_len,
        info.head_half,
        info.center,
        info.rotation,
    );
    push_arrowhead(
        &mut heads,
        p1,
        dir,
        info.head_len,
        info.head_half,
        info.center,
        info.rotation,
    );
    if let Some(heads) = heads.finish() {
        painter.fill_path(&heads, color);
    }

    // 2. 绘制主轴线段；文字居中时在文字两侧断开，避免线条穿过数字
    let inset = info.head_len * 0.85;
    let shaft_start = (p0.0 + dir.0 * inset, p0.1 + dir.1 * inset);
    let shaft_end = (p1.0 - dir.0 * inset, p1.1 - dir.1 * inset);
    let mut shaft = PathBuilder::new();
    if info.inline_text {
        let half_gap = if info.horizontal {
            info.text_width / 2.0 + 6.0 * info.unit
        } else {
            info.text_height / 2.0 + 4.0 * info.unit
        };
        let gap_start = (
            info.center.0 - dir.0 * half_gap,
            info.center.1 - dir.1 * half_gap,
        );
        let gap_end = (
            info.center.0 + dir.0 * half_gap,
            info.center.1 + dir.1 * half_gap,
        );
        let s0 = rotated(shaft_start, info.center, info.rotation);
        let g0 = rotated(gap_start, info.center, info.rotation);
        shaft.move_to(s0.0, s0.1);
        shaft.line_to(g0.0, g0.1);
        let g1 = rotated(gap_end, info.center, info.rotation);
        let s1 = rotated(shaft_end, info.center, info.rotation);
        shaft.move_to(g1.0, g1.1);
        shaft.line_to(s1.0, s1.1);
    } else if info.span_canvas > inset * 2.0 {
        let s0 = rotated(shaft_start, info.center, info.rotation);
        let s1 = rotated(shaft_end, info.center, info.rotation);
        shaft.move_to(s0.0, s0.1);
        shaft.line_to(s1.0, s1.1);
    }
    if let Some(shaft) = shaft.finish() {
        painter.stroke_path(
            &shaft,
            color,
            &Stroke {
                width: info.window.stroke(2.0 * info.unit, 1.25, None),
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            },
        );
    }

    // 3. 在箭头中间或外侧绘制带对比描边的宽度读数
    let label = format_span(info.span_pixels);
    crate::renderer::ruler_text::halo_number(
        painter,
        info.text_rect,
        &label,
        color,
        info.rotation,
        info.center,
        crate::renderer::ruler_text::Style {
            pixels: info.font_pixels,
            unit: info.unit,
            window: info.window,
            align: HAlign::Center,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::ruler::{RulerMode, RulerStyle};
    use cosmic_text::SwashCache;
    use tiny_skia::Pixmap;

    /// 【尺子测试】【简约绘制与留空】验证双头箭头在长跨度居中绘制宽度且跳过浮动详情卡片，短跨度将数字抬至外侧
    #[test]
    fn simple_arrow_renders_inline_width_and_skips_floating_info_card() {
        let mut measurement = Measurement::new(RulerMode::Length, (40.0, 100.0), 1.0)
            .unwrap()
            .with_style(RulerStyle::Simple);
        measurement.geometry = RulerGeometry::Length {
            start: (40.0, 100.0),
            end: (280.0, 100.0),
            rotation: 0.0,
        };
        let info = metrics(measurement, 3.0, WindowUnits::default(), None).unwrap();
        assert!(info.inline_text);
        assert_eq!(info.span_pixels, 240);

        let mut fonts = crate::fonts::bundled();
        let mut cache = SwashCache::new();
        let labels = crate::features::ruler::labels::prepare_all(
            std::iter::once((measurement, 3.0)),
            Rect::from_xywh(0.0, 0.0, 320.0, 200.0).unwrap(),
            &mut fonts,
        );
        assert!(labels.is_empty());

        let mut canvas = Pixmap::new(320, 200).unwrap();
        {
            let mut painter = Painter::new(&mut canvas).with_text(&mut fonts, &mut cache);
            draw(
                &mut painter,
                measurement,
                3.0,
                Color::from_rgba8(255, 77, 77, 255),
            );
        }
        // 1. 左箭头尖端附近、主轴和居中数字区域均应有绘制像素
        assert!(canvas.pixel(43, 100).unwrap().alpha() > 0);
        assert!(canvas.pixel(90, 100).unwrap().alpha() > 0);
        assert!(canvas.pixel(277, 100).unwrap().alpha() > 0);
        let canvas_ref = &canvas;
        let center_ink = (140..180)
            .flat_map(|x| (90..110).map(move |y| canvas_ref.pixel(x, y).unwrap().alpha()))
            .any(|alpha| alpha > 0);
        assert!(center_ink);

        // 2. 短跨度自动抬高文字避免与双箭头重叠
        measurement.geometry = RulerGeometry::Length {
            start: (100.0, 100.0),
            end: (120.0, 100.0),
            rotation: 0.0,
        };
        let short = metrics(measurement, 3.0, WindowUnits::default(), None).unwrap();
        assert!(!short.inline_text);
        assert!(short.text_rect.bottom() < 100.0);
    }
}
