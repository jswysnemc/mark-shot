//! 【属性面板】【数值读写】统一读取当前属性并提交标注与工具设置

use super::update_settings_panel;
use crate::editor::EditorState;
use crate::editor::edits::{active_annotation_idx, commit_settings_change};
use crate::types::{AnnotationShape, StrokeGroup};
use crate::ui::color_popover::ColorTarget;
use crate::ui::settings_panel::{StepperRange, ToggleField};

/// 【属性面板】【数值读写】根据 editor_state 返回选中标注或工具颜色
pub fn current_color(editor_state: &EditorState) -> tiny_skia::Color {
    if crate::editor::annotation_collection::properties::active(editor_state) {
        return crate::editor::annotation_collection::properties::first(editor_state)
            .map_or(editor_state.tool_settings.color, |annotation| {
                annotation.color
            });
    }
    active_annotation_idx(editor_state)
        .and_then(|idx| editor_state.annotations.get(idx))
        .map_or(editor_state.tool_settings.color, |ann| ann.color)
}

/// 【属性面板】【数值读写】根据 editor_state 返回文字描边颜色
pub fn current_outline_color(editor_state: &EditorState) -> tiny_skia::Color {
    let ann = active_annotation_idx(editor_state).and_then(|idx| editor_state.annotations.get(idx));
    match ann.map(|ann| &ann.shape) {
        Some(AnnotationShape::Text {
            outline: Some(color),
            ..
        }) => *color,
        _ => editor_state.tool_settings.outline_color,
    }
}

/// 【属性面板】【数值读写】根据 editor_state 与 target 返回属性色块颜色
pub fn swatch_color(editor_state: &EditorState, target: ColorTarget) -> tiny_skia::Color {
    match target {
        ColorTarget::Main => current_color(editor_state),
        ColorTarget::Outline => current_outline_color(editor_state),
        ColorTarget::TextBackground => active_annotation_idx(editor_state)
            .and_then(|index| match &editor_state.annotations[index].shape {
                AnnotationShape::Text { appearance, .. } => Some(appearance.background),
                _ => None,
            })
            .unwrap_or(editor_state.tool_settings.text_background),
    }
}

/// 【属性面板】【数值读写】根据 editor_state 返回当前笔宽所属工具分组
pub(super) fn stroke_group(editor_state: &EditorState) -> StrokeGroup {
    match active_annotation_idx(editor_state).and_then(|i| editor_state.annotations.get(i)) {
        Some(ann) => StrokeGroup::of_shape(&ann.shape),
        None => StrokeGroup::of_tool(editor_state.selected_tool),
    }
}

/// 【属性面板】【数值读写】将 range 对应 new_value 写入 editor_state；record_undo 决定历史记录，dirty_mask 返回重绘显示器
pub(super) fn apply_stepper_field(
    editor_state: &mut EditorState,
    range: StepperRange,
    new_value: f32,
    record_undo: bool,
    dirty_mask: &mut u32,
) {
    if range == StepperRange::Font {
        crate::editor::text_edits::set(
            editor_state,
            crate::editor::text_edits::Property::Points(new_value),
            record_undo,
            dirty_mask,
        );
        return;
    }
    if range == StepperRange::Magnification {
        crate::editor::magnifier_edits::set_zoom(editor_state, new_value, record_undo, dirty_mask);
        return;
    }
    if range == StepperRange::Strength {
        crate::editor::filter_edits::set_strength(editor_state, new_value, record_undo, dirty_mask);
        return;
    }
    let current = stepper_value(editor_state, range);
    let group = stroke_group(editor_state);
    let changed = (current - new_value).abs() > f32::EPSILON;

    commit_settings_change(
        editor_state,
        changed,
        record_undo,
        move |ts| match range {
            StepperRange::Stroke | StepperRange::NumberWidth | StepperRange::LineWidth => {
                ts.set_stroke(group, new_value)
            }
            StepperRange::Font => ts.font_size = new_value,
            StepperRange::Strength => ts.strength = new_value,
            StepperRange::Magnification => {}
        },
        move |ann| match &mut ann.shape {
            AnnotationShape::Text { font_size, .. } => *font_size = new_value,
            AnnotationShape::Redact { filter } => filter.strength = new_value,
            _ => ann.set_image_stroke_width(new_value),
        },
        dirty_mask,
    );
}

