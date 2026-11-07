//! 【激光】【状态管理】保存草稿、缓存平滑路径，并在过期或容量不足时释放笔迹

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tiny_skia::{Color, Path, Rect};

use crate::features::annotation_style::units::Units;
use crate::renderer::stroke_path::freehand_path;

pub const LIFETIME: Duration = Duration::from_millis(1800);
pub const REFRESH_INTERVAL: Duration = Duration::from_millis(33);
pub const MAX_STROKES: usize = 64;
pub const MAX_POINTS_PER_STROKE: usize = 4096;
pub const MAX_TOTAL_POINTS: usize = 16_384;

#[derive(Default)]
pub struct LaserTrace {
    draft: Option<LaserStroke>,
    completed: VecDeque<LaserStroke>,
    completed_points: usize,
}

pub struct LaserStroke {
    points: Vec<(f32, f32)>,
    pub(super) path: Option<Path>,
    pub(super) color: Color,
    pub(super) width: f32,
    units: Units,
    canvas_bounds: Rect,
    visual_bounds: Rect,
    expires_at: Option<Instant>,
}

impl LaserTrace {
    /// 【激光】【开始手势】以 point、color、width 创建草稿，bounds 限定源图范围；返回重绘区域
    pub fn begin(
        &mut self,
        point: (f32, f32),
        color: Color,
        width: f32,
        bounds: Rect,
    ) -> Option<Rect> {
        self.begin_in_units(point, color, width, bounds, Units::default())
    }

    /// 【激光】【原图手势】按point、color、原图width、bounds与units创建画布草稿，返回新旧重绘范围，不保存导航倍率
    pub(crate) fn begin_in_units(
        &mut self,
        point: (f32, f32),
        color: Color,
        width: f32,
        bounds: Rect,
        units: Units,
    ) -> Option<Rect> {
        if !width.is_finite() || width <= 0.0 {
            return None;
        }
        let point = clamp_point(point, bounds)?;
        let old = self.cancel_draft();
        let draft = LaserStroke::new(point, color, units.canvas(width), bounds, units);
        let damage = union(old, Some(draft.bounds()));
        self.draft = Some(draft);
        union(damage, self.trim_capacity())
    }

    /// 【激光】【采样手势】向草稿追加 point 并插入有限中间点；忽略抖动，返回新旧笔迹重绘区域
    pub fn extend(&mut self, point: (f32, f32)) -> Option<Rect> {
        let draft = self.draft.as_mut()?;
        let next = clamp_point(point, draft.canvas_bounds)?;
        let last = *draft.points.last()?;
        let distance = (next.0 - last.0).hypot(next.1 - last.1);
        let step = draft
            .units
            .canvas((draft.image_stroke_width() * 0.38).max(2.0));
        if !distance.is_finite() || distance < step * 0.35 {
            return None;
        }

        // 1. 【激光】【采样手势】保留原版距离插值规则，限制单次输入产生的点数
        let old = draft.bounds();
        let inserts = (distance / step).floor().min(24.0) as usize;
        for index in 1..=inserts {
            let fraction = index as f32 / (inserts + 1) as f32;
            draft.points.push((
                last.0 + (next.0 - last.0) * fraction,
                last.1 + (next.1 - last.1) * fraction,
            ));
        }
        draft.points.push(next);

        // 2. 【激光】【采样手势】长手势逐次简化，完成路径在消散期间直接复用
        simplify(&mut draft.points);
        draft.update_geometry();
        let damage = union(Some(old), Some(draft.bounds()));
        union(damage, self.trim_capacity())
    }

    /// 【激光】【提交手势】在 now 开始消散，多点草稿才进入临时列表；返回提交及淘汰区域
    pub fn commit(&mut self, now: Instant) -> Option<Rect> {
        let mut draft = self.draft.take()?;
        let damage = Some(draft.bounds());
        if draft.points.len() >= 2 {
            draft.expires_at = Some(now + LIFETIME);
            self.completed_points += draft.points.len();
            self.completed.push_back(draft);
        }
        union(damage, self.trim_capacity())
    }

    /// 【激光】【取消草稿】删除未提交手势，保留完成笔迹；返回需要清除的旧区域
    pub fn cancel_draft(&mut self) -> Option<Rect> {
        self.draft.take().map(|stroke| stroke.bounds())
    }

    /// 【激光】【消散刷新】依据 now 删除过期笔迹；返回所有完成笔迹区域，包括最后清除区域
    pub fn tick(&mut self, now: Instant) -> Option<Rect> {
        let mut damage = None;
        let mut removed_points = 0;
        self.completed.retain(|stroke| {
            damage = union(damage, Some(stroke.bounds()));
            if stroke.expires_at.is_some_and(|expires| expires <= now) {
                removed_points += stroke.points.len();
                false
            } else {
                true
            }
        });
        self.completed_points -= removed_points;
        damage
    }

