//! 【标注多选】【批量属性】按成员自身类型转换宽度，颜色与透明度保持独立事务和工具默认值语义

use crate::{
    editor::{EditorState, annotation_drag::rebuild_annotation, dirty::apply_damage_rects},
    features::annotation_collection::transform::sync_magnifier_geometry,
    tools::{Tool, drawing_space::DrawingSpace},
    types::{Annotation, AnnotationShape, StrokeGroup},
};
use cosmic_text::Edit;
use tiny_skia::Color;

/// 【标注多选】【画布滚轮】按event与now改变state全部成员原始宽度，dirty返回损坏，连续更新合并历史，返回实际变化
pub(crate) fn canvas_wheel(
    state: &mut EditorState,
    event: crate::types::ScrollEvent,
    now: std::time::Instant,
    dirty: &mut u32,
) -> bool {
    if state.tool_active
        || state.input.mouse_down
        || state.collection.drag.is_some()
        || state.collection.marquee.is_some()
    {
        return false;
    }
    let Some(first) = first(state) else {
        return false;
    };
    let (id, current) = (first.id, width(first).round() as i32);
    let multiplier = match first.shape {
        AnnotationShape::Text { .. } => 1.5,
        AnnotationShape::Redact { .. } | AnnotationShape::Number { .. } => 2.0,
        _ => 1.0,
    };
    state.prepare_text_size_wheel(Some(id), now);
    let delta = state
        .text_size_history
        .input
        .canvas(event.canvas_steps() * multiplier / 1.5);
    if delta == 0 {
        return false;
    }
    let before = state.snapshot();
    if set_width(state, current.saturating_add(delta), false, dirty) {
        state.queue_text_size_wheel(before, now);
        return true;
    }
    false
}

/// 【标注多选】【属性可用】返回state是否处于选择工具且有多项稳定编号，不为草稿或其他工具修改批量属性
pub(crate) fn active(state: &EditorState) -> bool {
    state.selected_tool == Tool::Pick && state.selected_annotation_ids().len() > 1
}

/// 【标注多选】【首项读取】按state返回编号顺序的首个有效成员，空选择返回空值，不按图层顺序替换首项
pub(crate) fn first(state: &EditorState) -> Option<&Annotation> {
    let id = state.selected_annotation_ids().first()?;
    state
        .annotations
        .iter()
        .find(|annotation| annotation.id == *id)
}

/// 【标注多选】【原始宽度】按annotation返回Qt宽度语义，文字为点字号减十九，滤镜为原图强度
pub(crate) fn width(annotation: &Annotation) -> f32 {
    match annotation.shape {
        AnnotationShape::Text { font_size, .. } => font_size - 19.0,
        AnnotationShape::Redact { filter } => filter.strength,
        _ => annotation.image_stroke_width(),
    }
}

/// 【标注多选】【宽度限制】按annotation与requested返回所属工具的合法宽度，文字保留负十一至二百八十一语义
fn clamped_width(annotation: &Annotation, requested: i32) -> f32 {
    let range = match annotation.shape {
        AnnotationShape::Text { .. } => (-11.0, 281.0),
        AnnotationShape::Redact { .. } => crate::features::filter::STRENGTH,
        AnnotationShape::Number { .. } => crate::features::number::WIDTH,
        AnnotationShape::Highlighter { .. } => (1.0, 48.0),
        _ => (1.0, 24.0),
    };
    (requested as f32).clamp(range.0, range.1)
}

/// 【标注多选】【文字改宽】按state、index与value重排字号和换行宽度，单行重新收框，首个定点同步新矩形，无返回值
fn text_width(state: &mut EditorState, index: usize, value: f32) {
    let before = &state.annotations[index];
    let wrapped = {
        let editor = crate::tools::text::ensure_text_editor(
            before,
            &mut state.text.editors,
            &mut state.text.font_system,
        );
        editor.with_buffer(|buffer| buffer.wrap() != cosmic_text::Wrap::None)
    };
    let stored = before.text_stored_bounds().unwrap_or(before.bbox);
    if let AnnotationShape::Text {
        font_size, frame, ..
    } = &mut state.annotations[index].shape
    {
        let points = value + 19.0;
        frame.width = wrapped.then(|| (stored.width() * points / *font_size).max(1.0));
        frame.height = None;
        frame.painted = None;
        *font_size = points;
    }
    if !wrapped {
        crate::tools::text::fit_frame(
            &mut state.annotations[index],
            None,
            &mut state.text.font_system,
        );
    }
    rebuild_annotation(state, index);
    let height = state.annotations[index].bbox.height();
    if let AnnotationShape::Text { start, frame, .. } = &mut state.annotations[index].shape {
        frame.anchor = Some(*start);
        if wrapped {
            frame.height = Some(height);
        }
    }
}

