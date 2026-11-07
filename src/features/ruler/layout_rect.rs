//! 【尺子】【布局矩形】双精度窗口运算共用于数字条带与测量说明

use tiny_skia::Rect;

/// 【尺子】【双精度矩形】仅用于布局判断，最终渲染坐标仍使用项目统一矩形
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExactRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl ExactRect {
    /// 【尺子】【范围合并】按other返回两个矩形的完整外接范围，退化矩形遵循原版空范围规则
    pub(crate) fn joined(self, other: Self) -> Self {
        if self.width <= 0.0 || self.height <= 0.0 {
            return other;
        }
        if other.width <= 0.0 || other.height <= 0.0 {
            return self;
        }
        let left = self.left.min(other.left);
        let top = self.top.min(other.top);
        Self {
            left,
            top,
            width: self.right().max(other.right()) - left,
            height: self.bottom().max(other.bottom()) - top,
        }
    }

    /// 【尺子】【矩形留白】按gap返回四边扩展的完整矩形，保留原版adjusted逐项加减的运算顺序
    pub(crate) fn outset(self, gap: f64) -> Self {
        Self {
            left: self.left - gap,
            top: self.top - gap,
            width: self.width + gap + gap,
            height: self.height + gap + gap,
        }
    }

    /// 【尺子】【矩形重叠】按other判断正面积交集，接触边界返回false
    pub(crate) fn intersects(self, other: Self) -> bool {
        self.left < other.right()
            && self.right() > other.left
            && self.top < other.bottom()
            && self.bottom() > other.top
    }

    /// 【尺子】【完整旋转范围】按center和degrees返回原版矩阵变换后的外接矩形
    pub(crate) fn rotated(self, center: (f64, f64), degrees: f32) -> Self {
        if degrees.abs() <= f32::EPSILON {
            return self;
        }
        let (sin, cos) = f64::from(degrees).to_radians().sin_cos();
        let translation = (
            center.0 - cos * center.0 + sin * center.1,
            center.1 - sin * center.0 - cos * center.1,
        );
        let points = [
            (self.left, self.top),
            (self.right(), self.top),
            (self.right(), self.bottom()),
            (self.left, self.bottom()),
        ]
        .map(|(x, y)| {
            (
                cos * x - sin * y + translation.0,
                sin * x + cos * y + translation.1,
            )
        });
        let left = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let top = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        Self {
            left,
            top,
            width: points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) - left,
            height: points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) - top,
        }
    }
    /// 【尺子】【精确读取】按rect返回双精度布局坐标
    pub(crate) fn from_rect(rect: Rect) -> Self {
        Self {
            left: f64::from(rect.left()),
            top: f64::from(rect.top()),
            width: f64::from(rect.width()),
            height: f64::from(rect.height()),
        }
    }
    /// 【尺子】【投影转换】按projection和inverse返回窗口或稳定坐标矩形，空投影保持原坐标
    pub(crate) fn projected(
        self,
        projection: Option<crate::editor::canvas_navigation::transform::ViewTransform>,
        inverse: bool,
    ) -> Self {
        let Some(view) = projection else {
            return self;
        };
        if inverse {
            Self {
                left: (self.left - view.translation.0) / view.scale,
                top: (self.top - view.translation.1) / view.scale,
                width: self.width / view.scale,
                height: self.height / view.scale,
            }
        } else {
            let left = self.left * view.scale + view.translation.0;
            let top = self.top * view.scale + view.translation.1;
            Self {
                left,
                top,
                width: self.right() * view.scale + view.translation.0 - left,
                height: self.bottom() * view.scale + view.translation.1 - top,
            }
        }
    }
    /// 【尺子】【右边界】无参数，返回完整右边界
    pub(crate) fn right(self) -> f64 {
        self.left + self.width
    }
    /// 【尺子】【下边界】无参数，返回完整下边界
    pub(crate) fn bottom(self) -> f64 {
        self.top + self.height
    }
    /// 【尺子】【渲染转换】无参数，返回单精度渲染矩形；非法尺寸返回空值
    pub(crate) fn rect(self) -> Option<Rect> {
        Rect::from_xywh(
            self.left as f32,
            self.top as f32,
            self.width as f32,
            self.height as f32,
        )
    }
    /// 【尺子】【完整容纳】按bounds、body、rotation和horizontal返回是否容纳完整条带，计算顺序遵循原版矩阵
    pub(crate) fn fits(self, bounds: Self, body: Self, rotation: f32, horizontal: bool) -> bool {
        if rotation.abs() <= f32::EPSILON {
            return if horizontal {
                self.top >= bounds.top && self.bottom() <= bounds.bottom()
            } else {
                self.left >= bounds.left && self.right() <= bounds.right()
            };
        }
        let center = (body.left + body.width / 2.0, body.top + body.height / 2.0);
        let (sin, cos) = f64::from(rotation).to_radians().sin_cos();
        let translation = (
            center.0 - cos * center.0 + sin * center.1,
            center.1 - sin * center.0 - cos * center.1,
        );
        [
            (self.left, self.top),
            (self.right(), self.top),
            (self.right(), self.bottom()),
            (self.left, self.bottom()),
        ]
        .into_iter()
        .all(|(x, y)| {
            let x1 = cos * x - sin * y + translation.0;
            let y1 = sin * x + cos * y + translation.1;
            x1 >= bounds.left && x1 <= bounds.right() && y1 >= bounds.top && y1 <= bounds.bottom()
        })
    }
}