    /// 【激光】【空闲判断】返回是否存在需要定时消散的完成笔迹；草稿只随输入重绘
    pub fn is_animating(&self) -> bool {
        !self.completed.is_empty()
    }

    /// 【激光】【绘制判断】返回临时列表与草稿是否均为空
    pub fn is_empty(&self) -> bool {
        self.completed.is_empty() && self.draft.is_none()
    }

    /// 【激光】【笔迹读取】按绘制顺序返回完成笔迹及最后的草稿，不复制路径或采样点
    pub fn strokes(&self) -> impl DoubleEndedIterator<Item = &LaserStroke> {
        self.completed.iter().chain(self.draft.iter())
    }

    /// 【激光】【容量统计】返回草稿与完成笔迹的总采样点数
    pub fn point_count(&self) -> usize {
        self.completed_points + self.draft.as_ref().map_or(0, |stroke| stroke.points.len())
    }

    /// 【激光】【容量清理】优先淘汰最旧完成笔迹；返回淘汰区域，保持数量与总点数上限
    fn trim_capacity(&mut self) -> Option<Rect> {
        let mut damage = None;
        while self.completed.len() > MAX_STROKES || self.point_count() > MAX_TOTAL_POINTS {
            let Some(stroke) = self.completed.pop_front() else {
                break;
            };
            self.completed_points -= stroke.points.len();
            damage = union(damage, Some(stroke.bounds()));
        }
        damage
    }
}

impl LaserStroke {
    /// 【激光】【草稿构造】按point、color、画布width、bounds与units初始化采样和辉光边界，返回独立草稿
    fn new(point: (f32, f32), color: Color, width: f32, bounds: Rect, units: Units) -> Self {
        let mut stroke = Self {
            points: vec![point],
            path: None,
            color,
            width,
            units,
            canvas_bounds: bounds,
            visual_bounds: bounds,
            expires_at: None,
        };
        stroke.update_geometry();
        stroke
    }

    /// 【激光】【采样读取】返回只读逻辑坐标数组
    pub fn points(&self) -> &[(f32, f32)] {
        &self.points
    }

    /// 【激光】【原图笔宽】无参数，返回创建时的原图笔宽，不受当前窗口和导航状态影响
    pub fn image_stroke_width(&self) -> f32 {
        self.units.image(self.width)
    }

    /// 【激光】【重绘边界】返回包含 2.6 倍辉光宽度和抗锯齿留量的逻辑区域
    pub fn bounds(&self) -> Rect {
        self.visual_bounds
    }

    /// 【激光】【消散比例】依据 now 返回 0 到 1 的透明度倍率；未提交草稿固定为 1
    pub fn opacity(&self, now: Instant) -> f32 {
        self.expires_at.map_or(1.0, |expires| {
            (expires.saturating_duration_since(now).as_secs_f32() / LIFETIME.as_secs_f32())
                .clamp(0.0, 1.0)
        })
    }

    /// 【激光】【路径缓存】采样变化后重建平滑路径和辉光边界；无返回值
    fn update_geometry(&mut self) {
        self.path = (self.points.len() >= 2)
            .then(|| freehand_path(&self.points, 0, None))
            .flatten();
        let pad = self.width.max(3.0) * 1.3 + 2.0;
        let (mut left, mut top, mut right, mut bottom) = (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        );
        for &(x, y) in &self.points {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
        self.visual_bounds = Rect::from_ltrb(left - pad, top - pad, right + pad, bottom + pad)
            .unwrap_or(self.canvas_bounds);
    }
}

/// 【激光】【坐标约束】把有效 point 限制到 bounds；非有限坐标返回空值
fn clamp_point(point: (f32, f32), bounds: Rect) -> Option<(f32, f32)> {
    (point.0.is_finite() && point.1.is_finite()).then(|| {
        (
            point.0.clamp(bounds.left(), bounds.right()),
            point.1.clamp(bounds.top(), bounds.bottom()),
        )
    })
}

/// 【激光】【长手势简化】超过上限时原地保留隔点采样及首尾点；points 为可变坐标数组，无返回值
fn simplify(points: &mut Vec<(f32, f32)>) {
    if points.len() <= MAX_POINTS_PER_STROKE {
        return;
    }
    let last = *points.last().unwrap();
    let mut write = 1;
    for read in (2..points.len() - 1).step_by(2) {
        points[write] = points[read];
        write += 1;
    }
    points[write] = last;
    points.truncate(write + 1);
}

/// 【激光】【区域合并】合并可选区域 a 与 b；返回包含两者的区域
fn union(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Rect::from_ltrb(
            a.left().min(b.left()),
            a.top().min(b.top()),
            a.right().max(b.right()),
            a.bottom().max(b.bottom()),
        ),
        (a, b) => a.or(b),
    }
}
