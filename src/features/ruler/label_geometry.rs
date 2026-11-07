//! 【尺子】【说明位置】角度说明优先，长度说明按实际刻度范围和完整标签避让

use super::{
    RulerGeometry,
    label_model::{InfoLabel, LabelRequest},
    layout_rect::ExactRect,
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

/// 【尺子】【长度刻度范围】按request和bounds返回当前窗口中的正文与实际数字条带外接矩形
fn scale(request: &LabelRequest, bounds: Rect) -> Option<ExactRect> {
    Some(
        Layout::in_projection(
            request.measurement,
            request.width,
            Some(bounds),
            request.projection,
        )?
        .projected_scale_bounds(),
    )
}

/// 【尺子】【长度说明候选】按request、scale和bounds依次选择下、上、右、左，occupied提供已登记的刻度和说明
fn length(
    request: &LabelRequest,
    scale: ExactRect,
    bounds: ExactRect,
    occupied: &[ExactRect],
) -> Option<ExactRect> {
    let RulerGeometry::Length {
        start,
        end,
        rotation,
    } = request.measurement.geometry
    else {
        return None;
    };
    let body = ExactRect {
        left: f64::from(start.0.min(end.0)),
        top: f64::from(start.1.min(end.1)),
        width: f64::from((end.0 - start.0).abs()),
        height: f64::from((end.1 - start.1).abs()),
    }
    .projected(Some(request.projection), false);
    let body = body.rotated(
        (body.left + body.width / 2.0, body.top + body.height / 2.0),
        rotation,
    );
    let gap = 8.0 * request.paint_unit;
    let size = request.paint_size;
    let positions = [
        (body.left, scale.bottom() + gap),
        (body.left, scale.top - size.1 - gap),
        (scale.right() + gap, body.top),
        (scale.left - size.0 - gap, body.top),
    ];
    let candidates = positions.map(|(left, top)| {
        floating(
            ExactRect {
                left,
                top,
                width: size.0,
                height: size.1,
            },
            bounds,
            gap,
        )
    });
    Some(
        candidates
            .into_iter()
            .find(|candidate| {
                !candidate.intersects(scale.outset(gap))
                    && occupied
                        .iter()
                        .all(|other| !candidate.intersects(other.outset(gap)))
            })
            .unwrap_or(candidates[0]),
    )
}

/// 【尺子】【稳定说明】按request和窗口rect返回稳定坐标中的单行说明，字号保持窗口整数像素
fn label(request: &LabelRequest, rect: ExactRect) -> Option<InfoLabel> {
    Some(InfoLabel {
        rect: rect.projected(Some(request.projection), true).rect()?,
        unit: request.unit,
        lines: request.lines.clone(),
        font_size: request.font_size,
    })
}

/// 【尺子】【全部说明布局】按requests与稳定bounds返回原版角度优先、长度避让的完整标签列表
pub(super) fn layout(requests: &[LabelRequest], bounds: Rect) -> Vec<InfoLabel> {
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
            if let Some(label) = label(request, rect) {
                result.push(label);
            }
        }
    }
    let lengths: Vec<_> = requests
        .iter()
        .filter_map(|request| scale(request, bounds).map(|scale| (request, scale)))
        .collect();
    // 2. 【尺子】【完整刻度登记】长度说明避开所有长度尺刻度，再按原始顺序登记已放置说明
    occupied.extend(lengths.iter().map(|(_, scale)| *scale));
    for (request, scale) in lengths {
        let allowed = ExactRect::from_rect(bounds).projected(Some(request.projection), false);
        if let Some(rect) = length(request, scale, allowed, &occupied) {
            occupied.push(rect);
            if let Some(label) = label(request, rect) {
                result.push(label);
            }
        }
    }
    result
}
