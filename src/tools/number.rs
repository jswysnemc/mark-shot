//! 【序号】【绘图手势】单击创建圆框，拖动创建指向标记，提交后推进独立编号

use tiny_skia::{Color, Rect};

use crate::editor::{DamageZone, EditorState};
use crate::features::number::NumberMark;
use crate::tools::{ToolBehavior, drawing_space::DrawingSpace};
use crate::types::{Annotation, AnnotationShape, MouseButton, StrokeGroup};
use crate::ui::toast::ToastKind;
use crate::utils::get_overlapping_monitors;

pub struct NumberTool;

impl ToolBehavior for NumberTool {
    /// 【序号】【鼠标按钮】处理 state 中 button 的 pressed 状态；只响应左键，dirty_mask 返回新旧序号重绘范围，无返回值
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
            state.input.mouse_down = false;
            state.tool_active = false;
            if state.selection.zone.is_none() {
                return;
            }
            let Some(position) = DrawingSpace::from_editor(state)
                .and_then(|space| space.start_point(state.canvas_point()))
            else {
                return;
            };
            let Some(number) = state.number_sequence.next() else {
                state
                    .toasts
                    .show(ToastKind::NumberExhausted, &mut state.text.font_system);
                *dirty_mask = u32::MAX;
                return;
            };
            let Some(mark) = NumberMark::new(position, number, state.tool_settings.number_style)
            else {
                return;
            };
            let mut annotation = Annotation {
                style_scale: 1.0,
                rotation_degrees: 0.0,
                frozen_bounds: None,
                id: state.next_id,
                shape: AnnotationShape::Number { mark },
                color: state.tool_settings.color,
                shadow_color: Color::TRANSPARENT,
                stroke_width: state.tool_settings.stroke(StrokeGroup::Number),
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
            if state.ann_drag.is_some() {
                return;
            }
            let Some(annotation) = state
                .pending
                .take_if(|annotation| matches!(annotation.shape, AnnotationShape::Number { .. }))
            else {
                return;
            };
            let AnnotationShape::Number { mark } = annotation.shape else {
                return;
            };
            // 1. 【序号】【提交标记】先保存下一编号，取消草稿和撤销提交才不会消耗或重复编号
            state.push_undo();
            state.number_sequence.advance(mark.number);
            state.next_id += 1;
            state.bake_annotation(&annotation);
            damage(state, annotation.damage_bbox(false), dirty_mask);
            state.annotations.push(annotation);
            state.prev_pending = None;
        }
    }

    /// 【序号】【鼠标移动】依据实际 global 更新 state 的指向尖端，圆心保持按下时位置；dirty_mask 返回新旧范围，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32) {
        if !state.input.mouse_down || state.ann_drag.is_some() {
            return;
        }
        let Some(point) =
            DrawingSpace::from_editor(state).and_then(|space| space.clamp_point(global))
        else {
            return;
        };
        let Some(annotation) = state.pending.as_mut() else {
            return;
        };
        let old = annotation.damage_bbox(false);
        let AnnotationShape::Number { mark } = &mut annotation.shape else {
            return;
        };
        mark.tip = point;
        mark.style = state.tool_settings.number_style;
        annotation.color = state.tool_settings.color;
        annotation.set_image_stroke_width(state.tool_settings.stroke(StrokeGroup::Number));
        annotation.update_bbox();
        let new = annotation.damage_bbox(false);
        for area in [old, new] {
            damage(state, area, dirty_mask);
        }
    }

    /// 【序号】【工具退出】取消 state 的未提交序号，保留计数和所有持久标注；dirty_mask 返回预览清除范围，无返回值
    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        crate::editor::drawing_gesture::cancel(state, dirty_mask);
    }
}

/// 【序号】【重绘通知】将 area 加入 state 的损坏范围并更新 dirty_mask；无返回值
fn damage(state: &mut EditorState, area: Rect, dirty_mask: &mut u32) {
    state.damage_rects.push(DamageZone::Global(area));
    *dirty_mask |= get_overlapping_monitors(&area, &state.placements);
}
