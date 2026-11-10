//! 【标注】【拖动事务】管理持久标注的命中、移动、缩放和历史提交

mod generic;

use crate::editor::{DamageZone, EditorState};
use crate::tools::text::update_text_bbox_inline;
use crate::types::{AnnDragState, Annotation, AnnotationShape, SelectionHandle};
use crate::utils::{apply_handle_drag, hit_test_rect_handle};

/// 【标注】【控制点命中】判断 ann 在全局 pos 下的操作类型；返回移动、缩放或空控制点
pub fn handle_hit_test_for_annotation(ann: &Annotation, pos: (f64, f64)) -> SelectionHandle {
    if let Some(line) = ann.shape.line()
        && ann.rotation_degrees != 0.0
    {
        let local = ann.to_unrotated((pos.0 as f32, pos.1 as f32));
        if let Some((index, _)) = line
            .points()
            .enumerate()
            .map(|(index, point)| (index, (point.0 - local.0).hypot(point.1 - local.1)))
            .filter(|(_, distance)| *distance <= 10.0)
            .min_by(|first, second| first.1.total_cmp(&second.1))
        {
            return SelectionHandle::LinePoint(index);
        }
        if ann.generic_rotation_handle().is_some_and(|(_, point)| {
            (f64::from(point.0) - pos.0).hypot(f64::from(point.1) - pos.1) <= 11.2
        }) {
            return SelectionHandle::Rotate;
        }
        return crate::features::annotation_selection::chrome::bounds(ann)
            .map_or(SelectionHandle::None, |bounds| {
                hit_test_rect_handle(&bounds, (f64::from(local.0), f64::from(local.1)))
            });
    }
    if let AnnotationShape::Redact { filter } = ann.shape {
        return crate::features::filter::drag::hit_test(filter, (pos.0 as f32, pos.1 as f32), true);
    }
    if let Some(line) = ann.shape.line() {
        return crate::features::line::drag::hit_test(
            line,
            crate::features::annotation_selection::chrome::bounds(ann).unwrap_or(ann.bbox),
            ann.stroke_width,
            (pos.0 as f32, pos.1 as f32),
            true,
        );
    }
    if let AnnotationShape::Number { mark } = ann.shape {
        return crate::features::number::drag::hit_test(
            mark,
            ann.stroke_width,
            (pos.0 as f32, pos.1 as f32),
            true,
        );
    }
    if let AnnotationShape::Magnifier { magnifier } = ann.shape {
        return crate::features::magnifier::drag::hit_test(
            magnifier,
            (pos.0 as f32, pos.1 as f32),
            true,
        );
    }
    if let AnnotationShape::Ruler { measurement } = ann.shape {
        return crate::features::ruler::drag::hit_test(
            measurement,
            (pos.0 as f32, pos.1 as f32),
            ann.stroke_width,
            true,
            true,
        );
    }
    let bbox = crate::features::annotation_selection::chrome::bounds(ann).unwrap_or(ann.bbox);
    if let Some((_, handle)) = ann.generic_rotation_handle() {
        let tolerance = crate::config::get().input.handle_hit_width * 0.5 * 1.4;
        if (pos.0 - f64::from(handle.0)).hypot(pos.1 - f64::from(handle.1)) <= f64::from(tolerance)
        {
            return SelectionHandle::Rotate;
        }
    }
    let local = ann.to_unrotated((pos.0 as f32, pos.1 as f32));
    hit_test_rect_handle(&bbox, (f64::from(local.0), f64::from(local.1)))
}

/// 【标注】【开始拖动】为 state 中 idx 标注保存原始快照与指针位置；无返回值
pub fn begin_drag_for_annotation(state: &mut EditorState, idx: usize) {
    let ann = &state.annotations[idx];
    let handle = state.canvas_handle(ann, state.canvas_point());
    if ann.shape.line().is_some() {
        let selected_point = match handle {
            SelectionHandle::LinePoint(index) => Some((ann.id, index)),
            _ => None,
        };
        if state.line_session.selected_point != selected_point {
            state.line_session.selected_point = selected_point;
            state
                .damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
        }
    }

    state.ann_drag = Some(AnnDragState {
        handle,
        start_global: state.canvas_point(),
        prev_global: state.canvas_point(),
        orig: ann.clone(),
        orig_index: idx,
    });
}

