//! 【尺子】【说明位置】角度说明优先，长度说明按实际刻度范围和完整标签避让

use super::{
    RulerGeometry,
    label_model::{InfoLabel, LabelRequest},
    layout_rect::ExactRect,
    length_label::{self, Frame},
    tick_layout::Layout,
};
use tiny_skia::Rect;

/// 【尺子】【浮动范围平移】按rect、bounds和gap依次约束右、左、下、上，不改变原版完整标签尺寸
fn floating(mut rect: ExactRect, bounds: ExactRect, gap: f64) -> ExactRect {
    if rect.right() > bounds.right() - gap {
        rect.left = bounds.right() - gap - rect.width;
    }
    if rect.left < bounds.left + gap {
        rect.left = bounds.left + gap;
    }
    if rect.bottom() > bounds.bottom() - gap {
        rect.top = bounds.bottom() - gap - rect.height;
    }
    if rect.top < bounds.top + gap {
        rect.top = bounds.top + gap;
    }
    rect
}

/// 【尺子】【公共说明平移】按rect、bounds和gap返回完整标签范围，仅平移，无效矩形沿用输入
pub(super) fn clamped(rect: Rect, bounds: Rect, gap: f32) -> Rect {
    floating(
        ExactRect::from_rect(rect),
        ExactRect::from_rect(bounds),
        f64::from(gap),
    )
    .rect()
    .unwrap_or(rect)
}

/// 【尺子】【窗口角度说明】按request和bounds返回角平分线方向的完整说明，三点由当前窗口投影计算
fn angle(request: &LabelRequest, bounds: ExactRect) -> Option<ExactRect> {
    let RulerGeometry::Angle { points, count: 3 } = request.measurement.geometry else {
        return None;
    };
    let points = points.map(|p| {
        request
            .projection
            .to_screen((f64::from(p.0), f64::from(p.1)))
    });
    let a = (points[1].0 - points[0].0, points[1].1 - points[0].1);
    let b = (points[2].0 - points[0].0, points[2].1 - points[0].1);
    let shorter = a.0.hypot(a.1).min(b.0.hypot(b.1));
    let size_scale = (1.0
        + (f64::from(request.width * request.measurement.pixel_scale) - 3.0) / 9.0)
        .clamp(0.75, 3.5);
    let radius = (shorter * 0.4 * size_scale)
        .clamp(14.0 * request.paint_unit, 44.0 * request.paint_unit)
        .min(shorter * 0.8);
    let start = (-a.1).atan2(a.0).to_degrees();
    let end = (-b.1).atan2(b.0).to_degrees();
    let delta = end - start;
    let sweep = delta - 360.0 * (delta / 360.0).round_ties_even();
    let bisector = (start + sweep / 2.0).to_radians();
    let distance =
        radius + 10.0 * request.paint_unit + request.paint_size.0.hypot(request.paint_size.1) / 2.0;
    let rect = ExactRect {
        left: points[0].0 + bisector.cos() * distance - request.paint_size.0 / 2.0,
        top: points[0].1 - bisector.sin() * distance - request.paint_size.1 / 2.0,
        width: request.paint_size.0,
        height: request.paint_size.1,
    };
    Some(floating(rect, bounds, 8.0 * request.paint_unit))
}

/// 【尺子】【长度刻度范围】按测量、笔宽、允许范围和窗口投影返回尺身与刻度条带的外接矩形
pub(super) fn scale(
    measurement: super::Measurement,
    width: f32,
    bounds: Rect,
    projection: crate::editor::canvas_navigation::transform::ViewTransform,
) -> Option<ExactRect> {
    Some(
        Layout::in_projection(measurement, width, Some(bounds), projection)?
            .projected_scale_bounds(),
    )
}

/// 【尺子】【稳定说明】按 request、窗口 rect 和选定的 lines 返回画布坐标中的标签，保留窗口字号
fn label(request: &LabelRequest, rect: ExactRect, lines: &[String]) -> Option<InfoLabel> {
    Some(InfoLabel {
        rect: rect.projected(Some(request.projection), true).rect()?,
        unit: request.unit,
        lines: lines.to_vec(),
        font_size: request.font_size,
    })
}

/// 【尺子】【全部说明布局】按 requests、bounds 和未标注尺身的 obstacles 返回角度优先的标签列表
pub(super) fn layout(
    requests: &[LabelRequest],
    bounds: Rect,
    obstacles: impl Iterator<Item = ExactRect>,
) -> Vec<InfoLabel> {
    let mut result = Vec::with_capacity(requests.len());
    let mut occupied = Vec::new();
    // 1. 【尺子】【角度优先】只登记角度说明，角度正文不挤占长度说明候选区域
    for request in requests
        .iter()
        .filter(|request| matches!(request.measurement.geometry, RulerGeometry::Angle { .. }))
    {
        let allowed = ExactRect::from_rect(bounds).projected(Some(request.projection), false);
        if let Some(rect) = angle(request, allowed) {
            occupied.push(rect);
            if let Some(label) = label(request, rect, &request.lines) {
                result.push(label);
            }
        }
    }
    let lengths: Vec<_> = requests
        .iter()
        .filter_map(|request| Frame::new(request, bounds).map(|frame| (request, frame)))
        .collect();
    if lengths.is_empty() {
        return result;
    }
    occupied.extend(obstacles);
    // 2. 【尺子】【刻度避让】仅排除当前尺身，其他尺身和先前放置的标签始终参与避让
    for (index, (request, frame)) in lengths.iter().enumerate() {
        let allowed = ExactRect::from_rect(bounds).projected(Some(request.projection), false);
        let others = lengths[..index]
            .iter()
            .chain(&lengths[index + 1..])
            .map(|(_, other)| other.scale);
        if let Some(placement) = length_label::place(
            request,
            frame,
            allowed,
            occupied.iter().copied().chain(others),
        ) {
            occupied.push(placement.rect);
            if let Some(label) = label(request, placement.rect, placement.lines) {
                result.push(label);
            }
        }
    }
    result
}
