//! 【放大镜】【属性事务】更新倍率和框形，复用持久标注的重绘及撤销流程

use crate::editor::EditorState;
use crate::editor::edits::{active_annotation_idx, commit_settings_change};
use crate::features::magnifier::{MagnifierShape, clamped_zoom};
use crate::types::AnnotationShape;

/// 【放大镜】【当前倍率】返回 state 的已选放大镜倍率；没有已选放大镜时返回工具默认倍率
pub fn current_zoom(state: &EditorState) -> f32 {
    active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .and_then(|annotation| match annotation.shape {
            AnnotationShape::Magnifier { magnifier } => Some(magnifier.zoom),
            _ => None,
        })
        .unwrap_or(state.tool_settings.magnifier_zoom)
}

/// 【放大镜】【当前框形】返回 state 的已选放大镜框形；没有已选放大镜时返回工具默认框形
pub fn current_shape(state: &EditorState) -> MagnifierShape {
    active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .and_then(|annotation| match annotation.shape {
            AnnotationShape::Magnifier { magnifier } => Some(magnifier.shape),
            _ => None,
        })
        .unwrap_or(state.tool_settings.magnifier_shape)
}

/// 【放大镜】【倍率提交】把 zoom 用于 state 的默认值和已选双框；record_undo 决定历史，dirty_mask 返回重绘显示器，无返回值
pub fn set_zoom(state: &mut EditorState, zoom: f32, record_undo: bool, dirty_mask: &mut u32) {
    let zoom = clamped_zoom(zoom);
    let changed = (current_zoom(state) - zoom).abs() > f32::EPSILON;
    commit_settings_change(
        state,
        changed,
        record_undo,
        |settings| settings.magnifier_zoom = zoom,
        |annotation| {
            if let AnnotationShape::Magnifier { magnifier } = &mut annotation.shape {
                *magnifier = magnifier.with_zoom(zoom);
            }
        },
        dirty_mask,
    );
}

/// 【放大镜】【框形提交】把 shape 用于 state 的默认值和已选双框；dirty_mask 返回重绘显示器，只为实际形状变化记录撤销，无返回值
pub fn set_shape(state: &mut EditorState, shape: MagnifierShape, dirty_mask: &mut u32) {
    let changed = current_shape(state) != shape;
    commit_settings_change(
        state,
        changed,
        true,
        |settings| settings.magnifier_shape = shape,
        |annotation| {
            if let AnnotationShape::Magnifier { magnifier } = &mut annotation.shape {
                *magnifier = magnifier.with_shape(shape);
            }
        },
        dirty_mask,
    );
}
