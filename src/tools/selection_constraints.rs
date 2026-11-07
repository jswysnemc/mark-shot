//! 【选区尺寸】【拖动约束】在图像像素坐标应用比例，窗口坐标只负责指针与画布布局

use crate::{
    editor::{EditorState, selection_geometry::ImageSpace},
    features::selection_size::geometry::{self, Handle},
    types::SelectionHandle,
};
use tiny_skia::Rect;

/// 【选区尺寸】【创建约束】按 state、anchor 和 pointer 返回当前锁定比例或 Shift 正方形选区，非法图像返回自由矩形
pub(super) fn create(state: &EditorState, anchor: (f64, f64), pointer: (f64, f64)) -> Option<Rect> {
    let ratio = if state.selection.aspect_ratio > 0.0 {
        state.selection.aspect_ratio
    } else if state.input.shift {
        1.0
    } else {
        0.0
    };
    if ratio <= 0.0 {
        let free = geometry::Rect::from_corners(anchor, pointer);
        return Rect::from_xywh(
            free.x as f32,
            free.y as f32,
            free.width as f32,
            free.height as f32,
        );
    }
    let space = ImageSpace::for_editor(state)?;
    space.global(geometry::create(
        space.point(anchor),
        space.point(pointer),
        ratio,
        space.size,
    ))
}

/// 【选区尺寸】【控制点映射】按 handle 返回原版八个边角方向，移动和其他控制点返回不约束
fn handle(handle: SelectionHandle) -> Handle {
    match handle {
        SelectionHandle::Left => Handle::Left,
        SelectionHandle::Right => Handle::Right,
        SelectionHandle::Top => Handle::Top,
        SelectionHandle::Bottom => Handle::Bottom,
        SelectionHandle::TopLeft => Handle::TopLeft,
        SelectionHandle::TopRight => Handle::TopRight,
        SelectionHandle::BottomLeft => Handle::BottomLeft,
        SelectionHandle::BottomRight => Handle::BottomRight,
        _ => Handle::Other,
    }
}

/// 【选区尺寸】【调整约束】按 state、before、adjusted 和 selected 返回保持对边或对角锚点的比例选区，移动不约束
pub(super) fn adjust(
    state: &EditorState,
    before: Rect,
    adjusted: Rect,
    selected: SelectionHandle,
) -> Rect {
    if state.selection.aspect_ratio <= 0.0 || selected == SelectionHandle::Move {
        return adjusted;
    }
    let Some(space) = ImageSpace::for_editor(state) else {
        return before;
    };
    let result = geometry::adjust(
        space.raw(before),
        handle(selected),
        space.raw(adjusted),
        state.selection.aspect_ratio,
        space.size,
    );
    space.global(result).unwrap_or(before)
}
