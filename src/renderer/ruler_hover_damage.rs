//! 【尺子悬停】【稀疏区域】将十字线与标签分解为互不重叠的物理矩形

use crate::features::ruler::{labels::InfoLabel, state::RulerHover};
use tiny_skia::{IntRect, Rect};

/// 【尺子悬停】【内容交集】按hover、labels、origin和scale返回damage内互不重叠的线条与标签范围
pub(super) fn regions(
    hover: &RulerHover,
    labels: &[InfoLabel],
    origin: (f32, f32),
    scale: f32,
    damage: IntRect,
) -> Vec<IntRect> {
    let padding = 1.0 + 2.0 / scale;
    let lines = [
        Rect::from_ltrb(
            hover.bounds.left() - padding,
            hover.point.1 - padding,
            hover.bounds.right() + padding,
            hover.point.1 + padding,
        ),
        Rect::from_ltrb(
            hover.point.0 - padding,
            hover.bounds.top() - padding,
            hover.point.0 + padding,
            hover.bounds.bottom() + padding,
        ),
    ];
    let rectangles: Vec<_> = lines
        .into_iter()
        .flatten()
        .chain(
            labels
                .iter()
                .chain(std::iter::once(&hover.label))
                .filter_map(|label| label.rect.outset(padding, padding)),
        )
        .filter_map(|rect| super::pixel_bounds::map(rect, origin, f64::from(scale)))
        .filter_map(|rect| rect.intersect(&damage))
        .collect();
    union(&rectangles)
}

/// 【尺子悬停】【矩形合并】按rectangles的纵向边缘分带，返回覆盖同一并集但不重叠的矩形，避免重复混合半透明交点
fn union(rectangles: &[IntRect]) -> Vec<IntRect> {
    let mut rows: Vec<_> = rectangles
        .iter()
        .flat_map(|rect| [rect.top(), rect.bottom()])
        .collect();
    rows.sort_unstable();
    rows.dedup();
    let mut result = Vec::new();
    for row in rows.windows(2) {
        let mut columns: Vec<_> = rectangles
            .iter()
            .filter(|rect| rect.top() <= row[0] && rect.bottom() >= row[1])
            .map(|rect| (rect.left(), rect.right()))
            .collect();
        columns.sort_unstable();
        let mut merged: Vec<(i32, i32)> = Vec::new();
        for (left, right) in columns {
            if let Some(last) = merged.last_mut()
                && left <= last.1
            {
                last.1 = last.1.max(right);
            } else {
                merged.push((left, right));
            }
        }
        result.extend(
            merged
                .into_iter()
                .filter_map(|(left, right)| IntRect::from_ltrb(left, row[0], right, row[1])),
        );
    }
    result
}
