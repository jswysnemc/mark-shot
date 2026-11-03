//! 【放大镜】【输入手势】创建持久双框标注并独立调整两个框，不承担原图采样

use tiny_skia::Rect;

use crate::editor::annotation_drag::{apply_annotation_drag, commit_drag_if_changed};
use crate::editor::{DamageZone, EditorState};
use crate::features::magnifier::{Magnifier, drag};
use crate::tools::ToolBehavior;
use crate::tools::drawing_space::DrawingSpace;
use crate::types::{
    AnnDragState, Annotation, AnnotationShape, CursorIcon, MouseButton, SelectionHandle,
    StrokeGroup,
};
use crate::utils::{cursor_for_handle, get_overlapping_monitors};

pub struct MagnifierTool;

impl ToolBehavior for MagnifierTool {
    /// 【放大镜】【鼠标按钮】处理 state 中 button 的 pressed 状态；dirty_mask 返回重绘显示器，只响应左键，无返回值
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        dirty_mask: &mut u32,
    ) {
        if !matches!(button, MouseButton::Left) {
            return;
        }
        if pressed {
            if let Some((index, handle)) = hit(state, state.input.pointer.global) {
                begin_drag(state, index, handle, dirty_mask);
            } else {
                begin_draft(state, dirty_mask);
            }
        } else {
            if state.ann_drag.is_some() {
                apply_annotation_drag(state, state.input.pointer.global);
                commit_drag_if_changed(state);
            } else if state.input.mouse_down {
                update_draft(state, dirty_mask);
                finish_draft(state, dirty_mask);
            }
            state.input.mouse_down = false;
            state.input.drag_start = None;
            state.tool_active = false;
        }
    }

    /// 【放大镜】【指针移动】根据 global 更新 state 的双框拖动或创建草稿；dirty_mask 返回重绘显示器，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32) {
        if state.ann_drag.is_some() {
            apply_annotation_drag(state, global);
        } else if state.input.mouse_down {
            update_draft(state, dirty_mask);
        }
    }

    /// 【放大镜】【工具退出】提交 state 的已有框拖动，取消创建草稿并结束输入；dirty_mask 返回重绘显示器，无返回值
    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        commit_drag_if_changed(state);
        crate::editor::drawing_gesture::cancel(state, dirty_mask);
    }

    /// 【放大镜】【工具光标】根据 state 的已选双框控制点或正在拖动的框返回操作光标
    fn cursor(&self, state: &EditorState) -> CursorIcon {
        let handle = state
            .ann_drag
            .as_ref()
            .map(|drag| drag.handle)
            .or_else(|| hit(state, state.input.pointer.global).map(|(_, handle)| handle));
        handle
            .and_then(|handle| cursor_for_handle(handle, state.ann_drag.is_some()))
            .unwrap_or(CursorIcon::Crosshair)
    }
}

/// 【放大镜】【持久框命中】在 state 中优先检查已选框，再从上到下命中其他框；global 为逻辑坐标，返回索引与操作
fn hit(state: &EditorState, global: (f64, f64)) -> Option<(usize, SelectionHandle)> {
    let point = (global.0 as f32, global.1 as f32);
    if let Some(index) = state.selected_annotation
        && let Some(annotation) = state.annotations.get(index)
        && let AnnotationShape::Magnifier { magnifier } = annotation.shape
    {
        let handle = drag::hit_test(magnifier, point, true);
        if handle != SelectionHandle::None {
            return Some((index, handle));
        }
    }
    state
        .annotations
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, annotation)| {
            let AnnotationShape::Magnifier { magnifier } = annotation.shape else {
                return None;
            };
            let handle = drag::hit_test(magnifier, point, false);
            (handle != SelectionHandle::None).then_some((index, handle))
        })
}

/// 【放大镜】【已有框拖动】为 state 的 index 标注保存 handle 与原始几何；dirty_mask 返回新旧控制框重绘，无返回值
fn begin_drag(
    state: &mut EditorState,
    index: usize,
    handle: SelectionHandle,
    dirty_mask: &mut u32,
) {
    if let Some(previous) = state
        .selected_annotation
        .and_then(|index| state.annotations.get(index))
    {
        damage(state, previous.damage_bbox(true), dirty_mask);
    }
    let annotation = state.annotations[index].clone();
    state.ann_drag = Some(AnnDragState {
        handle,
        start_global: state.input.pointer.global,
        prev_global: state.input.pointer.global,
        orig: annotation.clone(),
        orig_index: index,
    });
    state.selected_annotation = Some(index);
    state.input.mouse_down = true;
    state.tool_active = true;
    damage(state, annotation.damage_bbox(true), dirty_mask);
}

