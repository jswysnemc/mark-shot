//! 【选区尺寸】【坐标换算】把统一图像像素与全局逻辑选区互相映射，原图视口保留独立缩放

use super::EditorState;
use crate::features::selection_size::geometry;
use tiny_skia::Rect;

#[derive(Clone, Copy, Debug)]
pub struct ImageSpace {
    pub bounds: Rect,
    pub scale: f64,
    pub size: (i32, i32),
}

impl ImageSpace {
    /// 【选区尺寸】【图像空间】按 state 返回原图或冻结桌面的像素边界，没有有效图像时返回空值
    pub fn for_editor(state: &EditorState) -> Option<Self> {
        if let Some(file) = &state.file_canvas {
            return Some(Self {
                bounds: file.viewport,
                scale: f64::from(file.original.width()) / f64::from(file.viewport.width()),
                size: (file.original.width() as i32, file.original.height() as i32),
            });
        }
        let bounds = crate::utils::get_full_workspace_rect(&state.placements)?;
        let scale = f64::from(crate::renderer::capture_scale(
            &state.captures,
            &state.placements,
            state.selection.zone.unwrap_or(bounds),
        ));
        Some(Self {
            bounds,
            scale,
            size: (
                (f64::from(bounds.width()) * scale).ceil() as i32,
                (f64::from(bounds.height()) * scale).ceil() as i32,
            ),
        })
    }

    /// 【选区尺寸】【像素映射】按 global 返回图像内归一化像素矩形，图像外或空选区返回空值
    pub fn image(self, global: Rect) -> Option<geometry::Rect> {
        let clipped = global.intersect(&self.bounds)?;
        Some(self.raw(clipped))
    }

    /// 【选区尺寸】【未裁切映射】按 global 返回图像像素矩形，保留越界拖动供比例约束判断
    pub fn raw(self, global: Rect) -> geometry::Rect {
        geometry::Rect {
            x: (f64::from(global.left()) - f64::from(self.bounds.left())) * self.scale,
            y: (f64::from(global.top()) - f64::from(self.bounds.top())) * self.scale,
            width: f64::from(global.width()) * self.scale,
            height: f64::from(global.height()) * self.scale,
        }
    }

    /// 【选区尺寸】【端点映射】按 point 返回图像坐标，不截断拖动端点，约束逻辑负责边界
    pub fn point(self, point: (f64, f64)) -> (f64, f64) {
        (
            (point.0 - f64::from(self.bounds.left())) * self.scale,
            (point.1 - f64::from(self.bounds.top())) * self.scale,
        )
    }

    /// 【选区尺寸】【逻辑映射】按 image 返回全局逻辑矩形，空矩形或无效比例返回空值
    pub fn global(self, image: geometry::Rect) -> Option<Rect> {
        Rect::from_xywh(
            (f64::from(self.bounds.left()) + image.x / self.scale) as f32,
            (f64::from(self.bounds.top()) + image.y / self.scale) as f32,
            (image.width / self.scale) as f32,
            (image.height / self.scale) as f32,
        )
    }
}
