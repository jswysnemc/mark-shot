use crate::editor::{DamageZone, EditorState};
use crate::interaction::snap_line;
use crate::renderer::shadow_color_for;
use crate::tools::ToolBehavior;
use crate::types::annotations::{Annotation, AnnotationShape};
use crate::types::{MouseButton, ToolSettings};
use tiny_skia::Rect;

pub type MakeShape = fn((f32, f32), (f32, f32), &ToolSettings) -> AnnotationShape;

pub struct SimpleShapeTool {
    pub make_shape: MakeShape,
    pub settings: ToolSettings,
    pub stroke_width: f32,
    pub snap: bool,
}

impl ToolBehavior for SimpleShapeTool {
    /// 【框形标注】【创建提交】按state、button与pressed创建草稿或提交标注，dirty_mask返回调用方重绘范围，无返回值
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
            let color = self.settings.color;
            let mut ann = Annotation {
                style_scale: 1.0,
                rotation_degrees: 0.0,
                frozen_bounds: crate::tools::drawing_space::DrawingSpace::from_editor(state)
                    .map(|space| space.bounds),
                id: state.next_id,
                shape: (self.make_shape)(pos, pos, &self.settings),
                color,
                shadow_color: shadow_color_for(color),
                stroke_width: self.stroke_width,
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
        let space = crate::tools::drawing_space::DrawingSpace::from_editor(state);
        if let Some(ann) = state.pending.as_mut() {
            state.damage_rects.push(DamageZone::Global(ann.bbox));

            let pos = (canvas_point.0 as f32, canvas_point.1 as f32);
            let start = ann.shape.start_point();
            let end = if self.settings.fixed_ratio || state.input.shift && !self.snap {
                space.map_or_else(
                    || crate::features::shape::square_end(start, pos),
                    |space| space.rectangle_end(start, pos, true),
                )
            } else if self.snap && state.input.shift {
                snap_line(start, pos)
            } else {
                pos
            };
            ann.shape = (self.make_shape)(start, end, &self.settings);
            ann.update_bbox();

            state.damage_rects.push(DamageZone::Global(ann.bbox));
        }
    }
}
