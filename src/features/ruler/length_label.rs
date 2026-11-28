//! 【尺子】【长度说明选位】优先在真实尺身内分行，其次比较四周空位，最后才允许局部遮挡

use super::{
    label_fit::{Area, Interior, closest, distance},
    label_model::LabelRequest,
    layout_rect::ExactRect,
    tick_layout::Layout,
};

pub(super) struct Frame {
    body: ExactRect,
    rotation: f32,
    pub scale: ExactRect,
    bands: Vec<ExactRect>,
}

impl Frame {
    /// 【尺子】【尺身建模】按 request 和 bounds 返回投影后的正文、刻度外框及移入尺身的数字条带
    pub fn new(request: &LabelRequest, bounds: tiny_skia::Rect) -> Option<Self> {
        let layout = Layout::in_projection(
            request.measurement,
            request.width,
            Some(bounds),
            request.projection,
        )?;
        let projected =
            |rect| ExactRect::from_rect(rect).projected(Some(request.projection), false);
        let body = projected(layout.paint_rect);
        let mut bands = Vec::new();
        if layout.clamp {
            bands.extend(
                [layout.vertical_band, layout.horizontal_band]
                    .map(projected)
                    .into_iter()
                    .filter(|band| band.intersects(body)),
            );
        }
        Some(Self {
            body,
            rotation: layout.rotation,
            scale: layout.projected_scale_bounds(),
            bands,
        })
    }
}

#[derive(Clone, Copy)]
pub(super) struct Placement<'a> {
    pub rect: ExactRect,
    pub lines: &'a [String],
}

impl<'a> Placement<'a> {
    /// 【尺子】【候选比较】按 rank 保留成本最低的位置及文字，完全同分时保留原顺序，无返回值
    fn offer<Rank: PartialOrd + Copy>(self, best: &mut Option<(Rank, Self)>, rank: Rank) {
        if best.is_none_or(|(previous, _)| rank < previous) {
            *best = Some((rank, self));
        }
    }
}

/// 【尺子】【长度说明布局】按 request 的换行方式、尺身 frame、窗口 bounds 和其他 obstacles 返回标签位置与实际行
pub(super) fn place<'a>(
    request: &'a LabelRequest,
    frame: &Frame,
    bounds: ExactRect,
    obstacles: impl Iterator<Item = ExactRect> + Clone,
) -> Option<Placement<'a>> {
    let flows = || {
        std::iter::once((request.lines.as_slice(), request.paint_size)).chain(
            request
                .wraps
                .iter()
                .map(|flow| (flow.lines.as_slice(), flow.size)),
        )
    };
    let gap = 4.0 * request.paint_unit;
    let forbidden = |size| {
        obstacles
            .clone()
            .chain(frame.bands.iter().copied())
            .map(|rect| Area::blocked(rect, size, gap))
            .collect::<Vec<_>>()
    };
    let body = frame.body;
    let focus = body.center();
    let mut best = None;

    // 1. 【尺子】【尺内优先】优先选能容纳的多行组合，比较其在尺身两轴上的占用比例及偏离中心的距离
    let (sin, cos) = f64::from(frame.rotation).to_radians().sin_cos();
    for (lines, size) in flows() {
        let Some(inner) = Interior::new(body, frame.rotation, size, gap) else {
            continue;
        };
        let Some(center) = closest(
            Area::available(bounds, size, gap),
            Some(&inner),
            focus,
            &forbidden(size),
        ) else {
            continue;
        };
        let across = (cos.abs() * size.0 + sin.abs() * size.1) / body.width;
        let down = (sin.abs() * size.0 + cos.abs() * size.1) / body.height;
        let cost =
            across.max(down) + distance(center, focus).sqrt() / body.width.hypot(body.height);
        Placement {
            rect: ExactRect::centered(center, size),
            lines,
        }
        .offer(&mut best, (lines.len() == 1, cost, lines.len()));
    }
    if let Some((_, placement)) = best {
        return Some(placement);
    }

    // 2. 【尺子】【尺外比较】沿刻度外框四侧寻找离尺身最近的空位，横向空间可保留一行，较窄的侧边可分行
    let mut outside = None;
    let mut fallback = None;
    for (lines, size) in flows() {
        let area = Area::available(bounds, size, gap);
        let blockers = forbidden(size);
        for (axis, sign, edge) in [
            (1, 1.0, frame.scale.bottom()),
            (1, -1.0, frame.scale.top),
            (0, 1.0, frame.scale.right()),
            (0, -1.0, frame.scale.left),
        ] {
            let mut side = area;
            let mut anchor = focus;
            anchor[axis] = edge;
            let mut target = anchor;
            target[axis] += sign * (gap * 2.0 + [size.0, size.1][axis] * 0.5);
            if sign > 0.0 {
                side.low[axis] = side.low[axis].max(target[axis]);
            } else {
                side.high[axis] = side.high[axis].min(target[axis]);
            }
            if let Some(center) = closest(side, None, target, &blockers) {
                Placement {
                    rect: ExactRect::centered(center, size),
                    lines,
                }
                .offer(&mut outside, (distance(center, anchor), lines.len()));
            } else if outside.is_none() {
                // 3. 【尺子】【空间不足】先最大化可见比例，再最小化尺身和其他说明的遮挡，不删减字段或压缩字号
                let center = std::array::from_fn(|axis| {
                    if area.low[axis] <= area.high[axis] {
                        target[axis].clamp(area.low[axis], area.high[axis])
                    } else {
                        bounds.center()[axis]
                    }
                });
                let rect = ExactRect::centered(center, size);
                let surface = size.0 * size.1;
                let clipped = 1.0 - size.0.min(bounds.width) * size.1.min(bounds.height) / surface;
                let overlap = rect.overlap(frame.scale)
                    + obstacles
                        .clone()
                        .map(|other| rect.overlap(other))
                        .sum::<f64>();
                Placement { rect, lines }.offer(
                    &mut fallback,
                    (clipped, overlap / surface, distance(center, anchor)),
                );
            }
        }
    }
    outside
        .map(|(_, placement)| placement)
        .or_else(|| fallback.map(|(_, placement)| placement))
}
