//! 【框选翻译】【面板避让】保持原版六像素间距、八像素边距和外侧优先规则

use tiny_skia::Rect;

/// 【框选翻译】【边界收纳】按 bounds 平移 rect，返回保留尺寸的安全位置
fn clamp(rect: Rect, bounds: Rect) -> Rect {
    let x = rect.left().clamp(
        bounds.left() + 8.0,
        (bounds.right() - 8.0 - rect.width()).max(bounds.left() + 8.0),
    );
    let y = rect.top().clamp(
        bounds.top() + 8.0,
        (bounds.bottom() - 8.0 - rect.height()).max(bounds.top() + 8.0),
    );
    Rect::from_xywh(x, y, rect.width(), rect.height()).unwrap()
}

/// 【框选翻译】【语言定位】按 anchor、vertical、size 和 bounds 返回语言列表矩形
pub fn language(anchor: Rect, vertical: bool, size: (f32, f32), bounds: Rect) -> Rect {
    let (x, y) = if vertical {
        (
            if anchor.left() + anchor.width() / 2.0 > bounds.left() + bounds.width() / 2.0 {
                anchor.left() - 6.0 - size.0
            } else {
                anchor.right() + 6.0
            },
            anchor.top(),
        )
    } else {
        (
            anchor.right() - size.0,
            if anchor.top() + anchor.height() / 2.0 > bounds.top() + bounds.height() / 2.0 {
                anchor.top() - 6.0 - size.1
            } else {
                anchor.bottom() + 6.0
            },
        )
    };
    clamp(Rect::from_xywh(x, y, size.0, size.1).unwrap(), bounds)
}

/// 【框选翻译】【候选横行】按 region、top、left、size、bounds、obstacles、slide 和 outside 返回无重叠位置
#[allow(clippy::too_many_arguments)]
fn row(
    region: Rect,
    top: f32,
    left: f32,
    size: (f32, f32),
    bounds: Rect,
    obstacles: &[Rect],
    slide: bool,
    outside: bool,
) -> Option<Rect> {
    let mut lefts = vec![left];
    if slide {
        let mut sides = obstacles
            .iter()
            .flat_map(|r| [r.right() + 4.0, r.left() - 4.0 - size.0])
            .collect::<Vec<_>>();
        sides.sort_by(|a, b| (a - left).abs().total_cmp(&(b - left).abs()));
        lefts.extend(sides);
    }
    lefts.into_iter().find_map(|x| {
        let candidate = clamp(Rect::from_xywh(x, top, size.0, size.1).unwrap(), bounds);
        let beside = candidate.left() < region.right() && candidate.right() > region.left();
        let overlaps = obstacles.iter().any(|r| {
            candidate
                .intersect(
                    &Rect::from_xywh(
                        r.left() - 4.0,
                        r.top() - 4.0,
                        r.width() + 8.0,
                        r.height() + 8.0,
                    )
                    .unwrap(),
                )
                .is_some()
        });
        (beside && (!outside || candidate.intersect(&region).is_none()) && !overlaps)
            .then_some(candidate)
    })
}

/// 【框选翻译】【浮条定位】按 region、size、bounds 和 obstacles 优先外侧右对齐，其次平移，最后放入右上角
pub fn result(region: Rect, size: (f32, f32), bounds: Rect, obstacles: &[Rect]) -> Rect {
    let left = region.right() - size.0;
    for slide in [false, true] {
        for top in [region.top() - 6.0 - size.1, region.bottom() + 6.0] {
            if let Some(rect) = row(region, top, left, size, bounds, obstacles, slide, true) {
                return rect;
            }
        }
    }
    row(
        region,
        region.top() + 6.0,
        left - 6.0,
        size,
        bounds,
        obstacles,
        true,
        false,
    )
    .unwrap_or_else(|| {
        clamp(
            Rect::from_xywh(left - 6.0, region.top() + 6.0, size.0, size.1).unwrap(),
            bounds,
        )
    })
}
