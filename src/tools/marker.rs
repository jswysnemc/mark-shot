//! 【形状标记】【绘图手势】在源图范围内创建、更新和提交持久标记

use tiny_skia::Rect;

use crate::editor::{DamageZone, EditorState};
use crate::tools::{ToolBehavior, drawing_space::DrawingSpace};
use crate::types::{Annotation, AnnotationShape, MouseButton, StrokeGroup};
use crate::utils::get_overlapping_monitors;

pub struct MarkerTool;

impl ToolBehavior for MarkerTool {
    /// 【形状标记】【鼠标按钮】处理 state 中 button 的 pressed 状态；仅响应左键，dirty_mask 返回重绘显示器
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
            if state.selection.zone.is_none() {
                return;
            }
            let Some(position) = DrawingSpace::from_editor(state)
                .and_then(|space| space.start_point(state.canvas_point()))
            else {
                return;
            };
            let mut annotation = Annotation {
                style_scale: 1.0,
                rotation_degrees: 0.0,
                frozen_bounds: crate::tools::drawing_space::DrawingSpace::from_editor(state)
                    .map(|space| space.bounds),
                id: state.next_id,
                shape: AnnotationShape::Marker {
                    start: position,
                    end: position,
                    kind: state.tool_settings.marker_shape,
                    filled: state.tool_settings.marker_filled,
                    radius: state.tool_settings.corner_radius,
                },
                color: state.tool_settings.color,
                shadow_color: crate::renderer::shadow_color_for(state.tool_settings.color),
                stroke_width: state.tool_settings.stroke(StrokeGroup::Marker),
                bbox: Rect::from_xywh(position.0, position.1, 1.0, 1.0).unwrap(),
            };
            state.adopt_annotation_style(&mut annotation);
            annotation.update_bbox();
            state.input.mouse_down = true;
            state.tool_active = true;
            damage(state, annotation.damage_bbox(false), dirty_mask);
            state.pending = Some(annotation.clone());
            state.prev_pending = Some(annotation);
        } else {
            state.input.mouse_down = false;
            state.tool_active = false;
            let Some(annotation) = state
                .pending
                .take_if(|annotation| matches!(&annotation.shape, AnnotationShape::Marker { .. }))
            else {
                return;
            };
            // 1. 【形状标记】【提交标记】微小草稿只清除预览，不产生空标注或撤销记录
            let measures = match &annotation.shape {
                AnnotationShape::Marker { start, end, .. } => DrawingSpace::from_editor(state)
                    .is_some_and(|space| space.rectangle_measures(*start, *end, 2.0)),
                _ => false,
            };
            if !measures {
                damage(state, annotation.damage_bbox(false), dirty_mask);
                state.prev_pending = None;
                return;
            }
            state.push_undo();
            state.next_id += 1;
            state.bake_annotation(&annotation);
            damage(state, annotation.damage_bbox(false), dirty_mask);
            state.annotations.push(annotation);
            state.prev_pending = None;
        }
    }

    /// 【形状标记】【鼠标移动】根据 global 更新 state 的标记终点和样式；dirty_mask 返回新旧重绘区域，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32) {
        if !state.input.mouse_down {
            return;
        }
        let Some(space) = DrawingSpace::from_editor(state) else {
            return;
        };
        let Some(position) = space.clamp_point(global) else {
            return;
        };
        let Some(annotation) = state.pending.as_mut() else {
            return;
        };
        let old = annotation.damage_bbox(false);
        let AnnotationShape::Marker {
            start,
            end,
            kind,
            filled,
            ..
        } = &mut annotation.shape
        else {
            return;
        };
        *end = space.rectangle_end(*start, position, state.input.ctrl);
        *kind = state.tool_settings.marker_shape;
        *filled = state.tool_settings.marker_filled;
        annotation.set_image_stroke_width(state.tool_settings.stroke(StrokeGroup::Marker));
        annotation.color = state.tool_settings.color;
        annotation.update_bbox();
        let new = annotation.damage_bbox(false);
        for area in [old, new] {
            damage(state, area, dirty_mask);
        }
    }

    /// 【形状标记】【工具退出】取消 state 的未提交标记；dirty_mask 返回需要清除的区域，无返回值
    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        crate::editor::drawing_gesture::cancel(state, dirty_mask);
    }
}

/// 【形状标记】【重绘通知】将 area 加入 state 的重绘范围并更新 dirty_mask；无返回值
fn damage(state: &mut EditorState, area: Rect, dirty_mask: &mut u32) {
    state.damage_rects.push(DamageZone::Global(area));
    *dirty_mask |= get_overlapping_monitors(&area, &state.placements);
}