/// 【属性面板】【数值读写】将 field 对应 new_value 写入 editor_state 并刷新属性；dirty_mask 返回重绘显示器
pub fn apply_toggle_field(
    editor_state: &mut EditorState,
    field: ToggleField,
    new_value: bool,
    dirty_mask: &mut u32,
) {
    if matches!(field, ToggleField::Bold | ToggleField::Italic) {
        let property = if field == ToggleField::Bold {
            crate::editor::text_edits::Property::Bold(new_value)
        } else {
            crate::editor::text_edits::Property::Italic(new_value)
        };
        crate::editor::text_edits::set(editor_state, property, true, dirty_mask);
        update_settings_panel(editor_state, dirty_mask);
        return;
    }
    if field == ToggleField::MagnifierRectangle {
        let shape = if new_value {
            crate::features::magnifier::MagnifierShape::Rectangle
        } else {
            crate::features::magnifier::MagnifierShape::Circle
        };
        crate::editor::magnifier_edits::set_shape(editor_state, shape, dirty_mask);
        update_settings_panel(editor_state, dirty_mask);
        return;
    }
    let outline_color = editor_state.tool_settings.outline_color;
    commit_settings_change(
        editor_state,
        true,
        true,
        move |ts| match field {
            ToggleField::Bold => ts.bold = new_value,
            ToggleField::Italic => ts.italic = new_value,
            ToggleField::Outline => ts.outline = new_value,
            ToggleField::Fill => ts.fill = new_value,
            ToggleField::MarkerFill => ts.marker_filled = new_value,
            ToggleField::MagnifierRectangle => {}
        },
        move |ann| match (&mut ann.shape, field) {
            (AnnotationShape::Text { bold, .. }, ToggleField::Bold) => *bold = new_value,
            (AnnotationShape::Text { italic, .. }, ToggleField::Italic) => *italic = new_value,
            (AnnotationShape::Text { outline, .. }, ToggleField::Outline) => {
                *outline = new_value.then_some(outline_color)
            }
            (
                AnnotationShape::Rectangle { filled, .. } | AnnotationShape::Circle { filled, .. },
                ToggleField::Fill,
            ) => *filled = new_value,
            (AnnotationShape::Marker { filled, .. }, ToggleField::MarkerFill) => {
                *filled = new_value
            }
            _ => {}
        },
        dirty_mask,
    );
    // 1. 【属性面板】【数值读写】互斥模式按钮需要同步显示状态
    update_settings_panel(editor_state, dirty_mask);
    if field == ToggleField::MarkerFill {
        super::super::toolbar::update_toolbar(editor_state, dirty_mask);
    }
}

/// 【属性面板】【数值读写】根据 editor_state 与 range 返回当前标注或工具数值
pub(crate) fn stepper_value(editor_state: &EditorState, range: StepperRange) -> f32 {
    if range == StepperRange::Magnification {
        return crate::editor::magnifier_edits::current_zoom(editor_state);
    }
    let ts = &editor_state.tool_settings;
    match active_annotation_idx(editor_state).and_then(|i| editor_state.annotations.get(i)) {
        Some(ann) => match &ann.shape {
            AnnotationShape::Text { font_size, .. } => *font_size,
            AnnotationShape::Redact { filter } => filter.strength,
            _ => ann.image_stroke_width(),
        },
        None => match range {
            StepperRange::Stroke | StepperRange::NumberWidth | StepperRange::LineWidth => {
                ts.stroke(stroke_group(editor_state))
            }
            StepperRange::Font => ts.font_size,
            StepperRange::Strength => ts.strength,
            StepperRange::Magnification => ts.magnifier_zoom,
        },
    }
}
