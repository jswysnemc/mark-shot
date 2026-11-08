//! 【属性面板】【面板同步】管理可见控件、位置与悬停，不承担数值编辑

mod actions;
mod stepper;
mod values;
pub(crate) use stepper::live_apply_stepper_field;

pub use actions::run_settings_action;
pub use stepper::{
    apply_stepper_arrow_step, commit_stepper_text_edit, handle_settings_key_press,
    handle_settings_text_input, handle_stepper_scroll, sync_stepper_edit_text,
    tick_stepper_arrow_hold,
};
pub use values::{apply_toggle_field, current_color, current_outline_color, swatch_color};

use super::color::close_color_popover;
use crate::editor::EditorState;
use crate::editor::edits::active_annotation_idx;
use crate::tools::Tool;
use crate::types::AnnotationShape;
use crate::ui::color_popover::ColorTarget;
use crate::ui::panel::{UiPanel, sync_panel_hover, sync_panel_rect};
use crate::ui::settings_panel::{
    OCR_AWAITING_WIDGETS, OCR_DOWNLOADING_WIDGETS, OCR_NO_MODEL_WIDGETS, OCR_SCANNING_WIDGETS,
    OCR_WIDGETS, OCR_WIDGETS_DOWNLOADING, SettingsSource, SettingsWidget, ToggleField,
    compute_settings_placement, widgets_for_annotation, widgets_for_tool,
};
use stepper::format_stepper_number;
pub(crate) use values::stepper_value;

