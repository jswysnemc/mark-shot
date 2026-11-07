//! 【序号】【窗口几何】实际圆框和尾部遵循窗口限制，持久原图选框保持独立

use super::NumberMark;
use crate::features::annotation_style::window::WindowUnits;
use tiny_skia::Rect;

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub mark: NumberMark,
    pub radius: f32,
    pub outline: f32,
    pub tail_half: f32,
}

impl Geometry {
    /// 【序号】【窗口尺寸】按mark、width与window返回实际圆框半径、白边和尾部尺寸，不写回标注
    pub fn new(mark: NumberMark, width: f32, window: WindowUnits) -> Self {
        let radius = window.stroke(mark.bubble_radius(width), 13.0, None);
        Self {
            mark,
            radius,
            outline: window.stroke(width * 0.22, 2.0, Some(9.0)),
            tail_half: window.stroke(radius * 0.46, 8.0, Some(38.0)),
        }
    }

    /// 【序号】【窗口尾部】无参数，返回实际旋转后三角尾部，短指向不绘制尾部
    pub fn tail(self) -> Option<[(f32, f32); 3]> {
        if !self.mark.valid() {
            return None;
        }
        let tip = self.mark.to_visual(self.mark.tip);
        let delta = (tip.0 - self.mark.bubble.0, tip.1 - self.mark.bubble.1);
        let length = delta.0.hypot(delta.1);
        if !length.is_finite() || length <= self.radius * 0.45 {
            return None;
        }
        let direction = (delta.0 / length, delta.1 / length);
        let normal = (-direction.1, direction.0);
        let base = (
            self.mark.bubble.0 + direction.0 * self.radius * 0.82,
            self.mark.bubble.1 + direction.1 * self.radius * 0.82,
        );
        Some([
            tip,
            (
                base.0 + normal.0 * self.tail_half,
                base.1 + normal.1 * self.tail_half,
            ),
            (
                base.0 - normal.0 * self.tail_half,
                base.1 - normal.1 * self.tail_half,
            ),
        ])
    }

    /// 【序号】【窗口边界】无参数，返回包含白边、圆框与实际尾部的边界，无效标记返回空值
    pub fn bounds(self) -> Option<Rect> {
        if !self.mark.valid() {
            return None;
        }
        let radius = self.radius + self.outline / 2.0;
        let mut bounds = [
            self.mark.bubble.0 - radius,
            self.mark.bubble.1 - radius,
            self.mark.bubble.0 + radius,
            self.mark.bubble.1 + radius,
        ];
        for point in self.tail().into_iter().flatten() {
            bounds[0] = bounds[0].min(point.0);
            bounds[1] = bounds[1].min(point.1);
            bounds[2] = bounds[2].max(point.0);
            bounds[3] = bounds[3].max(point.1);
        }
        Rect::from_ltrb(bounds[0], bounds[1], bounds[2], bounds[3])
    }

    /// 【序号】【标签损坏边界】按window返回正文与八点标签的保守联合边界，预留旋转字形高度，不依赖字体缓存或分配文字缓冲
    pub fn painted_bounds(self, window: WindowUnits) -> Option<Rect> {
        let bounds = self.bounds()?;
        let half = self.mark.style.label(self.mark.number).chars().count() as f32
            * window.canvas(12.0)
            / 2.0
            + window.canvas(8.0);
        let label = Rect::from_xywh(
            self.mark.bubble.0 - half,
            self.mark.bubble.1 - half,
            half * 2.0,
            half * 2.0,
        )?;
        bounds.join(&label)
    }
}
