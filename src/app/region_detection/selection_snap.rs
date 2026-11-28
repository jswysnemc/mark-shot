//! 【截图选区】【双边吸附】仅修正正在拖动的边，横向和纵向可以对齐不同的识别区域

use super::snap_targets::{SnapAxes, Targets};
use crate::{
    editor::EditorState,
    types::{SelectionHandle, SignedRect},
};

/// 【截图选区】【拖动边界】按 state、handle 和指针 pointer 对齐 raw 的活动边；返回有符号选区，保留反向拖动及对边锚点
pub(crate) fn align(
    state: &mut EditorState,
    mut raw: SignedRect,
    handle: SelectionHandle,
    pointer: (f64, f64),
) -> SignedRect {
    use SelectionHandle::*;
    let ratio = state.selection.aspect_ratio as f32;
    let Some(targets) = Targets::for_selection(state, (pointer.0 as f32, pointer.1 as f32)) else {
        return raw;
    };
    let left = matches!(handle, Move | Left | TopLeft | BottomLeft);
    let right = matches!(handle, Move | Right | TopRight | BottomRight);
    let top = matches!(handle, Move | Top | TopLeft | TopRight);
    let bottom = matches!(handle, Move | Bottom | BottomLeft | BottomRight);
    let mut axes = SnapAxes::default();
    let mut inspect = |projection, area| axes.consider(projection, area);

    // 1. 【截图选区】【活动边匹配】单边只检查法向距离；拖动角点时，也检查尚未发生边段重叠的邻近角点
    for (moving_x, horizontal) in [(left, raw.left), (right, raw.right)] {
        if moving_x {
            targets.visit_edge(
                (horizontal, raw.top),
                (horizontal, raw.bottom),
                &mut inspect,
            );
            for (moving_y, vertical) in [(top, raw.top), (bottom, raw.bottom)] {
                if moving_y {
                    targets.visit((horizontal, vertical), &mut inspect);
                }
            }
        }
    }
    for (moving_y, vertical) in [(top, raw.top), (bottom, raw.bottom)] {
        if moving_y {
            targets.visit_edge((raw.left, vertical), (raw.right, vertical), &mut inspect);
        }
    }
    let projection = axes.projection();
    let (mut horizontal, mut vertical) = projection.apply((0.0, 0.0));

    // 2. 【截图选区】【比例优先】锁定比例的角点只有一个自由度，按需要移动较少的候选边联动另一轴
    if handle != Move && ratio > 0.0 && (left || right) && (top || bottom) {
        let slope = if left == top { ratio } else { -ratio };
        if projection.horizontal.is_some()
            && (projection.vertical.is_none() || horizontal.abs() <= (vertical * slope).abs())
        {
            vertical = horizontal / slope;
        } else {
            horizontal = vertical * slope;
        }
    }
    if left {
        raw.left += horizontal;
    }
    if right {
        raw.right += horizontal;
    }
    if top {
        raw.top += vertical;
    }
    if bottom {
        raw.bottom += vertical;
    }
    raw
}
