//! 【画布导航】【正文命中入口】窗口容差与原图标注分离，首次选择和专用工具共享当前持续视图

use crate::{
    editor::EditorState,
    features::{annotation_selection::body_hit, annotation_style::window::WindowUnits, ruler},
    types::{Annotation, AnnotationShape, SelectionHandle},
};

impl EditorState {
    /// 【画布导航】【命中单位】无额外参数，返回当前窗口逻辑像素到稳定画布的比例，不包含设备像素比例
    pub(crate) fn hit_window_units(&self) -> WindowUnits {
        WindowUnits::new(1.0 / self.canvas_transform().scale as f32)
    }

    /// 【画布导航】【首次正文命中】按annotation与point返回首次选择结果，point使用稳定画布坐标
    pub(crate) fn canvas_body_hit(&self, annotation: &Annotation, point: (f64, f64)) -> bool {
        body_hit::initial(
            annotation,
            (point.0 as f32, point.1 as f32),
            self.hit_window_units(),
        )
    }

    /// 【画布导航】【专用正文命中】按annotation与point返回序号或放大镜的生产专用操作，point使用稳定坐标
    pub(crate) fn canvas_special_drag(
        &self,
        annotation: &Annotation,
        point: (f64, f64),
    ) -> SelectionHandle {
        body_hit::special_drag(
            annotation,
            (point.0 as f32, point.1 as f32),
            self.hit_window_units(),
        )
    }

    /// 【画布导航】【尺子命中】按annotation、point与selected返回尺子操作，interior_moves区分选择工具移动和尺子工具新建职责
    pub(crate) fn canvas_ruler_handle(
        &self,
        annotation: &Annotation,
        point: (f64, f64),
        selected: bool,
        interior_moves: bool,
    ) -> SelectionHandle {
        let AnnotationShape::Ruler { measurement } = annotation.shape else {
            return SelectionHandle::None;
        };
        let window = self.hit_window_units();
        let point = (point.0 as f32, point.1 as f32);
        if selected
            && self
                .canvas_ruler_connector(annotation)
                .map(|(_, end)| end)
                .is_some_and(|handle| {
                    (handle.0 - point.0).hypot(handle.1 - point.1) <= window.canvas(11.2)
                })
        {
            return SelectionHandle::Rotate;
        }
        let handle = ruler::drag::body_hit(measurement, point, window.canvas(8.0), interior_moves);
        if handle != SelectionHandle::None {
            return handle;
        }
        if selected
            && measurement.mode() == ruler::RulerMode::Angle
            && let Some(bounds) = ruler::drag::window_frame(measurement, window)
        {
            return body_hit::bounds_drag(bounds, point, window);
        }
        SelectionHandle::None
    }
}
