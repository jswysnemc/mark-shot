//! 【线条】【编辑事务】处理头部、线型和骨架点，保持独立默认偏好与统一撤销快照

use crate::editor::annotation_drag::rebuild_annotation;
use crate::editor::dirty::apply_damage_rects;
use crate::editor::edits::active_annotation_idx;
use crate::editor::{DamageZone, EditorState};
use crate::features::{
    arrow::ArrowStyle,
    line::{LineStyle, Point, geometry, polyline::distance},
};
use crate::tools::Tool;
use crate::types::{Annotation, AnnotationShape};

/// 【线条】【属性可用】返回 state 当前工具或选中标注是否为直线类，不在拖动中修改持久属性
pub fn available(state: &EditorState) -> bool {
    if state.ann_drag.is_some() {
        return false;
    }
    match active_annotation_idx(state).and_then(|index| state.annotations.get(index)) {
        Some(annotation) => annotation.shape.line().is_some(),
        None => matches!(state.selected_tool, Tool::Line | Tool::Arrow),
    }
}

/// 【箭头】【属性可用】返回 state 是否可以修改箭头头部，不为普通直线显示头部选项
pub fn arrow_available(state: &EditorState) -> bool {
    available(state)
        && match active_annotation_idx(state).and_then(|index| state.annotations.get(index)) {
            Some(annotation) => matches!(annotation.shape, AnnotationShape::Arrow { .. }),
            None => state.selected_tool == Tool::Arrow,
        }
}

/// 【箭头】【当前头部】返回 state 选中箭头的头部，未选中时返回后续默认样式
pub fn current_head(state: &EditorState) -> ArrowStyle {
    match active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .map(|annotation| &annotation.shape)
    {
        Some(AnnotationShape::Arrow { style, .. }) => *style,
        _ => state.tool_settings.arrow_style,
    }
}

/// 【线条】【当前线型】返回 state 当前标注或后续工具线型，箭头和直线默认值分别保存
pub fn current_style(state: &EditorState) -> LineStyle {
    match active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .map(|annotation| &annotation.shape)
    {
        Some(AnnotationShape::Arrow { line_style, .. }) => *line_style,
        Some(AnnotationShape::Line { style, .. }) => *style,
        _ if state.selected_tool == Tool::Arrow => state.tool_settings.arrow_line_style,
        _ => state.tool_settings.line_style,
    }
}

/// 【线条】【标注事务】在 state 的 index 标注上执行 change 并更新缓存；dirty_mask 返回新旧范围，无返回值
fn change_annotation(
    state: &mut EditorState,
    index: usize,
    change: impl FnOnce(&mut Annotation),
    dirty_mask: &mut u32,
) {
    let annotation = &state.annotations[index];
    state
        .damage_rects
        .push(DamageZone::Global(annotation.damage_bbox(true)));
    state.layer_damage_rects.push(annotation.damage_bbox(false));
    state.push_undo();
    change(&mut state.annotations[index]);
    rebuild_annotation(state, index);
    state.settings_panel.dirty = true;
    apply_damage_rects(state, dirty_mask);
}

/// 【箭头】【头部修改】把 style 用于 state 当前箭头或后续默认值；dirty_mask 返回损坏区域，已有标注形成一次撤销
pub fn set_head(state: &mut EditorState, style: ArrowStyle, dirty_mask: &mut u32) {
    if !arrow_available(state) || current_head(state) == style {
        return;
    }
    if let Some(index) = active_annotation_idx(state) {
        change_annotation(
            state,
            index,
            |annotation| {
                if let AnnotationShape::Arrow { style: value, .. } = &mut annotation.shape {
                    *value = style;
                }
            },
            dirty_mask,
        );
    } else {
        state.tool_settings.arrow_style = style;
        state.settings_panel.dirty = true;
    }
}

/// 【线条】【线型修改】把 style 用于 state 当前标注或后续工具；dirty_mask 返回范围，不覆盖另一个工具的默认线型
pub fn set_style(state: &mut EditorState, style: LineStyle, dirty_mask: &mut u32) {
    if !available(state) || current_style(state) == style {
        return;
    }
    if let Some(index) = active_annotation_idx(state) {
        change_annotation(
            state,
            index,
            |annotation| match &mut annotation.shape {
                AnnotationShape::Arrow { line_style, .. } => *line_style = style,
                AnnotationShape::Line { style: value, .. } => *value = style,
                _ => {}
            },
            dirty_mask,
        );
    } else {
        if state.selected_tool == Tool::Arrow {
            state.tool_settings.arrow_line_style = style;
        } else {
            state.tool_settings.line_style = style;
        }
        state.settings_panel.dirty = true;
    }
}

/// 【线条】【骨架插入】在 state 的 index 路径附近插入 point 的投影；dirty_mask 返回范围，端点附近或容量不足返回 false
pub fn insert_control(
    state: &mut EditorState,
    index: usize,
    point: Point,
    dirty_mask: &mut u32,
) -> bool {
    if state.ann_drag.is_some() || state.pending.is_some() {
        return false;
    }
    let Some(annotation) = state.annotations.get(index) else {
        return false;
    };
    let Some(line) = annotation.shape.line() else {
        return false;
    };
    if line.points().any(|anchor| distance(anchor, point) <= 10.0) {
        return false;
    }
    let Some(insertion) = geometry::insertion_at(line, point, 10.0 + annotation.stroke_width * 0.5)
    else {
        return false;
    };
    let mut after = line.clone();
    if !after.insert_control(insertion.index, insertion.point) {
        return false;
    }
    let id = annotation.id;
    change_annotation(
        state,
        index,
        |annotation| {
            *annotation.shape.line_mut().unwrap() = after;
        },
        dirty_mask,
    );
    state.selected_annotation = Some(index);
    state.line_session.selected_point = Some((id, insertion.index));
    state.line_session.clear_click();
    true
}

/// 【线条】【骨架删除】删除 state 当前控制点而非整个标注；dirty_mask 返回范围，端点、过期索引及拖动中返回 false
pub fn remove_selected_control(state: &mut EditorState, dirty_mask: &mut u32) -> bool {
    if state.ann_drag.is_some() || state.pending.is_some() {
        return false;
    }
    let Some((id, point_index)) = state.line_session.selected_point else {
        return false;
    };
    let Some(index) = active_annotation_idx(state) else {
        return false;
    };
    let annotation = &state.annotations[index];
    if annotation.id != id {
        return false;
    }
    let Some(line) = annotation.shape.line() else {
        return false;
    };
    let mut after = line.clone();
    if !after.remove_control(point_index) {
        return false;
    }
    let next = if after.point(point_index).is_some() {
        Some(point_index)
    } else if !after.controls.is_empty() {
        Some(after.controls.len() + 1)
    } else {
        None
    };
    change_annotation(
        state,
        index,
        |annotation| {
            *annotation.shape.line_mut().unwrap() = after;
        },
        dirty_mask,
    );
    state.line_session.selected_point = next.map(|point| (id, point));
    state.line_session.clear_click();
    true
}
