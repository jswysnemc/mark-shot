use crate::editor::{DamageZone, EditorState};
use crate::renderer::shadow_color_for;
use crate::tools::ToolBehavior;
use crate::tools::pen::{straighten, tail_damage};
use crate::types::annotations::{Annotation, AnnotationShape};
use crate::types::{MouseButton, StrokeGroup};
use tiny_skia::Rect;

pub struct HighlighterTool;

impl ToolBehavior for HighlighterTool {
    fn on_button(
        &self,
        state: &mut EditorState,
        _button: MouseButton,
        pressed: bool,
        dirty_mask: &mut u32,
    ) {
        let pos = (
            state.input.pointer.global.0 as f32,
            state.input.pointer.global.1 as f32,
        );

        if pressed {
            state.input.straight = false;
            let mut ann = Annotation {
                id: state.next_id,
                shape: AnnotationShape::Highlighter {
                    points: vec![pos],
                    flat_ends: [false; 2],
                },
                color: state.tool_settings.color,
                shadow_color: shadow_color_for(state.tool_settings.color),
                stroke_width: state.tool_settings.stroke(StrokeGroup::Freehand),
                bbox: Rect::from_xywh(pos.0, pos.1, 1.0, 1.0).unwrap(),
            };
            ann.update_bbox();
            let tail_bbox = ann.pen_active_tail_bbox();
            state.damage_rects.push(DamageZone::Global(tail_bbox));
            state.pending = Some(ann.clone());
            state.prev_pending = Some(ann);
            *dirty_mask |= crate::utils::get_overlapping_monitors(&tail_bbox, &state.placements);
        } else if let Some(ann) = state.pending.take() {
            state.next_id += 1;
            state.push_undo();

            let damage = ann.damage_bbox(false);

            state.bake_annotation(&ann);
            state.damage_rects.push(DamageZone::Global(damage));
            *dirty_mask |= crate::utils::get_overlapping_monitors(&damage, &state.placements);

            state.annotations.push(ann);
            state.prev_pending = None;
        }
    }

    fn on_move(&self, state: &mut EditorState, _global: (f64, f64), dirty_mask: &mut u32) {
        if state.input.shift {
            straighten(state);
            return;
        }

        let raw_pos = (
            state.input.pointer.global.0 as f32,
            state.input.pointer.global.1 as f32,
        );

        let Some(ann) = state.pending.as_mut() else {
            return;
        };
        let stroke_width = ann.stroke_width;

        let AnnotationShape::Highlighter { points, flat_ends } = &mut ann.shape else {
            return;
        };

        let highlighter = &crate::config::get().tools.highlighter;
        let last = points.last().copied();
        let smoothed = if let Some(last) = last {
            (
                last.0 + (raw_pos.0 - last.0) * (1.0 - highlighter.smoothing),
                last.1 + (raw_pos.1 - last.1) * (1.0 - highlighter.smoothing),
            )
        } else {
            raw_pos
        };

        if let Some(last) = last {
            let dx = smoothed.0 - last.0;
            let dy = smoothed.1 - last.1;
            if dx * dx + dy * dy < highlighter.min_distance * highlighter.min_distance {
                return;
            }
        }

        points.push(smoothed);
        flat_ends[1] = false;

        let damage = tail_damage(points, crate::renderer::visual_pad(stroke_width));
        ann.update_bbox();

        if let Some(damage) = damage {
            state.damage_rects.push(DamageZone::Global(damage));
            *dirty_mask |= crate::utils::get_overlapping_monitors(&damage, &state.placements);
        }
    }
}
