//! 【区域吸附】【边界查询】尺子与选区共享识别边界、遮挡判断及窗口容差

use super::{State, geometry::Space};
use crate::{
    editor::EditorState,
    features::{
        region_detection::window::topmost,
        ruler::{Point, geometry::contains},
    },
};
use tiny_skia::Rect;

pub(super) struct Targets<'a> {
    region: &'a State,
    space: Space,
    zone: Rect,
    radius: f32,
    native: Option<Rect>,
    canvas_edges: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Projection {
    pub horizontal: Option<f32>,
    pub vertical: Option<f32>,
}

impl Projection {
    /// 【尺子】【边界投影】按候选矩形、锚点和容差返回两轴修正量，边的延长线不参与吸附
    fn new(rect: Rect, point: Point, radius: f32) -> Option<Self> {
        let nearest = |value: f32, low: f32, high: f32| {
            let delta = if (value - low).abs() < (value - high).abs() {
                low - value
            } else {
                high - value
            };
            (delta.abs() <= radius).then_some(delta)
        };
        let projection = Self {
            horizontal: nearest(point.0, rect.left(), rect.right()),
            vertical: nearest(point.1, rect.top(), rect.bottom()),
        };
        ((projection.horizontal.is_some() || projection.vertical.is_some())
            && contains(rect, projection.apply(point)))
        .then_some(projection)
    }

    /// 【尺子】【坐标修正】按原始锚点返回投影后的画布坐标，无候选的轴保持不变
    pub(super) fn apply(self, point: Point) -> Point {
        (
            point.0 + self.horizontal.unwrap_or_default(),
            point.1 + self.vertical.unwrap_or_default(),
        )
    }
}

impl<'a> Targets<'a> {
    /// 【区域吸附】【尺子范围】按 state、pointer 返回当前选区内部的识别边界查询
    pub(super) fn for_ruler(state: &'a mut EditorState, pointer: Point) -> Option<Self> {
        let zone = state
            .selection
            .zone
            .filter(|zone| contains(*zone, pointer))?;
        Self::new(state, pointer, Some(zone))
    }

    /// 【区域吸附】【选区范围】按 state、pointer 返回整个截图范围的识别边界，允许向原选区外拖动
    pub(super) fn for_selection(state: &'a mut EditorState, pointer: Point) -> Option<Self> {
        Self::new(state, pointer, None)
    }

    /// 【区域吸附】【识别来源】按 state、pointer 和可选的 clip 建立查询，原生控件每次移动最多请求一次
    fn new(state: &'a mut EditorState, _pointer: Point, clip: Option<Rect>) -> Option<Self> {
        let mut space = state.region_detection.space?;
        if let Some(file) = &state.file_canvas {
            space.bounds = file.viewport;
            space.scale = f64::from(file.original.width()) / f64::from(file.viewport.width());
        }
        let zone = clip.unwrap_or(space.bounds);
        let radius = state.hit_window_units().canvas(6.0);
        #[cfg(any(target_os = "windows", test))]
        let native = state
            .region_detection
            .native
            .as_mut()
            .and_then(|native| native.hover((_pointer.0.into(), _pointer.1.into())));
        #[cfg(not(any(target_os = "windows", test)))]
        let native = None;
        Some(Self {
            region: &state.region_detection,
            space,
            zone,
            radius,
            native,
            canvas_edges: clip.is_none(),
        })
    }

    /// 【尺子】【有效候选】按锚点检查邻近边角，将可见投影与区域面积交给 visit，不改变原生悬停目标
    pub(super) fn visit(&self, point: Point, mut visit: impl FnMut(Projection, i64)) {
        if !self.canvas_edges && !contains(self.zone, point) {
            return;
        }
        self.visit_projected(
            |rect| Projection::new(rect, point, self.radius).map(|projection| (point, projection)),
            &mut visit,
        );
    }

