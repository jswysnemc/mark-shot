//! 【序号】【几何变换】计算圆框、尾部和旋转轴心，分离界面尺寸与导出比例

use tiny_skia::Rect;

use super::{NumberMark, clamped_width};

impl NumberMark {
    /// 【序号】【圆框半径】按 width 和固定十三像素留量计算半径，导出时同比缩放；返回逻辑半径
    pub fn bubble_radius(self, width: f32) -> f32 {
        13.0 * self.unit() + clamped_width(width, self.style_scale) * 1.35
    }

    /// 【序号】【白色边框】按 width 计算二到九像素的白色描边，导出时同比缩放；返回逻辑笔宽
    pub fn outline_width(self, width: f32) -> f32 {
        (clamped_width(width, self.style_scale) * 0.22).clamp(2.0 * self.unit(), 9.0 * self.unit())
    }

    /// 【序号】【样式单位】返回有限正比例，无效比例使用一，避免独立尺寸查询出现非法范围
    fn unit(self) -> f32 {
        if self.style_scale.is_finite() && self.style_scale > 0.0 {
            self.style_scale
        } else {
            1.0
        }
    }

    /// 【序号】【视觉坐标】围绕圆心旋转未旋转 point，返回屏幕或导出中的实际逻辑坐标
    pub fn to_visual(self, point: (f32, f32)) -> (f32, f32) {
        rotated(point, self.bubble, self.rotation)
    }

    /// 【序号】【本地坐标】把实际逻辑 point 反旋转回标注空间，返回未旋转坐标
    pub fn to_local(self, point: (f32, f32)) -> (f32, f32) {
        rotated(point, self.bubble, -self.rotation)
    }

    /// 【序号】【指向尾部】按 width 计算实际旋转后三角尾部的三点；短于圆框半径的百分之四十五时返回空值
    pub fn tail(self, width: f32) -> Option<[(f32, f32); 3]> {
        if !self.valid() {
            return None;
        }
        let tip = self.to_visual(self.tip);
        let delta = (tip.0 - self.bubble.0, tip.1 - self.bubble.1);
        let length = delta.0.hypot(delta.1);
        let radius = self.bubble_radius(width);
        if !length.is_finite() || length <= radius * 0.45 {
            return None;
        }
        let direction = (delta.0 / length, delta.1 / length);
        let normal = (-direction.1, direction.0);
        let half = (radius * 0.46).clamp(8.0 * self.style_scale, 38.0 * self.style_scale);
        let base = (
            self.bubble.0 + direction.0 * radius * 0.82,
            self.bubble.1 + direction.1 * radius * 0.82,
        );
        Some([
            tip,
            (base.0 + normal.0 * half, base.1 + normal.1 * half),
            (base.0 - normal.0 * half, base.1 - normal.1 * half),
        ])
    }

    /// 【序号】【未旋转选框】按width返回生产圆框范围，不计白边与独立尖端；无效数据返回空值
    pub fn unrotated_bounds(self, width: f32) -> Option<Rect> {
        if !self.valid() {
            return None;
        }
        let radius = self.bubble_radius(width);
        Rect::from_xywh(
            self.bubble.0 - radius,
            self.bubble.1 - radius,
            radius * 2.0,
            radius * 2.0,
        )
    }

    /// 【序号】【正文边界】按 width 返回包含圆框白色边框和旋转后尖端的实际边界；无效数据返回空值
    pub fn body_bounds(self, width: f32) -> Option<Rect> {
        self.bounds_with_tip(width, self.to_visual(self.tip))
    }

    /// 【序号】【共同边界】按 width 和 tip 计算圆框及尖端的联合边界；返回有效矩形或空值
    fn bounds_with_tip(self, width: f32, tip: (f32, f32)) -> Option<Rect> {
        if !self.valid() {
            return None;
        }
        let radius = self.bubble_radius(width) + self.outline_width(width) / 2.0;
        Rect::from_ltrb(
            (self.bubble.0 - radius).min(tip.0),
            (self.bubble.1 - radius).min(tip.1),
            (self.bubble.0 + radius).max(tip.0),
            (self.bubble.1 + radius).max(tip.1),
        )
    }

    /// 【序号】【导出缩放】按正比例 scale 缩放坐标和固定尺寸，保留编号与样式；无效比例返回原标记
    pub fn scaled(self, scale: f32) -> Self {
        if !scale.is_finite() || scale <= 0.0 {
            return self;
        }
        Self {
            bubble: (self.bubble.0 * scale, self.bubble.1 * scale),
            tip: (self.tip.0 * scale, self.tip.1 * scale),
            style_scale: self.style_scale * scale,
            ..self
        }
    }

    /// 【序号】【整体平移】按 delta 移动圆心和尖端，保留旋转轴关系；非有限增量返回原标记
    pub fn translated(self, delta: (f32, f32)) -> Self {
        if !delta.0.is_finite() || !delta.1.is_finite() {
            return self;
        }
        Self {
            bubble: (self.bubble.0 + delta.0, self.bubble.1 + delta.1),
            tip: (self.tip.0 + delta.0, self.tip.1 + delta.1),
            ..self
        }
    }

    /// 【序号】【坐标映射】使用 map 映射圆心和实际尖端，反算未旋转尖端；返回保留角度与样式的新标记
    pub fn mapped(self, map: impl Fn((f32, f32)) -> (f32, f32)) -> Self {
        let bubble = map(self.bubble);
        let visual_tip = map(self.to_visual(self.tip));
        let mark = Self { bubble, ..self };
        Self {
            tip: mark.to_local(visual_tip),
            ..mark
        }
    }
}

/// 【序号】【定点旋转】把 point 绕 center 旋转 radians 弧度；返回旋转坐标，不改变输入
fn rotated(point: (f32, f32), center: (f32, f32), radians: f32) -> (f32, f32) {
    let (sin, cos) = radians.sin_cos();
    let delta = (point.0 - center.0, point.1 - center.1);
    (
        center.0 + delta.0 * cos - delta.1 * sin,
        center.1 + delta.0 * sin + delta.1 * cos,
    )
}