/// 【标注】【提交拖动】把 state 的临时标注放回图层，仅对实际变化记录撤销；无返回值
pub fn commit_drag_if_changed(state: &mut EditorState) {
    if let Some(drag) = state.ann_drag.take()
        && let Some(ann) = state.pending.take()
    {
        let actually_changed =
            !matches!(drag.handle, SelectionHandle::None) && !ann.same_edit_state(&drag.orig);

        let insert_idx = drag.orig_index.min(state.annotations.len());
        if actually_changed {
            if drag.orig.shape.line().is_some() {
                state.line_session.clear_click();
            }
            let mut pre_drag = state.annotations.clone();
            pre_drag.insert(insert_idx, drag.orig);
            state
                .undo_stack
                .push(crate::editor::history_snapshot::HistorySnapshot {
                    annotations: pre_drag,
                    number_sequence: state.number_sequence,
                    selected_id: Some(ann.id),
                    selected_ids: vec![ann.id],
                    next_id: state.next_id,
                });
            state.redo_stack.clear();
        }
        state.layer_damage_rects.push(ann.damage_bbox(false));
        state
            .damage_rects
            .push(DamageZone::Global(ann.damage_bbox(true)));
        state.annotations.insert(insert_idx, ann);
        state.prev_pending = None;
        state.select_annotation_index(Some(insert_idx));
    }
}

