//! 【滤镜】【区域创建】矩形区域按原图像素限制，创建取消不产生历史或遗留像素缓存

use tiny_skia::Rect;

use crate::editor::{DamageZone, EditorState};
use crate::features::filter::RegionFilter;
use crate::renderer::shadow_color_for;
use crate::tools::{ToolBehavior, drawing_space::DrawingSpace};
use crate::types::{Annotation, AnnotationShape, MouseButton};

pub struct FilterTool;

impl ToolBehavior for FilterTool {
    /// 【滤镜】【鼠标手势】处理 state 中 button 的 pressed 状态；dirty_mask 返回损坏区域，非左键不创建标注
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
        let Some(space) = DrawingSpace::from_editor(state) else {
            return;
        };
        if pressed {
            let Some(point) = space.start_point(state.canvas_point()) else {
                return;
            };
            let mut filter = RegionFilter::new(
                point,
                point,
                state.tool_settings.strength,
                state.tool_settings.filter_mode,
            )
            .unwrap();
            filter.style_scale = 1.0 / space.pixel_scale;
            let color = state.tool_settings.color;
            let mut annotation = Annotation {
                style_scale: 1.0,
                rotation_degrees: 0.0,
                frozen_bounds: None,
                id: state.next_id,
                shape: AnnotationShape::Redact { filter },
                color,
                shadow_color: shadow_color_for(color),
                stroke_width: 0.0,
                bbox: Rect::from_xywh(point.0, point.1, filter.style_scale, filter.style_scale)
                    .unwrap(),
            };
            state.adopt_annotation_style(&mut annotation);
            annotation.update_bbox();
            state.pending = Some(annotation.clone());
            state.prev_pending = Some(annotation);
        } else {
            self.on_move(state, state.canvas_point(), dirty_mask);
            let valid = state.pending.as_ref().is_some_and(|annotation| {
                matches!(annotation.shape, AnnotationShape::Redact { filter }
                    if space.rectangle_measures(filter.start, filter.end, 2.0))
            });
            if !valid {
                crate::editor::drawing_gesture::cancel(state, dirty_mask);
                return;
            }
            if let Some(annotation) = state.pending.take() {
                state.push_undo();
                state.next_id += 1;
                state.bake_annotation(&annotation);
                if let AnnotationShape::Redact { filter } = annotation.shape
                    && let Some(key) = crate::features::filter::raster::FilterKey::new(
                        crate::renderer::FrozenImage::from_parts(
                            state.file_canvas.as_ref(),
                            &state.captures,
                            &state.placements,
                        ),
                        filter,
                    )
                {
                    log::info!(
                        "【滤镜】【区域提交】id={} mode={} strength={} source=[{},{},{},{}] logical=[{:.4},{:.4},{:.4},{:.4}]",
                        annotation.id,
                        filter.mode.key(),
                        filter.strength,
                        key.region.left(),
                        key.region.top(),
                        key.region.right(),
                        key.region.bottom(),
                        filter.start.0,
                        filter.start.1,
                        filter.end.0,
                        filter.end.1
                    );
                }
                state
                    .damage_rects
                    .push(DamageZone::Global(annotation.damage_bbox(false)));
                state.annotations.push(annotation);
                state.prev_pending = None;
            }
        }
    }

    /// 【滤镜】【区域更新】将 state 草稿终点约束到 global 所在原图范围；dirty_mask 由调用方处理损坏，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), _dirty_mask: &mut u32) {
        let Some(point) =
            DrawingSpace::from_editor(state).and_then(|space| space.clamp_point(global))
        else {
            return;
        };
        let Some(annotation) = state.pending.as_mut() else {
            return;
        };
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
        let AnnotationShape::Redact { filter } = &mut annotation.shape else {
            return;
        };
        filter.end = point;
        annotation.update_bbox();
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
    }

    /// 【滤镜】【工具退出】取消 state 中未提交区域并释放临时缓存；dirty_mask 返回新旧绘制范围，无返回值
    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        crate::editor::drawing_gesture::cancel(state, dirty_mask);
        state
            .filters
            .retain_live(&state.annotations, state.pending.as_ref());
    }
}