/// 【属性面板】【面板同步】根据 editor_state 同步属性内容与可见位置；dirty_mask 返回重绘显示器，无返回值
pub fn update_settings_panel(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    if crate::editor::annotation_collection::properties::active(editor_state) {
        let text = crate::ui::annotation_collection::layout::title(
            editor_state.selected_annotation_ids().len(),
        );
        if editor_state
            .settings_panel
            .collection_title
            .as_ref()
            .is_none_or(|title| title.text != text)
        {
            editor_state.settings_panel.collection_title =
                Some(crate::ui::annotation_collection::title::Title::new(
                    text,
                    &mut editor_state.text.font_system,
                ));
        }
    }
    crate::editor::text_session::sync_style(editor_state, dirty_mask);
    if editor_state
        .text_session
        .as_ref()
        .is_some_and(|session| session.properties_hidden)
    {
        let old = editor_state.settings_panel.rect();
        editor_state.settings_panel.visible = false;
        if let Some(rect) = old {
            crate::ui::panel::emit_panel_damage(
                rect,
                editor_state.settings_panel.monitor_idx,
                &mut editor_state.damage_rects,
                dirty_mask,
            );
        }
        return;
    }
    let filter_mode = crate::editor::filter_edits::current_mode(editor_state);
    if editor_state.settings_panel.filter_mode != filter_mode {
        editor_state.settings_panel.filter_mode = filter_mode;
        editor_state.settings_panel.dirty = true;
    }
    let arrow_style = crate::editor::line_edits::current_head(editor_state);
    let line_style = crate::editor::line_edits::current_style(editor_state);
    if editor_state.settings_panel.arrow_style != arrow_style
        || editor_state.settings_panel.line_style != line_style
    {
        editor_state.settings_panel.arrow_style = arrow_style;
        editor_state.settings_panel.line_style = line_style;
        editor_state.settings_panel.dirty = true;
    }
    let number_style = crate::editor::number_edits::current_style(editor_state);
    if editor_state.settings_panel.number_style != number_style {
        editor_state.settings_panel.number_style = number_style;
        editor_state.settings_panel.dirty = true;
    }
    if editor_state.settings_panel.ruler_mode != editor_state.tool_settings.ruler_mode {
        editor_state.settings_panel.ruler_mode = editor_state.tool_settings.ruler_mode;
        editor_state.settings_panel.dirty = true;
    }
    let ann_idx = active_annotation_idx(editor_state);
    let selected_ann = ann_idx.and_then(|i| editor_state.annotations.get(i));
    editor_state.settings_panel.text_background = match selected_ann.map(|ann| &ann.shape) {
        Some(AnnotationShape::Text { appearance, .. }) => appearance.background,
        _ => editor_state.tool_settings.text_background,
    };

    let (marker_shape, marker_filled) = match selected_ann.map(|annotation| &annotation.shape) {
        Some(AnnotationShape::Marker { kind, filled, .. }) => (*kind, *filled),
        _ => (
            editor_state.tool_settings.marker_shape,
            editor_state.tool_settings.marker_filled,
        ),
    };
    if editor_state.settings_panel.marker_shape != marker_shape
        || editor_state.settings_panel.marker_filled != marker_filled
    {
        editor_state.settings_panel.marker_shape = marker_shape;
        editor_state.settings_panel.marker_filled = marker_filled;
        editor_state.settings_panel.dirty = true;
    }

    // A running scan owns the panel outright: its buttons act on a result that
    // isn't there yet.
    let ocr = editor_state.selected_tool == Tool::Ocr;
    let scanning = ocr && editor_state.ocr.runtime.is_busy();
    let awaiting = ocr && editor_state.ocr.await_region;
    let no_model = ocr && crate::ocr::engine_choice::source(&editor_state.ocr.models).is_none();
    let downloading = editor_state.ocr.models.is_downloading();

    let new_source = match selected_ann {
        _ if no_model => SettingsSource::OcrNoModel { downloading },
        _ if scanning => SettingsSource::OcrScanning,
        _ if awaiting => SettingsSource::OcrAwaiting { downloading },
        _ if ocr => SettingsSource::OcrResult { downloading },
        _ if crate::editor::annotation_collection::properties::active(editor_state) => {
            SettingsSource::Collection {
                revision: editor_state.collection.revision,
                count: editor_state.selected_annotation_ids().len(),
            }
        }
        Some(ann) => SettingsSource::Annotation(ann.id),
        None => SettingsSource::Tool(editor_state.selected_tool),
    };

    let mut new_widgets = match new_source {
        SettingsSource::Collection { .. } => &crate::ui::settings_panel::TEXT_WIDGETS[..3],
        SettingsSource::OcrScanning => OCR_SCANNING_WIDGETS,
        SettingsSource::OcrNoModel { downloading: false } => OCR_NO_MODEL_WIDGETS,
        SettingsSource::OcrAwaiting { downloading: false } => OCR_AWAITING_WIDGETS,
        SettingsSource::OcrNoModel { downloading: true }
        | SettingsSource::OcrAwaiting { downloading: true } => OCR_DOWNLOADING_WIDGETS,
        SettingsSource::OcrResult { downloading: false } => OCR_WIDGETS,
        SettingsSource::OcrResult { downloading: true } => OCR_WIDGETS_DOWNLOADING,
        SettingsSource::Annotation(_) => selected_ann.map_or(&[][..], widgets_for_annotation),
        SettingsSource::Tool(tool) => widgets_for_tool(tool),
    };
    let text = matches!(
        selected_ann.map(|ann| &ann.shape),
        Some(AnnotationShape::Text { .. })
    ) || (selected_ann.is_none() && editor_state.selected_tool == Tool::Text);
    let hide_radius = match selected_ann.map(|annotation| &annotation.shape) {
        Some(AnnotationShape::Marker { kind, .. }) => *kind != crate::types::MarkerShape::Square,
        Some(AnnotationShape::Magnifier { magnifier }) => {
            magnifier.shape != crate::features::magnifier::MagnifierShape::Rectangle
        }
        None if editor_state.selected_tool == Tool::Marker => {
            editor_state.tool_settings.marker_shape != crate::types::MarkerShape::Square
        }
        None if editor_state.selected_tool == Tool::Magnifier => {
            editor_state.tool_settings.magnifier_shape
                != crate::features::magnifier::MagnifierShape::Rectangle
        }
        _ => false,
    };
    if hide_radius
        && matches!(
            new_widgets.last(),
            Some(SettingsWidget::Stepper {
                range: crate::ui::settings_panel::StepperRange::CornerRadius,
                ..
            })
        )
    {
        new_widgets = &new_widgets[..new_widgets.len() - 1];
    }
    let text_layout = if let SettingsSource::Collection { .. } = new_source {
        Some(crate::ui::annotation_collection::layout::new(
            editor_state
                .settings_panel
                .collection_title
                .as_ref()
                .map_or(58.0, |title| title.cell_width),
        ))
    } else {
        text.then(|| {
            crate::ui::text_properties::layout::TextLayout::new(
                30.0,
                11.0,
                22.0,
                selected_ann.is_some() && editor_state.selected_tool == Tool::Pick,
            )
        })
    };
    if text_layout.is_some_and(|layout| layout.controls[5].is_some()) {
        new_widgets = crate::ui::settings_panel::TEXT_EDIT_WIDGETS;
    }
    let layout_changed = editor_state.settings_panel.text_layout != text_layout
        || editor_state.settings_panel.widgets.len() != new_widgets.len();
    editor_state.settings_panel.text_layout = text_layout;
    editor_state.settings_panel.text_points = selected_ann
        .and_then(|ann| match &ann.shape {
            AnnotationShape::Text { font_size, .. } => Some(*font_size),
            _ => None,
        })
        .unwrap_or(editor_state.tool_settings.font_size);
    if crate::editor::annotation_collection::properties::active(editor_state) {
        editor_state.settings_panel.text_points =
            crate::editor::annotation_collection::properties::first(editor_state).map_or(
                20.0,
                |annotation| {
                    crate::editor::annotation_collection::properties::width(annotation) + 19.0
                },
            );
    }
    editor_state.settings_panel.text_alpha = (selected_ann
        .or_else(|| {
            crate::editor::annotation_collection::properties::active(editor_state)
                .then(|| crate::editor::annotation_collection::properties::first(editor_state))
                .flatten()
        })
        .map_or(editor_state.tool_settings.color, |ann| ann.color)
        .alpha()
        * 100.0)
        .round() as i32;
    let source_changed = editor_state.settings_panel.active_source != Some(new_source);

    let old_rect = editor_state.settings_panel.rect();
    let old_monitor = editor_state.settings_panel.monitor_idx;

    if source_changed || layout_changed {
        editor_state.settings_panel.widgets = new_widgets;
        editor_state.settings_panel.active_source = Some(new_source);
        editor_state.settings_panel.size.0 = editor_state.settings_panel.width();
        editor_state.settings_panel.size = text_layout.map_or(
            (
                editor_state.settings_panel.size.0,
                crate::theme::size::panel_height(),
            ),
            |layout| layout.size,
        );
        editor_state.settings_panel.fields.values.clear();
    }

    let widgets = editor_state.settings_panel.widgets;
    for (idx, widget) in widgets.iter().enumerate() {
        match widget {
            SettingsWidget::Stepper { range, .. } => {
                let value = stepper_value(editor_state, *range);
                editor_state
                    .settings_panel
                    .sync_value(idx, format_stepper_number(value, *range));
            }
            SettingsWidget::Toggle { field, .. } => {
                let value = match selected_ann {
                    Some(ann) => match (&ann.shape, field) {
                        (
                            AnnotationShape::Rectangle { fixed_ratio, .. }
                            | AnnotationShape::Circle { fixed_ratio, .. },
                            ToggleField::FixedRatio,
                        ) => *fixed_ratio,
                        (AnnotationShape::Text { bold, .. }, ToggleField::Bold) => *bold,
                        (AnnotationShape::Text { italic, .. }, ToggleField::Italic) => *italic,
                        (AnnotationShape::Text { outline, .. }, ToggleField::Outline) => {
                            outline.is_some()
                        }
                        (
                            AnnotationShape::Rectangle { filled, .. }
                            | AnnotationShape::Circle { filled, .. },
                            ToggleField::Fill,
                        ) => *filled,
                        (AnnotationShape::Marker { filled, .. }, ToggleField::MarkerFill) => {
                            *filled
                        }
                        (
                            AnnotationShape::Magnifier { magnifier },
                            ToggleField::MagnifierRectangle,
                        ) => {
                            magnifier.shape == crate::features::magnifier::MagnifierShape::Rectangle
                        }
                        _ => false,
                    },
                    None => match field {
                        ToggleField::FixedRatio => editor_state.tool_settings.fixed_ratio,
                        ToggleField::Bold => editor_state.tool_settings.bold,
                        ToggleField::Italic => editor_state.tool_settings.italic,
                        ToggleField::Outline => editor_state.tool_settings.outline,
                        ToggleField::Fill => editor_state.tool_settings.fill,
                        ToggleField::MarkerFill => editor_state.tool_settings.marker_filled,
                        ToggleField::MagnifierRectangle => {
                            editor_state.tool_settings.magnifier_shape
                                == crate::features::magnifier::MagnifierShape::Rectangle
                        }
                    },
                };
                editor_state.settings_panel.set_toggled(idx, value);
            }
            _ => {}
        }
    }

    if source_changed
        && editor_state.color_popover.open
        && ((editor_state.color_popover.target == ColorTarget::Outline
            && !editor_state.settings_panel.has_outline_swatch())
            || (editor_state.color_popover.target == ColorTarget::TextBackground
                && !new_widgets
                    .iter()
                    .any(|widget| widget.color_target() == Some(ColorTarget::TextBackground))))
    {
        close_color_popover(editor_state, dirty_mask);
    }

    let should_be_visible =
        !new_widgets.is_empty() && !editor_state.tool_active && !editor_state.ui_hidden;

    if should_be_visible {
        let (pos, monitor_idx) = compute_settings_placement(editor_state);
        editor_state.settings_panel.position = pos;
        editor_state.settings_panel.render_pos = pos;
        editor_state.settings_panel.monitor_idx = monitor_idx;
        editor_state.settings_panel.visible = true;
    } else {
        editor_state.settings_panel.visible = false;
        editor_state.settings_panel.arrow_held = None;
    }

    let download = editor_state.ocr.models.download_progress();
    let download_changed = editor_state.settings_panel.download != download;
    editor_state.settings_panel.download = download;

    sync_panel_rect(
        &mut editor_state.settings_panel,
        old_rect,
        old_monitor,
        source_changed || layout_changed || download_changed,
        &mut editor_state.damage_rects,
        dirty_mask,
    );

    if editor_state.settings_panel.rect().is_some() {
        let prev_hover = editor_state.settings_panel.hovered;
        let (_, hovered) = editor_state
            .settings_panel
            .hit_test(editor_state.input.pointer.local);
        let hovered_arrow = hovered.and_then(|idx| {
            editor_state
                .settings_panel
                .stepper_arrow_hit(idx, editor_state.input.pointer.local)
                .map(|arrow| (idx, arrow))
        });

        if hovered != prev_hover
            && !editor_state.settings_panel.is_editing()
            && editor_state.input.text_property_press.is_none()
        {
            let snapshot = editor_state.settings_panel.pre_edit_snapshot.take();
            editor_state.commit_snapshot(snapshot);
        }

        sync_panel_hover(
            &mut editor_state.settings_panel,
            (hovered, hovered_arrow),
            &mut editor_state.damage_rects,
            dirty_mask,
        );

        if let Some(hold) = editor_state.settings_panel.arrow_held.as_ref() {
            let still_on_arrow = hovered_arrow == Some((hold.widget_idx, hold.arrow));
            if !still_on_arrow {
                editor_state.settings_panel.arrow_held = None;
            }
        }
    } else if !editor_state.settings_panel.is_editing() {
        let snapshot = editor_state.settings_panel.pre_edit_snapshot.take();
        editor_state.commit_snapshot(snapshot);
    }
}

