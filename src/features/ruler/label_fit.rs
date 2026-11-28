//! 【尺子】【可放置区域】在窗口、旋转尺身及障碍之间寻找水平标签的可用中心

use super::layout_rect::ExactRect;

pub(super) type Center = [f64; 2];

#[derive(Clone, Copy)]
pub(super) struct Area {
    pub low: Center,
    pub high: Center,
}

impl Area {
    /// 【尺子】【容纳范围】按 bounds、卡片 size 与留白 gap 返回中心允许范围，空间紧张时先让出窗口留白
    pub fn available(bounds: ExactRect, size: (f64, f64), gap: f64) -> Self {
        let inset = |extent: f64, card: f64| card * 0.5 + gap.min(((extent - card) * 0.5).max(0.0));
        let horizontal = inset(bounds.width, size.0);
        let vertical = inset(bounds.height, size.1);
        Self {
            low: [bounds.left + horizontal, bounds.top + vertical],
            high: [bounds.right() - horizontal, bounds.bottom() - vertical],
        }
    }

    /// 【尺子】【禁放范围】按障碍 rect、卡片 size 和间距 gap 返回两者发生遮挡的中心范围
    pub fn blocked(rect: ExactRect, size: (f64, f64), gap: f64) -> Self {
        let horizontal = size.0 * 0.5 + gap;
        let vertical = size.1 * 0.5 + gap;
        Self {
            low: [rect.left - horizontal, rect.top - vertical],
            high: [rect.right() + horizontal, rect.bottom() + vertical],
        }
    }

    /// 【尺子】【障碍命中】判断 center 是否严格落在禁放范围内，相切仍保留预留的间距
    fn blocks(self, center: Center) -> bool {
        (0..2).all(|axis| self.low[axis] < center[axis] && center[axis] < self.high[axis])
    }
}

pub(super) struct Interior {
    center: Center,
    axes: [Center; 2],
    half: Center,
}

impl Interior {
    /// 【尺子】【尺内范围】扣除水平卡片 size 在旋转尺身 body 两轴上的投影和 gap，不能容纳时返回空值
    pub fn new(body: ExactRect, rotation: f32, size: (f64, f64), gap: f64) -> Option<Self> {
        let (sin, cos) = f64::from(rotation).to_radians().sin_cos();
        let spare = [
            (body.width - cos.abs() * size.0 - sin.abs() * size.1) * 0.5,
            (body.height - sin.abs() * size.0 - cos.abs() * size.1) * 0.5,
        ];
        // 1. 【尺子】【紧凑留白】贴近容量上限时可让出装饰间距，但仍保留边框的半笔宽
        spare
            .iter()
            .all(|&extent| extent >= gap * 0.25)
            .then_some(Self {
                center: body.center(),
                axes: [[cos, sin], [-sin, cos]],
                half: spare.map(|extent| (extent - gap).max(0.0)),
            })
    }

    /// 【尺子】【尺内截面】沿 axis 固定坐标 fixed，返回另一轴与 range 共有的可放置区间
    fn span(&self, axis: usize, fixed: f64, mut range: [f64; 2]) -> Option<[f64; 2]> {
        let other = 1 - axis;
        for (normal, half) in self.axes.iter().zip(self.half) {
            let offset = (fixed - self.center[axis]) * normal[axis];
            if normal[other].abs() < 1e-12 {
                if offset.abs() > half + 1e-9 {
                    return None;
                }
            } else {
                let first = self.center[other] + (-half - offset) / normal[other];
                let last = self.center[other] + (half - offset) / normal[other];
                range[0] = range[0].max(first.min(last));
                range[1] = range[1].min(first.max(last));
            }
        }
        (range[0] <= range[1]).then_some(range)
    }
}

/// 【尺子】【位置距离】按 first 和 second 返回窗口距离的平方，比较候选位置时无需开方
pub(super) fn distance(first: Center, second: Center) -> f64 {
    (first[0] - second[0]).powi(2) + (first[1] - second[1]).powi(2)
}

/// 【尺子】【空位查找】在 area 及可选的 inside 内避开 blockers，返回尽量接近 target 的标签中心
pub(super) fn closest(
    area: Area,
    inside: Option<&Interior>,
    target: Center,
    blockers: &[Area],
) -> Option<Center> {
    if (0..2).any(|axis| area.low[axis] > area.high[axis]) {
        return None;
    }
    let preferred = std::array::from_fn(|axis| target[axis].clamp(area.low[axis], area.high[axis]));
    if inside.is_none_or(|inner| inner.span(0, preferred[0], [preferred[1]; 2]).is_some())
        && !blockers.iter().any(|rect| rect.blocks(preferred))
    {
        return Some(preferred);
    }

    // 1. 【尺子】【边界扫描】只扫描窗口和障碍的边，不逐像素搜索；两轴均检查，可找到旋转尺身被窗口裁出的狭小空位
    let mut nearby: Vec<_> = blockers
        .iter()
        .filter(|rect| {
            (0..2).all(|axis| rect.low[axis] < area.high[axis] && rect.high[axis] > area.low[axis])
        })
        .collect();
    let mut best = None;
    let mut cost = f64::INFINITY;
    for axis in 0..2 {
        let other = 1 - axis;
        let mut stops = vec![preferred[axis], area.low[axis], area.high[axis]];
        for rect in &nearby {
            stops.extend([rect.low[axis], rect.high[axis]]);
        }
        if let Some(inner) = inside {
            for sign in [-1.0, 1.0] {
                for cross in [-1.0, 1.0] {
                    stops.push(
                        inner.center[axis]
                            + sign * inner.half[0] * inner.axes[0][axis]
                            + cross * inner.half[1] * inner.axes[1][axis],
                    );
                }
            }
        }
        stops.sort_unstable_by(f64::total_cmp);
        stops.dedup();
        nearby.sort_unstable_by(|left, right| left.low[other].total_cmp(&right.low[other]));
        for fixed in stops
            .into_iter()
            .filter(|&value| value >= area.low[axis] && value <= area.high[axis])
        {
            let range = [area.low[other], area.high[other]];
            let Some([mut low, high]) =
                inside.map_or(Some(range), |inner| inner.span(axis, fixed, range))
            else {
                continue;
            };
            let mut consider = |start: f64, end: f64| {
                if start <= end {
                    let mut center = target;
                    center[axis] = fixed;
                    center[other] = target[other].clamp(start, end);
                    let score = distance(center, target);
                    if score < cost {
                        cost = score;
                        best = Some(center);
                    }
                }
            };
            // 2. 【尺子】【截面避让】按起点合并本截面上的遮挡，只在剩余区间选择最接近目标的坐标
            for rect in nearby
                .iter()
                .filter(|rect| rect.low[axis] < fixed && fixed < rect.high[axis])
            {
                if rect.low[other] > high {
                    break;
                }
                consider(low, rect.low[other].min(high));
                low = low.max(rect.high[other]);
            }
            consider(low, high);
        }
    }
    best
}