/// 【标注多选】【默认宽度】按shape与value同步state生产工具默认值，普通描边共用值，荧光笔和长度尺独立，无返回值
fn sync_defaults(state: &mut EditorState, shape: &AnnotationShape, value: f32) {
    match shape {
        AnnotationShape::Highlighter { .. } => state
            .tool_settings
            .set_stroke(StrokeGroup::Highlighter, value),
        AnnotationShape::Number { .. } => {
            state.tool_settings.set_stroke(StrokeGroup::Number, value)
        }
        AnnotationShape::Ruler { .. } => state.tool_settings.set_stroke(StrokeGroup::Ruler, value),
        AnnotationShape::Text { .. } => state.tool_settings.font_size = value + 19.0,
        AnnotationShape::Redact { .. } => state.tool_settings.strength = value,
        _ => {
            for group in [
                StrokeGroup::Rectangle,
                StrokeGroup::Circle,
                StrokeGroup::Arrow,
                StrokeGroup::Line,
                StrokeGroup::Numbered,
                StrokeGroup::Freehand,
                StrokeGroup::Laser,
                StrokeGroup::Marker,
                StrokeGroup::Magnifier,
            ] {
                state.tool_settings.set_stroke(group, value);
            }
        }
    }
}

/// 【标注多选】【宽度应用】把requested原始宽度写入state成员与各自工具默认值，record_undo控制历史，dirty返回损坏，返回实际变化
pub(crate) fn set_width(
    state: &mut EditorState,
    requested: i32,
    record_undo: bool,
    dirty: &mut u32,
) -> bool {
    if !active(state) || state.collection.drag.is_some() {
        return false;
    }
    let ids = state.selected_annotation_ids().to_vec();
    if !state.annotations.iter().any(|annotation| {
        ids.contains(&annotation.id) && width(annotation) != clamped_width(annotation, requested)
    }) {
        return false;
    }
    if record_undo {
        state.push_undo();
    }
    let canvas = DrawingSpace::from_editor(state).map(|space| space.bounds);
    state.damage_collection();
    // 1. 【标注多选】【成员顺序】默认值按原版选择编号顺序同步，同一工具分组最后一个成员决定结果
    for id in ids {
        let Some(index) = state
            .annotations
            .iter()
            .position(|annotation| annotation.id == id)
        else {
            continue;
        };
        let value = clamped_width(&state.annotations[index], requested);
        let shape = state.annotations[index].shape.clone();
        sync_defaults(state, &shape, value);
        match &mut state.annotations[index].shape {
            AnnotationShape::Text { .. } => {
                text_width(state, index, value);
                state.tool_settings.font_size = value + 19.0;
            }
            AnnotationShape::Redact { filter } => {
                filter.strength = value;
                state.tool_settings.strength = value;
                rebuild_annotation(state, index);
            }
            _ => {
                state.annotations[index].set_image_stroke_width(value);
                rebuild_annotation(state, index);
            }
        }
        if let Some(canvas) = canvas {
            sync_magnifier_geometry(&mut state.annotations[index], canvas);
        }
    }
    state.damage_collection();
    state.settings_panel.dirty = true;
    apply_damage_rects(state, dirty);
    true
}

/// 【标注多选】【颜色应用】把color或仅其透明度写入state全部成员，record_undo控制事务，dirty返回损坏；返回是否实际变化
pub(crate) fn set_color(
    state: &mut EditorState,
    color: Color,
    opacity_only: bool,
    record_undo: bool,
    dirty: &mut u32,
) -> bool {
    if !active(state) || state.collection.drag.is_some() {
        return false;
    }
    let ids = state.selected_annotation_ids().to_vec();
    let changed = state.annotations.iter().any(|annotation| {
        ids.contains(&annotation.id)
            && if opacity_only {
                annotation.color.alpha() != color.alpha()
            } else {
                annotation.color != color
            }
    });
    let detailed =
        !opacity_only && state.color_popover.open && state.color_popover.radial.is_none();
    if detailed && state.color_popover.pre_edit_snapshot.is_none() {
        state.color_popover.pre_edit_snapshot = Some(state.snapshot());
        // 1. 【标注多选】【颜色历史】原版首次属性输入立即捕获，同色也清除重做，关闭时不能再次提交
        state.push_undo();
        state.color_popover.pre_edit_checkpointed = true;
    } else if !detailed && record_undo && (!opacity_only || changed) {
        state.push_undo();
    }
    if !changed {
        return false;
    }
    state.damage_collection();
    for index in 0..state.annotations.len() {
        if !ids.contains(&state.annotations[index].id) {
            continue;
        }
        if opacity_only {
            state.annotations[index].color.set_alpha(color.alpha());
        } else {
            state.annotations[index].color = color;
        }
        rebuild_annotation(state, index);
    }
    state.damage_collection();
    state.settings_panel.dirty = true;
    apply_damage_rects(state, dirty);
    true
}