/// 【标注】【更新拖动】依据 global 指针位置移动或缩放 state 中的标注；记录新旧重绘区域，无返回值
pub fn apply_annotation_drag(state: &mut EditorState, global: (f64, f64)) {
    let (handle, prev_global, start_global, orig_index) = match &state.ann_drag {
        Some(drag) => (
            drag.handle,
            drag.prev_global,
            drag.start_global,
            drag.orig_index,
        ),
        None => return,
    };

    if matches!(handle, SelectionHandle::None) {
        return;
    }
    let filter_geometry = state.ann_drag.as_ref().and_then(|drag| {
        if let AnnotationShape::Redact { filter } = drag.orig.shape {
            Some(filter)
        } else {
            None
        }
    });
    let filter_bounds = filter_geometry.and_then(|_| {
        crate::tools::drawing_space::DrawingSpace::from_editor(state).map(|space| space.bounds)
    });
    let ruler_geometry = state.ann_drag.as_ref().and_then(|drag| {
        if let AnnotationShape::Ruler { measurement } = drag.orig.shape {
            Some(measurement)
        } else {
            None
        }
    });
    let ruler_point = if ruler_geometry.is_some() {
        crate::tools::drawing_space::DrawingSpace::from_editor(state)
            .and_then(|space| space.clamp_point(global))
    } else {
        None
    };
    let ruler_window = state.hit_window_units();
    let magnifier_geometry = state.ann_drag.as_ref().and_then(|drag| {
        if let AnnotationShape::Magnifier { magnifier } = drag.orig.shape {
            Some(magnifier)
        } else {
            None
        }
    });
    let number_geometry = state.ann_drag.as_ref().and_then(|drag| {
        if let AnnotationShape::Number { mark } = drag.orig.shape {
            Some((mark, drag.orig.stroke_width))
        } else {
            None
        }
    });
    let number_bounds = number_geometry.and_then(|_| {
        crate::tools::drawing_space::DrawingSpace::from_editor(state).map(|space| space.bounds)
    });
    let line_geometry = state.ann_drag.as_ref().and_then(|drag| {
        if drag.orig.rotation_degrees != 0.0 {
            return None;
        }
        drag.orig.shape.line().map(|line| {
            (
                line.clone(),
                crate::features::annotation_selection::chrome::bounds(&drag.orig)
                    .unwrap_or(drag.orig.bbox),
            )
        })
    });
    let line_bounds = line_geometry.as_ref().and_then(|_| {
        crate::tools::drawing_space::DrawingSpace::from_editor(state).map(|space| space.bounds)
    });
    let generic_options = state
        .ann_drag
        .as_ref()
        .filter(|drag| drag.orig.uses_generic_rotation())
        .and_then(|_| crate::tools::drawing_space::DrawingSpace::from_editor(state))
        .map(
            |space| crate::features::annotation_rotation::drag::Options {
                bounds: space.bounds,
                pixel_scale: space.pixel_scale,
                keep_aspect: state.input.ctrl,
                snap_rotation: state.input.shift,
            },
        );
    // 1. 【标注】【更新拖动】暂时移出持久图层，避免每次移动重建全部标注
    if state.pending.is_none() {
        if orig_index < state.annotations.len() {
            let ann = state.annotations.remove(orig_index);
            state.layer_damage_rects.push(ann.damage_bbox(false));
            state
                .damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
            state.pending = Some(ann.clone());
            state.prev_pending = Some(ann);
        } else {
            return;
        }
    }

    let Some(ann) = state.pending.as_mut() else {
        return;
    };

    state
        .damage_rects
        .push(DamageZone::Global(ann.damage_bbox(true)));

    // 2. 【标注】【更新拖动】普通手势和专用工具均以开始时的几何快照计算，不累计旋转或缩放误差
    if let (Some(before), Some(bounds)) = (filter_geometry, filter_bounds) {
        ann.shape = AnnotationShape::Redact {
            filter: crate::features::filter::drag::dragged(
                before,
                handle,
                (start_global.0 as f32, start_global.1 as f32),
                (global.0 as f32, global.1 as f32),
                bounds,
                state.input.ctrl,
                state.input.shift,
            ),
        };
        ann.update_bbox();
    } else if let (Some((before, bbox)), Some(bounds)) = (line_geometry, line_bounds) {
        if let Some(line) = ann.shape.line_mut() {
            *line = crate::features::line::drag::dragged(
                &before,
                bbox,
                handle,
                (start_global.0 as f32, start_global.1 as f32),
                (global.0 as f32, global.1 as f32),
                bounds,
                state.input.ctrl,
                state.input.shift,
            );
        }
        ann.update_bbox();
    } else if let (Some((before, width)), Some(bounds)) = (number_geometry, number_bounds) {
        let (mark, width) = crate::features::number::drag::dragged(
            before,
            width,
            handle,
            (start_global.0 as f32, start_global.1 as f32),
            (global.0 as f32, global.1 as f32),
            bounds,
            state.input.ctrl,
            state.input.shift,
        );
        ann.shape = AnnotationShape::Number { mark };
        ann.stroke_width = width;
        ann.update_bbox();
    } else if let Some(before) = magnifier_geometry {
        ann.shape = AnnotationShape::Magnifier {
            magnifier: crate::features::magnifier::drag::dragged(
                before,
                handle,
                (start_global.0 as f32, start_global.1 as f32),
                (global.0 as f32, global.1 as f32),
                state.input.ctrl,
                state.input.shift,
            ),
        };
        ann.update_bbox();
    } else if let (Some(before), Some(point)) = (ruler_geometry, ruler_point) {
        ann.shape = AnnotationShape::Ruler {
            measurement: crate::features::ruler::drag::dragged_with_window(
                before,
                handle,
                (start_global.0 as f32, start_global.1 as f32),
                point,
                state.input.ctrl,
                state.input.shift,
                ruler_window,
            ),
        };
        ann.update_bbox();
    } else if let Some(options) = generic_options {
        let before = &state.ann_drag.as_ref().unwrap().orig;
        generic::apply(
            ann,
            before,
            handle,
            start_global,
            global,
            options,
            &mut state.text,
        );
    } else {
        match handle {
            SelectionHandle::Move => {
                let dx = (global.0 - prev_global.0) as f32;
                let dy = (global.1 - prev_global.1) as f32;
                ann.translate_mut(dx, dy);
            }
            _ => {
                if matches!(ann.shape, AnnotationShape::Text { .. }) {
                    apply_text_resize_incremental(ann, handle, prev_global, global);
                    let editor = crate::tools::text::ensure_text_editor(
                        ann,
                        &mut state.text.editors,
                        &mut state.text.font_system,
                    );
                    update_text_bbox_inline(ann, editor, &mut state.text.font_system);
                } else {
                    let total_dx = (global.0 - start_global.0) as f32;
                    let total_dy = (global.1 - start_global.1) as f32;
                    let orig = state.ann_drag.as_ref().unwrap().orig.clone();
                    apply_shape_resize_from_orig(ann, &orig, handle, total_dx, total_dy);
                }
            }
        }
    }

    state
        .damage_rects
        .push(DamageZone::Global(ann.damage_bbox(true)));

    // 3. 【取色标注】【拖动采样】引线指向点变化后刷新原图颜色，属性栏与标签保持一致
    if let Some(color) = crate::features::color_sample::sampling::refresh(
        ann,
        crate::renderer::FrozenImage::from_parts(
            state.file_canvas.as_ref(),
            &state.captures,
            &state.placements,
        ),
    ) {
        state.color_inspector.live = Some(color);
        state.settings_panel.dirty = true;
    }
    if let Some(drag) = state.ann_drag.as_mut() {
        drag.prev_global = global;
    }
}

