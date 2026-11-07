//! 【滚轮提示】【窗口避让】按原版四个候选位置选择气泡，控件和热点都参与重叠面积判断

use tiny_skia::Rect;

/// 【滚轮提示】【交集面积】按first与second返回正面积交集，空交集返回零
fn overlap(first: Rect, second: Rect) -> f64 {
    first.intersect(&second).map_or(0.0, |rect| {
        f64::from(rect.width()) * f64::from(rect.height())
    })
}

/// 【滚轮提示】【气泡位置】按position、size、footprint、viewport和obstacles返回窗口内重叠最少的原版候选
pub fn badge(
    position: (f32, f32),
    size: (f32, f32),
    footprint: Option<f32>,
    viewport: Rect,
    obstacles: &[Rect],
) -> Rect {
    let radius = footprint.unwrap_or(0.0) / 2.0;
    let distance = (radius + 12.0).max(18.0);
    let pointer_radius = (radius + 4.0).max(12.0);
    let pointer = Rect::from_xywh(
        position.0 - pointer_radius,
        position.1 - pointer_radius,
        pointer_radius * 2.0,
        pointer_radius * 2.0,
    )
    .unwrap();
    let candidates = [
        (distance, 16.0),
        (distance, -size.1 - 16.0),
        (-distance - size.0, 16.0),
        (-distance - size.0, -size.1 - 16.0),
    ];
    let mut fallback = None;
    let mut minimum = f64::INFINITY;
    // 1. 【滚轮提示】【候选顺序】优先右下、右上、左下、左上，边缘平移后仍检查实际遮挡面积
    for (x, y) in candidates {
        let rect = Rect::from_xywh(
            (position.0 + x).clamp(
                viewport.left() + 8.0,
                (viewport.right() - size.0 - 8.0).max(viewport.left() + 8.0),
            ),
            (position.1 + y).clamp(
                viewport.top() + 8.0,
                (viewport.bottom() - size.1 - 8.0).max(viewport.top() + 8.0),
            ),
            size.0,
            size.1,
        )
        .unwrap();
        let area = overlap(rect, pointer)
            + obstacles
                .iter()
                .map(|obstacle| overlap(rect, obstacle.outset(4.0, 4.0).unwrap_or(*obstacle)))
                .sum::<f64>();
        if area == 0.0 {
            return rect;
        }
        if area < minimum {
            minimum = area;
            fallback = Some(rect);
        }
    }
    fallback.unwrap()
}
