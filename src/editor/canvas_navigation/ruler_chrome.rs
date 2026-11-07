//! 【画布导航】【尺子允许范围】选区投影与完整工作区求交，输入和绘制读取相同范围

use crate::{
    editor::EditorState,
    features::ruler::window_chrome,
    tools::drawing_space::DrawingSpace,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::Rect;

impl EditorState {
    /// 【画布导航】【尺子可见范围】无参数，返回当前可见选区的稳定坐标；交集为空时保留原版选区回退
    pub(crate) fn ruler_allowed_bounds(&self) -> Option<Rect> {
        let space = DrawingSpace::from_editor(self)?;
        let selection = self
            .selection
            .zone
            .unwrap_or(space.bounds)
            .intersect(&space.bounds)?;
        let screen = self
            .navigation
            .window
            .or_else(|| crate::utils::get_full_workspace_rect(&self.placements))?;
        let view = self.canvas_transform();
        let first = view.to_canvas((f64::from(screen.left()), f64::from(screen.top())));
        let last = view.to_canvas((f64::from(screen.right()), f64::from(screen.bottom())));
        let visible =
            Rect::from_ltrb(first.0 as f32, first.1 as f32, last.0 as f32, last.1 as f32)?;
        Some(
            selection
                .intersect(&visible)
                .filter(|rect| !rect.is_empty())
                .unwrap_or(selection),
        )
    }

    /// 【画布导航】【尺子连接点】按annotation返回当前稳定坐标的生产连接点，非尺子返回空值
    pub(crate) fn canvas_ruler_connector(
        &self,
        annotation: &Annotation,
    ) -> Option<((f32, f32), (f32, f32))> {
        let AnnotationShape::Ruler { measurement } = annotation.shape else {
            return None;
        };
        window_chrome::in_projection(
            measurement,
            annotation.stroke_width,
            self.ruler_allowed_bounds(),
            self.canvas_transform(),
        )
    }

    /// 【画布导航】【尺子损坏范围】按annotation和selected返回当前窗口正文与控件范围，非尺子返回空值
    pub(crate) fn ruler_window_damage(
        &self,
        annotation: &Annotation,
        selected: bool,
    ) -> Option<Rect> {
        let AnnotationShape::Ruler { measurement } = annotation.shape else {
            return None;
        };
        let window = self.hit_window_units();
        let body = crate::renderer::ruler_chrome::bounds(
            measurement,
            annotation.stroke_width,
            self.ruler_allowed_bounds(),
            window,
            self.canvas_transform(),
        )?;
        if !selected {
            return Some(body);
        }
        let frame = crate::features::ruler::drag::window_frame(measurement, window)?
            .outset(window.canvas(8.0), window.canvas(8.0))?;
        let (_, handle) = self.canvas_ruler_connector(annotation)?;
        let handle = Rect::from_xywh(
            handle.0 - window.canvas(8.0),
            handle.1 - window.canvas(8.0),
            window.canvas(16.0),
            window.canvas(16.0),
        )?;
        body.join(&frame)?.join(&handle)
    }
}