/// 【放大镜】【创建开始】在 state 原图范围内保存固定取样起点；dirty_mask 返回选择框重绘，不提交空草稿，无返回值
fn begin_draft(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.selection.zone.is_none() {
        return;
    }
    let Some(space) = DrawingSpace::from_editor(state) else {
        return;
    };
    let Some(start) = space.start_point(state.input.pointer.global) else {
        return;
    };
    let Some(magnifier) = Magnifier::new(
        state.tool_settings.magnifier_shape,
        start,
        state.tool_settings.magnifier_zoom,
        space.bounds,
        space.pixel_scale,
    ) else {
        return;
    };
    if let Some(previous) = state
        .selected_annotation
        .and_then(|index| state.annotations.get(index))
    {
        damage(state, previous.damage_bbox(true), dirty_mask);
    }
    state.selected_annotation = None;
    state.input.drag_start = Some(state.input.pointer.global);
    state.input.mouse_down = true;
    state.tool_active = true;
    let annotation = Annotation {
        id: state.next_id,
        shape: AnnotationShape::Magnifier { magnifier },
        color: state.tool_settings.color,
        shadow_color: crate::renderer::shadow_color_for(state.tool_settings.color),
        stroke_width: state.tool_settings.stroke(StrokeGroup::Magnifier),
        bbox: Rect::from_xywh(start.0, start.1, 1.0, 1.0).unwrap(),
    };
    state.pending = Some(annotation.clone());
    state.prev_pending = Some(annotation);
}

/// 【放大镜】【创建跟随】用 state 的原始取样起点重新计算镜框，避免边缘约束累计漂移；dirty_mask 返回新旧重绘区域，无返回值
fn update_draft(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(space) = DrawingSpace::from_editor(state) else {
        return;
    };
    let Some(start) = state
        .input
        .drag_start
        .and_then(|point| space.start_point(point))
    else {
        return;
    };
    let Some(point) = space.clamp_point(state.input.pointer.global) else {
        return;
    };
    let Some(magnifier) = Magnifier::new(
        state.tool_settings.magnifier_shape,
        start,
        state.tool_settings.magnifier_zoom,
        space.bounds,
        space.pixel_scale,
    )
    .map(|draft| draft.draft_to(point)) else {
        return;
    };
    let Some(annotation) = state.pending.as_mut() else {
        return;
    };
    if !matches!(annotation.shape, AnnotationShape::Magnifier { .. }) {
        return;
    }
    let old = annotation.damage_bbox(false);
    annotation.shape = AnnotationShape::Magnifier { magnifier };
    annotation.color = state.tool_settings.color;
    annotation.stroke_width = state.tool_settings.stroke(StrokeGroup::Magnifier);
    annotation.update_bbox();
    let new = annotation.damage_bbox(false);
    damage(state, old, dirty_mask);
    damage(state, new, dirty_mask);
}

/// 【放大镜】【创建提交】将 state 的有效草稿写入持久图层与一次撤销；dirty_mask 返回草稿移除区域，空手势不产生历史，无返回值
fn finish_draft(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(annotation) = state.pending.take() else {
        return;
    };
    state.prev_pending = None;
    damage(state, annotation.damage_bbox(false), dirty_mask);
    if !matches!(annotation.shape, AnnotationShape::Magnifier { magnifier } if magnifier.measures())
    {
        return;
    }
    state.push_undo();
    state.next_id += 1;
    state.bake_annotation(&annotation);
    state.annotations.push(annotation);
}

/// 【放大镜】【重绘通知】把 area 加入 state 的逻辑重绘区域；dirty_mask 返回重叠显示器，无返回值
fn damage(state: &mut EditorState, area: Rect, dirty_mask: &mut u32) {
    state.damage_rects.push(DamageZone::Global(area));
    *dirty_mask |= get_overlapping_monitors(&area, &state.placements);
}
