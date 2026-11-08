//! 【取色工具】【点位标注】普通工具创建颜色标签，调色板单次取色只更新绘图颜色，属性面板负责格式复制

use tiny_skia::Color;

use crate::editor::{DamageZone, EditorState};
use crate::tools::ToolBehavior;
use crate::types::{MouseButton, SpecialKey};
use crate::ui::magnifier::magnifier_rect;
use crate::ui::settings_panel::ValueField;
use crate::ui::toast::ToastKind;

pub struct EyedropperTool;

impl ToolBehavior for EyedropperTool {
    /// 【取色工具】【按钮输入】按 button 和 pressed 创建或提交 state 的颜色标签，dirty_mask 返回重绘，无返回值
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        dirty_mask: &mut u32,
    ) {
        if matches!(button, MouseButton::Left) {
            if pressed {
                pick(state, dirty_mask);
            } else {
                commit_sample(state, dirty_mask);
            }
        }
    }

    /// 【取色标注】【标签拖动】按 global 更新 state 中标签位置，dirty_mask 返回旧新范围，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32) {
        if state.input.mouse_down
            && let Some(annotation) = &mut state.pending
            && let crate::types::AnnotationShape::ColorSample { sample } = &mut annotation.shape
        {
            sample.label = (global.0 as f32, global.1 as f32);
            annotation.update_bbox();
            crate::editor::dirty::mark_all_dirty(dirty_mask, state.placements.len());
        }
    }

    fn on_key(&self, state: &mut EditorState, key: SpecialKey, _dirty_mask: &mut u32) {
        if state.input.ctrl && matches!(key, SpecialKey::KeyC) {
            let color = state.tool_settings.color;
            copy_value(state, ValueField::Hex, color);
        }
    }

    fn on_activate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        crate::features::color_sample::inspector::pointer(state, dirty_mask);
        damage_loupe(state, dirty_mask);
    }

    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        crate::features::color_sample::inspector::close(state, dirty_mask);
        crate::editor::drawing_gesture::cancel(state, dirty_mask);
        damage_loupe(state, dirty_mask);
    }
}

/// 【取色标注】【采样开始】按 state 指针冻结颜色；单次调色更新属性，普通工具创建取色标注，dirty_mask 返回重绘，无返回值
pub fn pick(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(color) = color_under_pointer(state) else {
        return;
    };

    if state.pick_once {
        crate::editor::edits::use_color(state, color, dirty_mask);
        end_pick_once(state, dirty_mask);
        return;
    }
    let Some(space) = crate::tools::drawing_space::DrawingSpace::from_editor(state) else {
        return;
    };
    let Some(source) = space.start_point(state.canvas_point()) else {
        return;
    };
    let size = 14.0 * state.annotation_units().canvas_per_image();
    let label = (
        (source.0 + 20.0)
            .min(space.bounds.right() - size * 9.0)
            .max(space.bounds.left()),
        (source.1 + 20.0)
            .min(space.bounds.bottom() - size * 2.2)
            .max(space.bounds.top()),
    );
    let sample = crate::features::color_sample::ColorSample {
        source,
        label,
        color,
        size,
    };
    state.color_inspector.live = Some(color);
    state.color_inspector.remember(color);
    state.pending = Some(crate::types::Annotation {
        id: state.next_id,
        style_scale: state.annotation_units().canvas_per_image(),
        rotation_degrees: 0.0,
        frozen_bounds: Some(space.bounds),
        shape: crate::types::AnnotationShape::ColorSample { sample },
        color,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 1.0,
        bbox: sample.bounds(),
    });
    state.input.mouse_down = true;
    state.tool_active = true;
    crate::editor::dirty::mark_all_dirty(dirty_mask, state.placements.len());
}

/// 【取色标注】【提交采样】提交 state 的颜色草稿到历史与正文，dirty 返回重绘；每个采样点都可以独立撤销和移动
fn commit_sample(state: &mut EditorState, dirty: &mut u32) {
    let Some(annotation) = state.pending.take_if(|annotation| {
        matches!(
            annotation.shape,
            crate::types::AnnotationShape::ColorSample { .. }
        )
    }) else {
        return;
    };
    state.input.mouse_down = false;
    state.tool_active = false;
    state.push_undo();
    state.next_id += 1;
    state.color_popover.record_used_color(annotation.color);
    state.bake_annotation(&annotation);
    let inspect = if let crate::types::AnnotationShape::ColorSample { sample } = annotation.shape {
        let pointer = state.canvas_point();
        (pointer.0 - f64::from(sample.source.0)).hypot(pointer.1 - f64::from(sample.source.1)) < 4.0
    } else {
        false
    };
    state.annotations.push(annotation);
    state.prev_pending = None;
    if inspect {
        crate::features::color_sample::inspector::open(state, dirty);
    }
    crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
}

pub fn toggle_pick_once(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.pick_once {
        end_pick_once(state, dirty_mask);
        return;
    }
    state.pick_once = true;
    state
        .toasts
        .show(ToastKind::PickColor, &mut state.text.font_system);
    damage_loupe(state, dirty_mask);
}

pub fn end_pick_once(state: &mut EditorState, dirty_mask: &mut u32) {
    if !state.pick_once {
        return;
    }
    state.pick_once = false;
    state.toasts.dismiss(ToastKind::PickColor);
    damage_loupe(state, dirty_mask);
}

pub fn copy_value(state: &mut EditorState, field: ValueField, color: Color) {
    let text = field.text(color);
    copy_text(state, &text);
}

/// 【取色面板】【复制文本】将 text 复制到剪贴板并在 state 中反馈结果，不关闭截图或清除标注
pub fn copy_text(state: &mut EditorState, text: &str) {
    let message = match crate::utils::copy_to_clipboard(text) {
        Ok(()) => format!("Copied {text}"),
        Err(error) => {
            log::error!("【取色面板】【复制文本】{error}");
            format!("Couldn't copy: {error}")
        }
    };
    state
        .toasts
        .show_text(ToastKind::ColorCopied, message, &mut state.text.font_system);
}

/// 【取色工具】【原图采样】按 state 指针映射原图像素并反预乘，返回颜色；图片范围外返回空值
pub(crate) fn color_under_pointer(state: &EditorState) -> Option<Color> {
    let point = state.canvas_point();
    let pixel = crate::renderer::FrozenImage::from_parts(
        state.file_canvas.as_ref(),
        &state.captures,
        &state.placements,
    )
    .sample_nearest((point.0 as f32, point.1 as f32))?;
    let pixel = tiny_skia::PremultipliedColorU8::from_rgba(pixel[0], pixel[1], pixel[2], pixel[3])?
        .demultiply();
    Some(Color::from_rgba8(
        pixel.red(),
        pixel.green(),
        pixel.blue(),
        pixel.alpha(),
    ))
}

fn damage_loupe(state: &mut EditorState, dirty_mask: &mut u32) {
    let monitor_idx = state.input.pointer.monitor_idx;
    let Some(placement) = state.placements.get(monitor_idx) else {
        return;
    };
    let (mw, mh) = (placement.size.0 as f32, placement.size.1 as f32);
    let cursor = (
        state.input.pointer.local.0 as f32,
        state.input.pointer.local.1 as f32,
    );
    let rect = magnifier_rect(cursor, mw, mh, true);
    state
        .damage_rects
        .push(DamageZone::Local { monitor_idx, rect });
    crate::editor::dirty::mark_dirty(dirty_mask, monitor_idx);
}
