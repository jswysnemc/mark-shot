//! 【截图区域识别】【像素坐标】统一原图像素与全局逻辑边界，正式选区保持独立

use crate::{
    editor::EditorState,
    features::region_detection::{PixelRect, window::Window},
};
use tiny_skia::Rect;

#[derive(Clone, Copy)]
pub(super) struct Space {
    pub bounds: Rect,
    pub scale: f64,
    pub size: (i32, i32),
}

impl Space {
    /// 【截图区域识别】【截图空间】按 state 的冻结截图建立统一原图空间，返回有效边界，不读取已标注图层
    pub fn for_editor(state: &EditorState) -> Option<Self> {
        if state.file_canvas.is_some() {
            return None;
        }
        let bounds = crate::utils::get_full_workspace_rect(&state.placements)?;
        let scale = f64::from(crate::renderer::capture_scale(
            &state.captures,
            &state.placements,
            bounds,
        ));
        let size = if state.captures.len() == 1 {
            (
                state.captures[0].pixmap.width() as i32,
                state.captures[0].pixmap.height() as i32,
            )
        } else {
            (
                (f64::from(bounds.width()) * scale).round() as i32,
                (f64::from(bounds.height()) * scale).round() as i32,
            )
        };
        Some(Self {
            bounds,
            scale,
            size,
        })
    }

    /// 【截图区域识别】【指针映射】按 point 返回原图内整数像素坐标，未覆盖输出或越界点返回空值
    pub fn point(self, point: (f64, f64)) -> Option<(i32, i32)> {
        let x = (point.0 - f64::from(self.bounds.left())) * self.scale;
        let y = (point.1 - f64::from(self.bounds.top())) * self.scale;
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= f64::from(self.size.0)
            || y >= f64::from(self.size.1)
        {
            return None;
        }
        Some((x.round() as i32, y.round() as i32))
    }

    /// 【截图区域识别】【窗口映射】按 windows 裁切真实窗口并转换到原图像素，返回保留 z 层级的候选
    pub fn windows(self, windows: &[crate::backend::window_detection::Window]) -> Vec<Window> {
        windows
            .iter()
            .filter_map(|window| {
                let rect = window.rect.intersect(&self.bounds)?;
                let left = ((f64::from(rect.left()) - f64::from(self.bounds.left())) * self.scale)
                    .round() as i32;
                let top = ((f64::from(rect.top()) - f64::from(self.bounds.top())) * self.scale)
                    .round() as i32;
                let right = ((f64::from(rect.right()) - f64::from(self.bounds.left())) * self.scale)
                    .round() as i32;
                let bottom = ((f64::from(rect.bottom()) - f64::from(self.bounds.top()))
                    * self.scale)
                    .round() as i32;
                Some(Window {
                    rect: PixelRect::new(left, top, right - left, bottom - top),
                    z_order: window.z_order,
                })
            })
            .collect()
    }

    /// 【截图区域识别】【全局边界】按 rect 返回候选对应的全局逻辑矩形，空矩形返回空值
    pub fn global(self, rect: PixelRect) -> Option<Rect> {
        Rect::from_xywh(
            (f64::from(self.bounds.left()) + f64::from(rect.left) / self.scale) as f32,
            (f64::from(self.bounds.top()) + f64::from(rect.top) / self.scale) as f32,
            (f64::from(rect.width()) / self.scale) as f32,
            (f64::from(rect.height()) / self.scale) as f32,
        )
    }
}
