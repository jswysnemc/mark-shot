//! 【标注多选】【控件衔接】群组删除按钮使用轴对齐联合边界，绘制数据采用稳定编号

use crate::{editor::EditorState, features::annotation_selection::chrome};
use tiny_skia::Rect;

pub(crate) struct Overlay<'a> {
    pub ids: &'a [u64],
    pub bounds: Option<Rect>,
    pub marquee: Option<Rect>,
    pub dragging: bool,
    pub canvas: Rect,
}

/// 【标注多选】【删除位置】按state与monitor返回群组右上方20像素按钮，成员在该显示器完全不可见返回空值
pub(crate) fn delete_rect(state: &EditorState, monitor: usize) -> Option<Rect> {
    let bounds = state.collection_bounds()?;
    let placement = state.placements.get(monitor)?;
    let local = state.canvas_local_rect(bounds, monitor)?;
    local.intersect(&placement.rect()?)?;
    Some(chrome::delete_rect(
        local,
        (local.right(), local.top()),
        false,
        (placement.size.0 as f32, placement.size.1 as f32),
    ))
}