    /// 【尺子】【平行边对齐】按尺边端点查找与候选边界重叠的区段，仅沿垂直于尺边的方向吸附
    pub(super) fn visit_edge(&self, first: Point, last: Point, visit: impl FnMut(Projection, i64)) {
        let horizontal = (first.1 - last.1).abs() < 0.001;
        if !horizontal && (first.0 - last.0).abs() >= 0.001 {
            return;
        }
        self.visit_projected(
            |rect| {
                let visible = rect.intersect(&self.zone)?;
                let (low, high) = if horizontal {
                    (
                        first.0.min(last.0).max(visible.left()),
                        first.0.max(last.0).min(visible.right()),
                    )
                } else {
                    (
                        first.1.min(last.1).max(visible.top()),
                        first.1.max(last.1).min(visible.bottom()),
                    )
                };
                if low >= high {
                    return None;
                }
                let middle = (low + high) * 0.5;
                let point = if horizontal {
                    (middle, first.1)
                } else {
                    (first.0, middle)
                };
                if !self.canvas_edges && !contains(self.zone, point) {
                    return None;
                }
                let mut projection = Projection::new(rect, point, self.radius)?;
                if horizontal {
                    projection.vertical?;
                    projection.horizontal = None;
                } else {
                    projection.horizontal?;
                    projection.vertical = None;
                }
                Some((point, projection))
            },
            visit,
        );
    }

    /// 【尺子】【可见边界】按 project 计算每个区域的投影，visit 接收通过选区和窗口遮挡检查的结果及面积
    fn visit_projected(
        &self,
        project: impl Fn(Rect) -> Option<(Point, Projection)>,
        mut visit: impl FnMut(Projection, i64),
    ) {
        let visible = |rect| {
            let (point, projection) = project(rect)?;
            let target = projection.apply(point);
            contains(self.zone, target).then_some((target, projection))
        };
        if let Some((_, projection)) = self.native.and_then(visible) {
            visit(projection, 0);
        }
        if self.canvas_edges
            && let Some((_, projection)) = visible(self.space.bounds)
        {
            visit(projection, i64::MAX);
        }
        let controls = self
            .region
            .detector
            .iter()
            .flat_map(|detector| detector.candidates())
            .filter(|candidate| !candidate.is_window())
            .map(|candidate| (candidate.rect, false));
        for (rect, window) in
            controls.chain(self.region.windows.iter().map(|window| (window.rect, true)))
        {
            if rect.width() <= 0 || rect.height() <= 0 {
                continue;
            }
            let Some((target, projection)) = self.space.global(rect).and_then(visible) else {
                continue;
            };
            // 1. 【尺子】【遮挡核验】从边界内侧判断前窗，允许从右边或下边的外侧接近
            let inside = (
                (((target.0 - self.space.bounds.left()) as f64 * self.space.scale).floor() as i32)
                    .clamp(rect.left, rect.right - 1),
                (((target.1 - self.space.bounds.top()) as f64 * self.space.scale).floor() as i32)
                    .clamp(rect.top, rect.bottom - 1),
            );
            if topmost(&self.region.windows, inside).is_some_and(|front| {
                if window {
                    front != rect
                } else {
                    !front.contains_rect(rect)
                }
            }) {
                continue;
            }
            visit(projection, rect.area());
        }
    }
}

#[derive(Default)]
pub(super) struct SnapAxes {
    best: [Option<(f32, i64)>; 2],
}

impl SnapAxes {
    /// 【区域吸附】【分轴择优】按 projection 和 area 分别记录横纵修正，原生控件优先，同距离优先小区域
    pub(super) fn consider(&mut self, projection: Projection, area: i64) {
        for (best, candidate) in self
            .best
            .iter_mut()
            .zip([projection.horizontal, projection.vertical])
        {
            if let Some(delta) = candidate
                && best.is_none_or(|(old, size)| {
                    (area != 0, delta.abs(), area) < (size != 0, old.abs(), size)
                })
            {
                *best = Some((delta, area));
            }
        }
    }

    /// 【区域吸附】【双边结果】返回各轴的最佳修正量，未接近任何边的轴保持空值
    pub(super) fn projection(&self) -> Projection {
        Projection {
            horizontal: self.best[0].map(|entry| entry.0),
            vertical: self.best[1].map(|entry| entry.0),
        }
    }
}
