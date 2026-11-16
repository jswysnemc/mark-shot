//! 【框形属性】【编辑事务】统一工具默认值、选中标注参数和撤销记录

use super::{
    EditorState,
    edits::{active_annotation_idx, commit_settings_change},
};
use crate::tools::Tool;
use crate::types::AnnotationShape;

/// 【框形属性】【矩形目标】按选中标注或当前工具返回半径是否写入矩形专用默认值
fn rectangle_target(state: &EditorState) -> bool {
    active_annotation_idx(state).map_or(state.selected_tool == Tool::Rectangle, |index| {
        matches!(
            state.annotations[index].shape,
            AnnotationShape::Rectangle { .. }
        )
    })
}

/// 【框形属性】【半径读取】返回 state 当前标注或默认工具的原图像素半径
pub fn radius(state: &EditorState) -> f32 {
    if let Some(index) = active_annotation_idx(state) {
        match &state.annotations[index].shape {
            AnnotationShape::Rectangle { radius, .. } | AnnotationShape::Marker { radius, .. } => {
                return *radius;
            }
            AnnotationShape::Magnifier { magnifier } => return magnifier.radius,
            AnnotationShape::Redact { filter } => return filter.radius,
            _ => {}
        }
    }
    if state.selected_tool == Tool::Rectangle {
        state.tool_settings.rectangle_radius
    } else {
        state.tool_settings.corner_radius
    }
}

/// 【框形属性】【半径写入】将 value 应用到 state 的工具与标注；record 控制撤销，dirty 返回重绘范围
pub fn set_radius(state: &mut EditorState, value: f32, record: bool, dirty: &mut u32) {
    let value = crate::features::shape::radius(value);
    let rectangle = rectangle_target(state);
    commit_settings_change(
        state,
        radius(state) != value,
        record,
        |tools| {
            if rectangle {
                tools.rectangle_radius = value;
            } else {
                tools.corner_radius = value;
            }
        },
        |annotation| match &mut annotation.shape {
            AnnotationShape::Rectangle { radius, .. } | AnnotationShape::Marker { radius, .. } => {
                *radius = value
            }
            AnnotationShape::Magnifier { magnifier } => magnifier.radius = value,
            AnnotationShape::Redact { filter } => filter.radius = value,
            _ => {}
        },
        dirty,
    );
}

/// 【框形属性】【等比写入】将 enabled 写入 state；开启时保留起点并调整终点，dirty 返回重绘范围
pub fn set_ratio(state: &mut EditorState, enabled: bool, dirty: &mut u32) {
    let space = crate::tools::drawing_space::DrawingSpace::from_editor(state);
    commit_settings_change(
        state,
        true,
        true,
        |tools| tools.fixed_ratio = enabled,
        |annotation| {
            if let AnnotationShape::Rectangle {
                start,
                end,
                fixed_ratio,
                ..
            }
            | AnnotationShape::Circle {
                start,
                end,
                fixed_ratio,
                ..
            } = &mut annotation.shape
            {
                *fixed_ratio = enabled;
                if enabled {
                    *end = space.map_or_else(
                        || crate::features::shape::square_end(*start, *end),
                        |space| space.rectangle_end(*start, *end, true),
                    );
                }
            }
        },
        dirty,
    );
}
