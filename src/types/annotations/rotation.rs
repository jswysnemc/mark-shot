//! 【普通标注】【坐标接口】原始边界保持未旋转，使用独立角度转换绘制及鼠标坐标

use super::{Annotation, AnnotationShape};
use crate::features::{annotation_rotation::geometry, annotation_selection::chrome};
use tiny_skia::Rect;

impl Annotation {
    /// 【普通标注】【模型识别】返回当前形状是否使用通用旋转，专用工具继续使用自身模型，无参数
    pub fn uses_generic_rotation(&self) -> bool {
        if self.shape.line().is_some() && self.rotation_degrees != 0.0 {
            return true;
        }
        matches!(
            self.shape,
            AnnotationShape::Rectangle { .. }
                | AnnotationShape::Circle { .. }
                | AnnotationShape::Marker { .. }
                | AnnotationShape::Pen { .. }
                | AnnotationShape::Highlighter { .. }
                | AnnotationShape::Text { .. }
        )
    }

    /// 【普通标注】【旋转轴心】返回正文未旋转选框中心，空几何使用已计算边界，无参数
    pub fn rotation_center(&self) -> geometry::Point {
        geometry::center(chrome::bounds(self).unwrap_or(self.bbox))
    }

    /// 【普通标注】【反向坐标】按 point 返回正文未旋转坐标，专用工具原样返回输入点
    pub fn to_unrotated(&self, point: geometry::Point) -> geometry::Point {
        if self.uses_generic_rotation() {
            geometry::rotated(point, self.rotation_center(), -self.rotation_degrees)
        } else {
            point
        }
    }

    /// 【普通标注】【可见坐标】按 point 返回旋转后的正文坐标，专用工具原样返回输入点
    pub fn to_visual(&self, point: geometry::Point) -> geometry::Point {
        if self.uses_generic_rotation() {
            geometry::rotated(point, self.rotation_center(), self.rotation_degrees)
        } else {
            point
        }
    }

    /// 【普通标注】【可见范围】按 bounds 返回旋转后的轴对齐范围，专用工具原样返回已计算矩形
    pub fn rotated_bounds(&self, bounds: Rect) -> Rect {
        if self.uses_generic_rotation() {
            geometry::bounds(bounds, self.rotation_center(), self.rotation_degrees)
        } else {
            bounds
        }
    }

    /// 【普通标注】【可见选框】返回普通标注顺时针四角，不包含阴影和额外命中留量，无参数
    pub fn generic_frame_corners(&self) -> Option<[geometry::Point; 4]> {
        self.uses_generic_rotation().then_some(())?;
        Some(geometry::corners(
            chrome::bounds(self)?,
            self.rotation_center(),
            self.rotation_degrees,
        ))
    }

    /// 【普通标注】【旋转把手】返回普通标注的上边连接点及旋转点，专用工具和空几何返回空值，无参数
    pub fn generic_rotation_handle(&self) -> Option<(geometry::Point, geometry::Point)> {
        self.uses_generic_rotation().then_some(())?;
        Some(geometry::handle(
            chrome::bounds(self)?,
            self.rotation_center(),
            self.rotation_degrees,
        ))
    }
}
