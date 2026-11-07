//! 【画布导航】【控制点命中】屏幕容差独立于持续倍率，持久标注仍在稳定坐标中编辑

use crate::{
    editor::EditorState,
    features::annotation_selection::rotation,
    types::{Annotation, AnnotationShape, SelectionHandle},
};

impl EditorState {
    /// 【画布导航】【群组控制点】按bounds与point返回共同控制点，point使用稳定坐标，命中容差保持屏幕逻辑单位
    pub(crate) fn canvas_collection_handle(
        &self,
        bounds: tiny_skia::Rect,
        point: (f64, f64),
    ) -> SelectionHandle {
        let view = self.canvas_transform();
        if view.is_identity() {
            return crate::features::annotation_collection::geometry::hit_test(bounds, point);
        }
        let screen = view.to_screen(point);
        let (top, handle) =
            crate::features::annotation_collection::geometry::rotation_handle(bounds);
        let handle = rotation::projected(top, handle, 1.0 / view.scale as f32);
        let rotation = view.to_screen((f64::from(handle.0), f64::from(handle.1)));
        if (rotation.0 - screen.0).hypot(rotation.1 - screen.1) <= 11.2 {
            return SelectionHandle::Rotate;
        }
        let Some(bounds) = view.rect(bounds) else {
            return SelectionHandle::None;
        };
        let handle = crate::features::annotation_collection::geometry::hit_test(bounds, screen);
        if handle == SelectionHandle::Rotate {
            SelectionHandle::None
        } else {
            handle
        }
    }

    /// 【画布导航】【标注控制点】按annotation与point返回生产控制点或正文操作，point使用稳定画布坐标，窗口容差不随持续倍率变化
    pub(crate) fn canvas_handle(
        &self,
        annotation: &Annotation,
        point: (f64, f64),
    ) -> SelectionHandle {
        let window = self.hit_window_units();
        let position = (point.0 as f32, point.1 as f32);
        if matches!(annotation.shape, AnnotationShape::Ruler { .. }) {
            return self.canvas_ruler_handle(
                annotation,
                point,
                true,
                self.selected_tool != crate::tools::Tool::Ruler,
            );
        }
        // 1. 【画布导航】【已选命中】旋转把手优先，正文和控制点使用当前窗口容差
        if rotation::connector(annotation)
            .map(|(start, end)| rotation::projected(start, end, window.canvas(1.0)))
            .is_some_and(|handle| {
                (handle.0 - position.0).hypot(handle.1 - position.1) <= window.canvas(11.2)
            })
        {
            return SelectionHandle::Rotate;
        }
        crate::features::annotation_selection::body_hit::selected(annotation, position, window)
    }
}