/// 【标注】【更新属性】重算 state 中 idx 的图形或文字边界，并通知图层重绘；无返回值
pub fn rebuild_annotation(state: &mut EditorState, idx: usize) {
    let Some(ann) = state.annotations.get(idx) else {
        return;
    };
    state.layer_damage_rects.push(ann.damage_bbox(false));
    state
        .damage_rects
        .push(DamageZone::Global(ann.damage_bbox(true)));

    if matches!(state.annotations[idx].shape, AnnotationShape::Text { .. }) {
        let editor = crate::tools::text::ensure_text_editor(
            &state.annotations[idx],
            &mut state.text.editors,
            &mut state.text.font_system,
        );
        update_text_bbox_inline(
            &mut state.annotations[idx],
            editor,
            &mut state.text.font_system,
        );
    } else {
        state.annotations[idx].update_bbox();
    }

    state
        .layer_damage_rects
        .push(state.annotations[idx].damage_bbox(false));
    state
        .damage_rects
        .push(DamageZone::Global(state.annotations[idx].damage_bbox(true)));
    state.annotations_dirty = true;
}
/// 【标注】【图形缩放】以 orig 快照和 handle、total_dx、total_dy 调整 ann；避免累计误差，无返回值
fn apply_shape_resize_from_orig(
    ann: &mut Annotation,
    orig: &Annotation,
    handle: SelectionHandle,
    total_dx: f32,
    total_dy: f32,
) {
    let mut source = orig.clone();
    source.bbox = crate::features::annotation_selection::chrome::bounds(orig).unwrap_or(orig.bbox);
    let target = apply_handle_drag(&source.bbox, handle, (total_dx as f64, total_dy as f64));
    *ann = source.resize_to_bbox(target);
}
/// 【标注】【文字缩放】依据 handle 与 prev、global 变化调整 ann 字号和位置；无返回值
fn apply_text_resize_incremental(
    ann: &mut Annotation,
    handle: SelectionHandle,
    prev: (f64, f64),
    global: (f64, f64),
) {
    let AnnotationShape::Text {
        font_size, start, ..
    } = &mut ann.shape
    else {
        return;
    };
    let bbox = ann.bbox;

    let (anchor_x, anchor_y) = match handle {
        SelectionHandle::TopLeft => (bbox.right(), bbox.bottom()),
        SelectionHandle::TopRight => (bbox.left(), bbox.bottom()),
        SelectionHandle::BottomLeft => (bbox.right(), bbox.top()),
        SelectionHandle::BottomRight => (bbox.left(), bbox.top()),
        _ => return,
    };

    let prev_dx = prev.0 as f32 - anchor_x;
    let prev_dy = prev.1 as f32 - anchor_y;
    let new_dx = global.0 as f32 - anchor_x;
    let new_dy = global.1 as f32 - anchor_y;

    let prev_dist = (prev_dx.powi(2) + prev_dy.powi(2)).sqrt().max(1.0);
    let new_dist = (new_dx.powi(2) + new_dy.powi(2)).sqrt();

    let dot = prev_dx * new_dx + prev_dy * new_dy;
    let scale = if dot <= 0.0 {
        0.0
    } else {
        new_dist / prev_dist
    };

    let text = &crate::config::get().annotations.text;
    let new_font_size = (*font_size * scale).clamp(text.resize_min, text.resize_max);
    let applied_scale = new_font_size / *font_size;

    start.0 = anchor_x + (start.0 - anchor_x) * applied_scale;
    start.1 = anchor_y + (start.1 - anchor_y) * applied_scale;
    *font_size = new_font_size;
}
