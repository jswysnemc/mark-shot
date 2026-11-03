//! 【尺子】【说明布局】按原图边界适配完整测量文字，长度说明避开其他尺子和已放置说明

use cosmic_text::FontSystem;
use tiny_skia::Rect;

use super::geometry::{overlaps, rect_center, rotated};
use super::{Measurement, Point, RulerGeometry};
use crate::renderer::text::measure_line_width;

#[derive(Clone, Debug, PartialEq)]
pub struct LabelRequest {
    pub measurement: Measurement,
    pub unit: f32,
    pub lines: Vec<String>,
    pub font_size: f32,
    pub size: (f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct InfoLabel {
    pub rect: Rect,
    pub unit: f32,
    pub lines: Vec<String>,
    pub font_size: f32,
}

/// 【尺子】【说明准备】使用 measurement、width、bounds 和 fonts 计算完整文字与尺寸；窄范围改为多行，不省略测量数据
pub fn prepare(
    measurement: Measurement,
    width: f32,
    bounds: Rect,
    fonts: &mut FontSystem,
) -> Option<LabelRequest> {
    let text = measurement.info_text()?;
    let unit = measurement.unit(width);
    let gap = (8.0 * unit)
        .min(bounds.width() / 8.0)
        .min(bounds.height() / 8.0);
    let maximum_width = (bounds.width() - gap * 2.0).max(1.0);
    let maximum_height = (bounds.height() - gap * 2.0).max(1.0);
    let mut font_size = 13.0 * unit;
    let padding_x = (20.0 * unit).min(maximum_width / 4.0);
    let padding_y = (10.0 * unit).min(maximum_height / 4.0);
    let mut lines = vec![text];
    let measured = measure_line_width(&lines[0], font_size, fonts);
    if measured + padding_x > maximum_width
        && let Some((width, height, diagonal, area)) = measurement.length_metrics()
    {
        lines = vec![
            format!("{width} x {height} px"),
            format!("diag {diagonal} px"),
            format!("area {area} px"),
        ];
    }
    let text_width = lines
        .iter()
        .map(|line| measure_line_width(line, font_size, fonts))
        .fold(0.0, f32::max);
    let line_height = crate::theme::font::line_height();
    let text_height = lines.len() as f32 * font_size * line_height;
    let factor = ((maximum_width - padding_x) / text_width.max(1.0))
        .min((maximum_height - padding_y) / text_height.max(1.0))
        .clamp(f32::EPSILON, 1.0);
    font_size *= factor;
    Some(LabelRequest {
        measurement,
        unit,
        lines,
        font_size,
        size: (
            (text_width * factor + padding_x)
                .min(maximum_width)
                .max(1.0),
            (text_height * factor + padding_y)
                .min(maximum_height)
                .max(1.0),
        ),
    })
}

/// 【尺子】【浮动范围约束】把 rect 放回 bounds 内并保留 gap；超大范围缩小到可用尺寸，返回有效矩形
pub fn clamped(rect: Rect, bounds: Rect, gap: f32) -> Rect {
    let gap = gap
        .max(0.0)
        .min(bounds.width() / 8.0)
        .min(bounds.height() / 8.0);
    let width = rect.width().min((bounds.width() - gap * 2.0).max(1.0));
    let height = rect.height().min((bounds.height() - gap * 2.0).max(1.0));
    let left = rect.left().clamp(
        bounds.left() + gap,
        (bounds.right() - gap - width).max(bounds.left() + gap),
    );
    let top = rect.top().clamp(
        bounds.top() + gap,
        (bounds.bottom() - gap - height).max(bounds.top() + gap),
    );
    Rect::from_xywh(left, top, width, height).unwrap_or(bounds)
}

/// 【尺子】【说明候选】依据 body、size 和 gap 返回下、上、右、左四个说明候选矩形
fn candidates(body: Rect, size: (f32, f32), gap: f32) -> [Rect; 4] {
    [
        (body.left(), body.bottom() + gap),
        (body.left(), body.top() - size.1 - gap),
        (body.right() + gap, body.top()),
        (body.left() - size.0 - gap, body.top()),
    ]
    .map(|(x, y)| Rect::from_xywh(x, y, size.0, size.1).unwrap_or(body))
}

/// 【尺子】【角度圆弧】根据 measurement 与 unit 返回半径、第一边方向和较小有向夹角；不完整角度返回空值
pub fn angle_arc(measurement: Measurement, unit: f32) -> Option<(f32, f32, f32)> {
    let RulerGeometry::Angle { points, count: 3 } = measurement.geometry else {
        return None;
    };
    let first = (points[1].0 - points[0].0, points[1].1 - points[0].1);
    let second = (points[2].0 - points[0].0, points[2].1 - points[0].1);
    let shorter = first.0.hypot(first.1).min(second.0.hypot(second.1));
    let radius = (shorter * 0.4 * unit / measurement.style_scale)
        .clamp(14.0 * unit, 44.0 * unit)
        .min(shorter * 0.8);
    let start = first.1.atan2(first.0);
    let sweep =
        (first.0 * second.1 - first.1 * second.0).atan2(first.0 * second.0 + first.1 * second.1);
    Some((radius, start, sweep))
}

/// 【尺子】【角度说明位置】将 request 放在角平分线圆弧外侧；bounds 约束范围，返回说明矩形
fn angle_rect(request: &LabelRequest, bounds: Rect) -> Rect {
    let (radius, start, sweep) =
        angle_arc(request.measurement, request.unit).unwrap_or((0.0, 0.0, 0.0));
    let vertex = request.measurement.start();
    let bisector = start + sweep / 2.0;
    let distance = radius + 10.0 * request.unit + request.size.0.hypot(request.size.1) / 2.0;
    let center = (
        vertex.0 + bisector.cos() * distance,
        vertex.1 + bisector.sin() * distance,
    );
    let rect = Rect::from_xywh(
        center.0 - request.size.0 / 2.0,
        center.1 - request.size.1 / 2.0,
        request.size.0,
        request.size.1,
    )
    .unwrap_or(bounds);
    clamped(rect, bounds, 8.0 * request.unit)
}

/// 【尺子】【全部说明布局】将 requests 按角度优先、长度避让顺序放入 bounds；返回最后绘制的说明列表
pub fn layout(requests: &[LabelRequest], bounds: Rect) -> Vec<InfoLabel> {
    let mut occupied: Vec<Rect> = requests
        .iter()
        .filter_map(|request| request.measurement.visual_bounds(request.unit))
        .collect();
    let mut result = Vec::with_capacity(requests.len());
    // 1. 【尺子】【全部说明布局】角度说明固定在角平分线方向，先登记为占用范围
    for request in requests
        .iter()
        .filter(|request| matches!(request.measurement.geometry, RulerGeometry::Angle { .. }))
    {
        let rect = angle_rect(request, bounds);
        occupied.push(rect);
        result.push(InfoLabel {
            rect,
            unit: request.unit,
            lines: request.lines.clone(),
            font_size: request.font_size,
        });
    }
    // 2. 【尺子】【全部说明布局】长度说明避开全部测量范围与已放置说明
    for request in requests
        .iter()
        .filter(|request| matches!(request.measurement.geometry, RulerGeometry::Length { .. }))
    {
        let body = request
            .measurement
            .visual_bounds(request.unit)
            .unwrap_or(bounds);
        let gap = 8.0 * request.unit;
        let candidates = candidates(body, request.size, gap);
        let rect = candidates
            .iter()
            .map(|candidate| clamped(*candidate, bounds, gap))
            .find(|candidate| {
                occupied
                    .iter()
                    .all(|other| !overlaps(other.outset(gap, gap).unwrap_or(*other), *candidate))
            })
            .unwrap_or_else(|| clamped(candidates[0], bounds, gap));
        occupied.push(rect);
        result.push(InfoLabel {
            rect,
            unit: request.unit,
            lines: request.lines.clone(),
            font_size: request.font_size,
        });
    }
    result
}

/// 【尺子】【说明集合】为 rulers 提供的测量与笔宽计算完整说明；bounds 为全局允许范围，fonts 提供字体，返回布局结果
pub fn prepare_all(
    rulers: impl Iterator<Item = (Measurement, f32)>,
    bounds: Rect,
    fonts: &mut FontSystem,
) -> Vec<InfoLabel> {
    let requests: Vec<_> = rulers
        .filter_map(|(measurement, width)| prepare(measurement, width, bounds, fonts))
        .collect();
    layout(&requests, bounds)
}

impl Measurement {
    /// 【尺子】【刻度视觉范围】在几何边界上加入 unit 对应的刻度与数字留量；返回旋转后的外接矩形
    pub fn visual_bounds(self, unit: f32) -> Option<Rect> {
        if let RulerGeometry::Length { rotation, .. } = self.geometry {
            let bounds = self.unrotated_bounds()?;
            let digits = self.length_metrics().map_or(1, |(width, height, _, _)| {
                width.max(height).to_string().len()
            });
            let left = (14.0 + digits as f32 * 8.0) * unit;
            let top = 32.0 * unit;
            let local = Rect::from_ltrb(
                bounds.left() - left,
                bounds.top() - top,
                bounds.right() + left,
                bounds.bottom() + top,
            )?;
            let center = rect_center(bounds);
            let points = [
                (local.left(), local.top()),
                (local.right(), local.top()),
                (local.right(), local.bottom()),
                (local.left(), local.bottom()),
            ]
            .map(|point| rotated(point, center, rotation));
            return super::geometry::points_bounds(&points, 1.0 / self.pixel_scale);
        }
        self.bounds()?.outset(4.0 * unit, 4.0 * unit)
    }
}

/// 【尺子】【数字边界约束】在数字对应 tick 位于 bounds 内时约束 rect；范围外刻度不移动到截图内部
pub fn number_rect(rect: Rect, tick: Point, bounds: Option<Rect>) -> Rect {
    match bounds {
        Some(bounds) if super::geometry::contains(bounds, tick) => clamped(rect, bounds, 0.0),
        _ => rect,
    }
}
