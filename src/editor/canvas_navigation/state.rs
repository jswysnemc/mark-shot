//! 【画布导航】【持续状态】多个输出共用工作区视图，手势和Ctrl计时不进入标注历史

use super::transform::ViewTransform;
use crate::{
    editor::EditorState,
    features::canvas_navigation::{model::Navigation, policy::Policy, scrollbars::Scrollbars},
};
use std::time::Instant;
use tiny_skia::Rect;

#[derive(Clone, Copy)]
pub(crate) struct Pan {
    pub center: (f64, f64),
    pub start: (f64, f64),
}

#[derive(Clone, Copy)]
pub(crate) struct SliderDrag {
    pub horizontal: bool,
    pub anchor: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct FrameKey {
    pub baked_background: bool,
    pub view: ViewTransform,
    pub selection: Option<Rect>,
    pub offset: (f32, f32),
    pub scale: f32,
    pub radius: f32,
    pub size: (i32, i32),
}

#[derive(Default)]
pub(crate) struct State {
    pub view: Option<Navigation>,
    pub frames: Vec<Option<FrameKey>>,
    pub window: Option<Rect>,
    pub reference: Option<crate::features::canvas_navigation::coordinates::Rect>,
    pub bars: Scrollbars,
    pub bar_hover: Option<bool>,
    pub pan: Option<Pan>,
    pub slider: Option<SliderDrag>,
    pub ctrl_tap: Option<Instant>,
    pub preview: Option<crate::app::canvas_navigation::preview::Preview>,
    pub preview_fonts: Option<cosmic_text::FontSystem>,
}

impl EditorState {
    /// 【画布导航】【导航策略】无参数，返回图片入口及截图编辑阶段的导航职责
    pub(crate) fn navigation_policy(&self) -> Policy {
        Policy {
            image_enabled: self.file_canvas.is_some(),
            editing: self.selection.zone.is_some(),
        }
    }

    /// 【画布导航】【状态准备】无参数，按原图或完整工作区初始化共享视图，返回是否具备有效尺寸
    pub(crate) fn ensure_navigation(&mut self) -> bool {
        let available = self.navigation_policy().available();
        if let Some(view) = &mut self.navigation.view {
            view.refresh(available);
            return true;
        }
        let Some(mut window) = crate::utils::get_full_workspace_rect(&self.placements) else {
            return false;
        };
        if self.image_window.enabled {
            window = crate::app::image_window::layout::viewport((
                window.width() as u32,
                window.height() as u32,
            ));
        }
        let image = self.file_canvas.as_ref().map_or(
            (
                window.width().round() as u32,
                window.height().round() as u32,
            ),
            |file| (file.original.width(), file.original.height()),
        );
        let Some(mut view) = Navigation::new(
            image,
            (
                window.width().round() as u32,
                window.height().round() as u32,
            ),
            self.navigation_policy().available(),
        ) else {
            return false;
        };
        view.set_free_pan(self.file_canvas.is_some());
        self.navigation.reference = Some(view.projection().widget_rect);
        self.navigation.window = Some(window);
        self.navigation.view = Some(view);
        true
    }

    /// 【画布导航】【视图读取】无参数，返回稳定画布到屏幕的共享变换，未初始化和复位时返回恒等映射
    pub fn canvas_transform(&self) -> ViewTransform {
        let state = &self.navigation;
        let Some((view, window, reference)) = state
            .view
            .zip(state.window)
            .zip(state.reference)
            .map(|((a, b), c)| (a, b, c))
        else {
            return ViewTransform::default();
        };
        let rect = view.projection().widget_rect;
        if rect == reference {
            return ViewTransform::default();
        }
        let scale = rect.width / reference.width;
        ViewTransform {
            scale,
            translation: (
                f64::from(window.left()) * (1.0 - scale) + rect.x - reference.x * scale,
                f64::from(window.top()) * (1.0 - scale) + rect.y - reference.y * scale,
            ),
        }
    }

    /// 【画布导航】【画布指针】无参数，返回当前屏幕指针对应的稳定画布点，UI继续读取原有local和global
    pub fn canvas_point(&self) -> (f64, f64) {
        self.canvas_transform().to_canvas(self.input.pointer.global)
    }

    /// 【画布导航】【窗口指针】无参数，返回完整工作区的窗口局部指针，支持负坐标和跨输出移动
    pub(crate) fn navigation_point(&self) -> (f64, f64) {
        let origin = self.navigation.window.map_or((0.0, 0.0), |rect| {
            (f64::from(rect.left()), f64::from(rect.top()))
        });
        (
            self.input.pointer.global.0 - origin.0,
            self.input.pointer.global.1 - origin.1,
        )
    }

    /// 【画布导航】【局部矩形】按rect和monitor返回屏幕局部边界，不在此处裁切以保留完整控件几何
    pub(crate) fn canvas_local_rect(&self, rect: Rect, monitor: usize) -> Option<Rect> {
        let rect = self.canvas_transform().rect(rect)?;
        let offset = self.placements.get(monitor)?.offset();
        Rect::from_xywh(
            rect.x() - offset.0,
            rect.y() - offset.1,
            rect.width(),
            rect.height(),
        )
    }
}