/// 【属性面板】【弹层位置】根据 editor_state 与 width、height、offset 计算弹层位置；返回显示器本地坐标和显示器索引
pub fn compute_popover_placement(
    editor_state: &EditorState,
    (width, height): (f32, f32),
    offset: f32,
) -> ((f32, f32), usize) {
    let sp = &editor_state.settings_panel;
    let tb = &editor_state.toolbar;
    let monitor_idx = sp.monitor_idx;

    let monitor_width = editor_state.placements[monitor_idx].size.0 as f32;
    let monitor_height = editor_state.placements[monitor_idx].size.1 as f32;

    let sp_bottom = sp.render_pos.1 + sp.size.1;
    let tb_bottom = tb.render_pos.1 + tb.size.1;
    let combined_bottom = sp_bottom.max(tb_bottom);

    let mut side_y = combined_bottom - height;

    if side_y < offset {
        side_y = offset;
    }
    if side_y + height > monitor_height - offset {
        side_y = monitor_height - height - offset;
    }

    let x_left = sp.render_pos.0 - width - offset;
    let x_right = sp.render_pos.0 + sp.size.0 + offset;

    let space_left = x_left >= offset;
    let space_right = x_right + width <= monitor_width - offset;

    if space_left {
        return ((x_left, side_y), monitor_idx);
    } else if space_right {
        return ((x_right, side_y), monitor_idx);
    }

    let mut final_x = sp.render_pos.0;

    if final_x < offset {
        final_x = offset;
    }
    if final_x + width > monitor_width - offset {
        final_x = monitor_width - width - offset;
    }

    let y_below = sp.render_pos.1 + sp.size.1 + offset;
    let y_above = sp.render_pos.1 - offset - height;

    let space_below = y_below + height <= monitor_height;
    let space_above = y_above >= 0.0;

    let sp_is_below_tb = sp.render_pos.1 >= tb.render_pos.1;

    let final_y = if sp_is_below_tb {
        if space_below {
            y_below
        } else if space_above {
            y_above
        } else {
            monitor_height - height - offset
        }
    } else {
        if space_above {
            y_above
        } else if space_below {
            y_below
        } else {
            offset
        }
    };

    ((final_x, final_y), monitor_idx)
}
