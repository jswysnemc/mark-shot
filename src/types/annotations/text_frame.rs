//! 【文字标注】【矩形接口】原图选框、存储矩形及实际字形损坏范围各自保留独立职责

use super::{Annotation, AnnotationShape};
use tiny_skia::Rect;

impl Annotation {
    /// 【文字标注】【存储读取】返回当前逻辑坐标中的原始矩形，缺少尺寸使用原图自然选框，无参数
    pub fn text_stored_bounds(&self) -> Option<Rect> {
        let AnnotationShape::Text {
            start,
            frame,
            appearance,
            ..
        } = &self.shape
        else {
            return None;
        };
        Rect::from_xywh(
            start.0,
            start.1,
            frame.width.unwrap_or(self.bbox.width()),
            frame.height.unwrap_or(self.bbox.height()),
        )
        .filter(|bounds| !bounds.is_empty())
        .or_else(|| {
            let anchor = frame.anchor.unwrap_or(*start);
            Rect::from_xywh(
                anchor.0,
                anchor.1,
                360.0 * appearance.size_scale,
                140.0 * appearance.size_scale,
            )
        })
    }

    /// 【文字标注】【字形读取】返回当前比例的字形范围，未建立字体缓存时使用选框，无参数
    pub fn text_painted_bounds(&self) -> Rect {
        match &self.shape {
            AnnotationShape::Text { frame, .. } => frame.painted.unwrap_or(self.bbox),
            _ => self.bbox,
        }
    }

    /// 【文字标注】【状态比较】按other比较真实编辑状态，文字缓存和相同矩形的默认表示不构成历史变化
    pub(crate) fn same_edit_state(&self, other: &Self) -> bool {
        if !matches!(self.shape, AnnotationShape::Text { .. })
            || !matches!(other.shape, AnnotationShape::Text { .. })
        {
            return self == other;
        }
        let normalized = |annotation: &Self| {
            let mut result = annotation.clone();
            if let AnnotationShape::Text { start, frame, .. } = &mut result.shape {
                frame.width = Some(frame.width.unwrap_or(annotation.bbox.width()));
                frame.height = Some(frame.height.unwrap_or(annotation.bbox.height()));
                frame.anchor = Some(frame.anchor.unwrap_or(*start));
                frame.painted = None;
            }
            result.bbox = Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap();
            result
        };
        normalized(self) == normalized(other)
    }
}
