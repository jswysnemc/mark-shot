//! 【输入法】【组合字符几何】使用已显示文字布局计算原生输入法所需的 UTF-16 字符位置

use crate::{
    editor::EditorState,
    types::{AnnotationShape, text_input::CompositionGeometry},
};

/// 【输入法】【组合布局】按 state 与 monitor_idx 返回当前组合串字符边界矩形，位置以目标表面逻辑坐标表示，不猜测字宽
pub(super) fn snapshot(state: &EditorState, monitor_idx: usize) -> Option<CompositionGeometry> {
    let session = state.text_session.as_ref()?;
    let composition = session.composition.as_ref()?;
    let AnnotationShape::Text {
        start, appearance, ..
    } = &composition.annotation.shape
    else {
        return None;
    };
    let editor = state.text.editors.get(&composition.annotation.id)?;
    let placement = state.placements.get(monitor_idx)?;
    let padding = crate::features::text::geometry::padding(appearance.size_scale);
    let view = state.canvas_transform();
    // 1. 【输入法】【实际字形】按已排版的字符位置投影到表面，组合首字位置与当前光标位置分别保留
    Some(crate::features::text::input_method::geometry::layout(
        editor,
        composition.position,
        &composition.preedit.text,
        |x, y, height| {
            let point = view.to_screen((
                f64::from(start.0 + padding.0 + x),
                f64::from(start.1 + padding.1 + y),
            ));
            (
                (point.0 - f64::from(placement.position.0)).floor() as i32,
                (point.1 - f64::from(placement.position.1)).floor() as i32,
                1,
                (f64::from(height) * view.scale).ceil().max(1.0) as i32,
            )
        },
    ))
}
