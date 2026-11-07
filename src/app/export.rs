//! 【截图】【导出合成】刷新编辑属性并合成最终图像

use super::*;

/// Undo and redo can change what the panels show, so refresh is needed
pub(super) fn refresh_panels_after_history(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    panels::settings::update_settings_panel(editor_state, dirty_mask);
    editor_state.settings_panel.dirty = true;
    let sp_mon = editor_state.settings_panel.monitor_idx;
    if let Some(rect) = editor_state.settings_panel.rect() {
        editor_state.damage_local(sp_mon, rect);
    }
    mark_dirty(dirty_mask, sp_mon);

    if !editor_state.color_popover.open {
        return;
    }

    // checks is it for outline or text color
    if crate::editor::edits::active_annotation_idx(editor_state).is_some() {
        let color = panels::settings::swatch_color(editor_state, editor_state.color_popover.target);
        editor_state.color_popover.select_color(color);
    }
    panels::color::update_color_popover(editor_state, dirty_mask);
    editor_state.color_popover.dirty = true;
    let cp_mon = editor_state.color_popover.monitor_idx;
    if let Some(rect) = editor_state.color_popover.rect() {
        editor_state.damage_local(cp_mon, rect);
    }
    mark_dirty(dirty_mask, cp_mon);
}

pub fn selection_render_info(
    selection: &Option<Rect>,
    prev_selection: &Option<Rect>,
    placement: &Placement,
) -> (Option<Rect>, Option<Rect>, Option<SelectionEdges>) {
    let local_sel = selection
        .as_ref()
        .and_then(|sel| global_selection_to_local(sel, placement));
    let prev_local = prev_selection
        .as_ref()
        .and_then(|sel| global_selection_to_local(sel, placement));

    let mut edges = None;
    if let (Some(sel), Some(_)) = (selection.as_ref(), local_sel.as_ref()) {
        edges = Some(selection_edges_for_monitor(sel, placement));
    }
    (local_sel, prev_local, edges)
}

/// Returns the final PNG and the global position of its top-left corner.
/// position is required for the pin feature ^^^
pub(crate) fn render_final(editor_state: &mut EditorState) -> Option<(Vec<u8>, (i32, i32), f32)> {
    let (png, origin, scale) = render_final_precise(editor_state)?;
    Some((
        png,
        (origin.0.round() as i32, origin.1.round() as i32),
        scale,
    ))
}

/// 【截图导出】【精确原点】按 editor_state 返回物理裁切 PNG、双精度全局原点和比例，标注与译文交接共用原点
pub(in crate::app) fn render_final_precise(
    editor_state: &mut EditorState,
) -> Option<(Vec<u8>, (f64, f64), f32)> {
    crate::editor::text_session::commit(editor_state, &mut 0);
    if editor_state.file_canvas.is_some() {
        return super::image_export::render_precise(editor_state);
    }
    let sel = match editor_state.selection.zone {
        Some(s) => s,
        None => get_full_workspace_rect(&editor_state.placements)?,
    };

    let scale = renderer::capture_scale(&editor_state.captures, &editor_state.placements, sel);
    let (mut out, origin, _) = renderer::composite_precise(
        &editor_state.captures,
        &editor_state.placements,
        sel,
        Some(scale),
    )?;

    let offset = (
        (origin.0 * f64::from(scale)) as f32,
        (origin.1 * f64::from(scale)) as f32,
    );
    let mut scaled_editors = HashMap::new();
    // 1. 【截图】【原图滤镜】导出重新读取冻结原图，预览缓存和先前标注都不会成为滤镜来源
    let in_place = HashMap::new();
    for ann in &editor_state.annotations {
        let scaled;
        let (ann, editors) = if scale == 1.0 {
            (ann, &mut editor_state.text.editors)
        } else {
            scaled = ann.scaled(scale);
            (&scaled, &mut scaled_editors)
        };
        renderer::draw_annotation(
            &mut renderer::Painter::new(&mut out)
                .with_frozen_image(
                    renderer::FrozenImage::desktop(
                        &editor_state.captures,
                        &editor_state.placements,
                    )
                    .mapped(scale, (0.0, 0.0)),
                )
                .with_drawing_bounds(renderer::scale_rect(&sel, scale))
                .with_text(
                    &mut editor_state.text.font_system,
                    &mut editor_state.text.swash_cache,
                )
                .with_origin(offset),
            ann,
            false,
            editors,
            &in_place,
            None,
        );
    }

    if let Some(bounds) = renderer::scale_rect(&sel, scale) {
        let rulers = editor_state.annotations.iter().filter_map(|annotation| {
            if let crate::types::AnnotationShape::Ruler { measurement } = annotation.shape {
                Some((measurement.scaled(scale), annotation.stroke_width * scale))
            } else {
                None
            }
        });
        let labels = crate::features::ruler::labels::prepare_all(
            rulers,
            bounds,
            &mut editor_state.text.font_system,
        );
        renderer::ruler_overlay::draw_labels(
            &mut renderer::Painter::new(&mut out)
                .with_origin(offset)
                .with_text(
                    &mut editor_state.text.font_system,
                    &mut editor_state.text.swash_cache,
                ),
            &labels,
        );
    }

    Some((encode_png(&out), origin, scale))
}
