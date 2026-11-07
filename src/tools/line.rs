//! 【线条】【绘图手势】直线和箭头共用端点创建、原图边界、四十五度吸附与草稿取消

use tiny_skia::{Color, Rect};

use crate::editor::{DamageZone, EditorState};
use crate::features::line::{LineSkeleton, clamped_width, geometry};
use crate::tools::{ToolBehavior, drawing_space::DrawingSpace};
use crate::types::{Annotation, AnnotationShape, MouseButton, StrokeGroup};

pub struct LineTool {
    pub arrow: bool,
}

impl ToolBehavior for LineTool {
    /// 【线条】【鼠标按钮】处理 state 的 button、pressed；只响应左键，dirty_mask 返回新旧绘制范围，无返回值
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
            let Some(point) = DrawingSpace::from_editor(state)
                .and_then(|space| space.start_point(state.canvas_point()))
            else {
                return;
            };
            let Some(line) = LineSkeleton::new(point, point) else {
                return;
            };
            let group = if self.arrow {
                StrokeGroup::Arrow
            } else {
                StrokeGroup::Line
            };
            let shape = if self.arrow {
                AnnotationShape::Arrow {
                    line,
                    style: state.tool_settings.arrow_style,
                    line_style: state.tool_settings.arrow_line_style,
                }
            } else {
                AnnotationShape::Line {
                    line,
                    style: state.tool_settings.line_style,
                }
            };
            let mut annotation = Annotation {
                style_scale: 1.0,
                rotation_degrees: 0.0,
                frozen_bounds: None,
                id: state.next_id,
                shape,
                color: state.tool_settings.color,
                shadow_color: Color::TRANSPARENT,
                stroke_width: clamped_width(state.tool_settings.stroke(group), 1.0),
                bbox: Rect::from_xywh(point.0, point.1, 1.0, 1.0).unwrap(),
            };
            state.adopt_annotation_style(&mut annotation);
            annotation.update_bbox();
            state
                .damage_rects
                .push(DamageZone::Global(annotation.damage_bbox(false)));
            state.pending = Some(annotation.clone());
            state.prev_pending = Some(annotation);
            state.input.mouse_down = true;
            state.tool_active = true;
        } else {
            state.input.mouse_down = false;
            state.tool_active = false;
            if state.ann_drag.is_some() {
                return;
            }
            let valid = state
                .pending
                .as_ref()
                .and_then(|annotation| annotation.shape.line())
                .is_some_and(|line| {
                    geometry::sampled(line).is_some_and(|sample| sample.length >= line.style_scale)
                });
            if !valid {
                crate::editor::drawing_gesture::cancel(state, dirty_mask);
                return;
            }
            let annotation = state.pending.take().unwrap();
            // 1. 【线条】【提交骨架】只为有实际长度的路径记录一次历史，空点击不留下隐藏标注
            state.push_undo();
            state.next_id += 1;
            state.bake_annotation(&annotation);
            state
                .damage_rects
                .push(DamageZone::Global(annotation.damage_bbox(false)));
            state.annotations.push(annotation);
            state.prev_pending = None;
        }
    }

    /// 【线条】【鼠标移动】用 global 更新 state 草稿终点；Shift 吸附四十五度，dirty_mask 返回损坏区域，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), _dirty_mask: &mut u32) {
        if !state.input.mouse_down || state.ann_drag.is_some() {
            return;
        }
        let Some(space) = DrawingSpace::from_editor(state) else {
            return;
        };
        let Some(point) = space.clamp_point(global) else {
            return;
        };
        let Some(annotation) = state.pending.as_mut() else {
            return;
        };
        let before = annotation.damage_bbox(false);
        let Some(line) = annotation.shape.line_mut() else {
            return;
        };
        let point = if state.input.shift {
            geometry::snapped_end(line.start, point)
        } else {
            point
        };
        line.end = (
            point.0.clamp(space.bounds.left(), space.bounds.right()),
            point.1.clamp(space.bounds.top(), space.bounds.bottom()),
        );
        annotation.update_bbox();
        state.damage_rects.push(DamageZone::Global(before));
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
    }

    /// 【线条】【工具退出】取消 state 未提交骨架并恢复旧图层；dirty_mask 返回重绘范围，无返回值
    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        crate::editor::drawing_gesture::cancel(state, dirty_mask);
    }
}
