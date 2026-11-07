use crate::editor::{DamageZone, EditorState};
use crate::renderer::shadow_color_for;
use crate::tools::ToolBehavior;
use crate::types::annotations::{Annotation, AnnotationShape};
use crate::types::{MouseButton, StrokeGroup};
use tiny_skia::Rect;

pub struct NumeratedArrowTool;

impl ToolBehavior for NumeratedArrowTool {
    /// 【编号箭头】【创建提交】按state、button与pressed创建或提交原图笔宽草稿，dirty_mask由调用方处理重绘，无返回值
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        _dirty_mask: &mut u32,
    ) {
        if button != MouseButton::Left {
            return;
        }
        let pos = (state.canvas_point().0 as f32, state.canvas_point().1 as f32);

        if pressed {
            let mut used_numbers = Vec::new();
            for ann in &state.annotations {
                if let AnnotationShape::NumeratedArrow { number, .. } = &ann.shape {
                    used_numbers.push(*number);
                }
            }

            used_numbers.sort_unstable();
            let mut next_number = 1;
            for num in used_numbers {
                if num == next_number {
                    next_number += 1;
                } else if num > next_number {
                    break;
                }
            }

            let mut ann = Annotation {
                style_scale: 1.0,
                rotation_degrees: 0.0,
                frozen_bounds: None,
                id: state.next_id,
                shape: AnnotationShape::NumeratedArrow {
                    start: pos,
                    end: pos,
                    number: next_number,
                },
                color: state.tool_settings.color,
                shadow_color: shadow_color_for(state.tool_settings.color),
                stroke_width: state.tool_settings.stroke(StrokeGroup::Numbered),
                bbox: Rect::from_xywh(pos.0, pos.1, 1.0, 1.0).unwrap(),
            };
            state.adopt_annotation_style(&mut ann);
            ann.update_bbox();

            state.damage_rects.push(DamageZone::Global(ann.bbox));

            state.pending = Some(ann.clone());
            state.prev_pending = Some(ann);
        } else if let Some(ann) = state.pending.take() {
            state.next_id += 1;
            state.push_undo();
            state.bake_annotation(&ann);
            state
                .damage_rects
                .push(DamageZone::Global(ann.damage_bbox(false)));
            state.annotations.push(ann);
            state.prev_pending = None;
        }
    }

    /// 【画布导航】【工具移动】按state稳定画布指针更新草稿，_global提供已转换位置，_dirty_mask返回重绘范围，无返回值
    fn on_move(&self, state: &mut EditorState, _global: (f64, f64), _dirty_mask: &mut u32) {
        let canvas_point = state.canvas_point();
        if let Some(ann) = state.pending.as_mut() {
            state.damage_rects.push(DamageZone::Global(ann.bbox));

            let pos = (canvas_point.0 as f32, canvas_point.1 as f32);
            let start = ann.shape.start_point();

            let current_number = match &ann.shape {
                AnnotationShape::NumeratedArrow { number, .. } => *number,
                _ => 1,
            };

            ann.shape = AnnotationShape::NumeratedArrow {
                start,
                end: pos,
                number: current_number,
            };
            ann.update_bbox();

            state.damage_rects.push(DamageZone::Global(ann.bbox));
        }
    }
}
