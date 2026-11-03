//! 【界面】【帧绘制】按显示器回调节奏合成并提交损坏区域

use super::*;

/// what has to be redrawed
#[derive(Clone, Copy)]
pub(super) enum Redraw {
    Area(Rect),
    Whole,
}

impl Redraw {
    pub(super) fn join(self, other: Redraw) -> Redraw {
        match (self, other) {
            (Redraw::Area(a), Redraw::Area(b)) => a.join(&b).map_or(Redraw::Whole, Redraw::Area),
            _ => Redraw::Whole,
        }
    }

    pub(super) fn area(self) -> Option<Rect> {
        match self {
            Redraw::Area(rect) => Some(rect),
            Redraw::Whole => None,
        }
    }
}

/// every monitor draws on its own frame callbacks
/// until then the damage piles up in `owed`
pub(super) struct Pacing {
    pub(super) owed: Vec<Option<Redraw>>,
    pub(super) dimmed_for: Vec<Option<Rect>>,
}

impl Pacing {
    pub(super) fn new(monitors: usize) -> Self {
        Self {
            owed: vec![None; monitors],
            dimmed_for: vec![None; monitors],
        }
    }

    pub(super) fn owes_to(&self, monitor_idx: usize) -> bool {
        self.owed[monitor_idx].is_some()
    }
}

/// What every monitor of one frame draws from.
pub(super) struct Frame {
    pub(super) active_text_id: Option<u64>,
    pub(super) scan_badge: Option<(Rect, f32)>,
    pub(super) current_color: tiny_skia::Color,
    pub(super) outline_color: tiny_skia::Color,
    pub(super) annotations_hidden: bool,
}

pub(super) fn render_dirty_monitors(
    editor_state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    mut dirty_mask: u32,
    annotations_hidden: bool,
    pacing: &mut Pacing,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::editor::text_session::sync_style(editor_state, &mut dirty_mask);
    crate::editor::text_composition::refresh_style(editor_state, &mut dirty_mask);
    crate::editor::ruler_labels::refresh(editor_state, &mut dirty_mask);
    let frame = Frame {
        active_text_id: active_text_id(editor_state),
        scan_badge: editor_state.ocr.scan_started.and_then(|started| {
            Some((
                editor_state.ocr.view.region()?,
                started.elapsed().as_secs_f32(),
            ))
        }),
        current_color: panels::settings::current_color(editor_state),
        outline_color: panels::settings::current_outline_color(editor_state),
        annotations_hidden,
    };
    editor_state.refresh_filters();

    for i in 0..editor_state.captures.len() {
        if is_dirty(dirty_mask, i) {
            rebuild_layer(editor_state, i, &frame);
            let area = editor_state
                .monitor_dirty_rect(i)
                .map_or(Redraw::Whole, Redraw::Area);
            pacing.owed[i] = Some(pacing.owed[i].map_or(area, |owed| owed.join(area)));
        }
        let Some(owed) = pacing.owed[i] else {
            continue;
        };
        // the layer is rebuilt right away, as its damage will be cleared afterwards
        // (only the composite and the commit wait for the frame callback)
        if !overlay.ready(i) {
            continue;
        }
        pacing.owed[i] = None;
        let damage = render_monitor(
            editor_state,
            i,
            &frame,
            owed.area(),
            &mut pacing.dimmed_for[i],
        );
        overlay.stage_frame(i, editor_state.canvas[i].data(), damage)?;
    }
    overlay.flush()?;

    Ok(())
}

/// 【文字绘制】【活动身份】从 state 返回当前候选或正文的显示身份，输入和撤销仍使用真实编辑器身份
pub(super) fn active_text_id(state: &EditorState) -> Option<u64> {
    state
        .text_session
        .as_ref()
        .map(|session| session.presented().id)
        .or_else(|| {
            state
                .text
                .editing
                .as_ref()
                .map(|editing| editing.annotation_id)
        })
}

pub(super) fn rebuild_layer(editor_state: &mut EditorState, i: usize, _frame: &Frame) {
    let layer_dirty = editor_state.monitor_layer_dirty_rect(i);
    if layer_dirty.is_none() && !editor_state.annotations_dirty {
        return;
    }
    let placement = &editor_state.placements[i];
    renderer::rebuild_annotations_layer(
        &mut renderer::Painter::new(&mut editor_state.annotations_layer[i])
            .with_scale(placement.scale)
            .with_frozen_image(renderer::FrozenImage::from_parts(
                editor_state.file_canvas.as_ref(),
                &editor_state.captures,
                &editor_state.placements,
            ))
            .with_drawing_bounds(editor_state.ruler.label_bounds)
            .with_text(
                &mut editor_state.text.font_system,
                &mut editor_state.text.swash_cache,
            )
            .with_origin(placement.offset()),
        &editor_state.annotations,
        &mut editor_state.text.editors,
        &editor_state.filters.images,
        editor_state
            .text_session
            .as_ref()
            .and_then(|session| session.original_id),
        layer_dirty,
    );
}

/// Draws one monitor's canvas and returns the region that changed
pub(super) fn render_monitor(
    editor_state: &mut EditorState,
    i: usize,
    frame: &Frame,
    dirty_rect: Option<Rect>,
    dimmed_for: &mut Option<Rect>,
) -> Option<DamageRect> {
    // No loupe while reading text: it sits right where the
    // pointer is selecting and hides the line under it.
    let picking = editor_state.picking();
    let is_mag_monitor = editor_state.selected_tool != Tool::Ocr
        && editor_state.magnifier_shown()
        && editor_state
            .magnifier
            .current
            .as_ref()
            .is_some_and(|m| m.monitor_idx == i);

    // to not redraw the whole dim layer on every frame, only the changed part is redrawn;
    // that's why each monitor has to remember its own previous selection,
    // to compare the new one against it
    //
    // (a monitor may skip frames, so the shared prev_zone can be outdated for it)
    let zone = editor_state.selection.zone;
    let (local_sel, prev_local, edges) =
        selection_render_info(&zone, dimmed_for, &editor_state.placements[i]);
    let selection_dirty = *dimmed_for != zone;
    *dimmed_for = zone;

    let offset = editor_state.placements[i].offset();
    let scale = editor_state.placements[i].scale;
    let text_caret_visible = super::text_caret::visible(editor_state);

    let damage = dirty_rect.as_ref().and_then(|r| {
        renderer::rect_bounds(
            &renderer::scale_rect(r, scale)?,
            editor_state.canvas[i].width(),
            editor_state.canvas[i].height(),
        )
    });

    let dragged_index = editor_state.dragged_index();
    let dirty = dirty_rect.as_ref();
    let toolbar = panel_to_draw(&mut editor_state.toolbar, i, dirty);
    let action_toolbar = panel_to_draw(&mut editor_state.action_toolbar, i, dirty);
    let settings_panel = panel_to_draw(&mut editor_state.settings_panel, i, dirty);
    let font_panel = panel_to_draw(&mut editor_state.font_panel, i, dirty);
    let color_picker = panel_to_draw(&mut editor_state.color_popover, i, dirty);
    let model_list = panel_to_draw(&mut editor_state.model_popover, i, dirty);
    let marker_popover = panel_to_draw(&mut editor_state.marker_popover, i, dirty);
    let ruler_popover = panel_to_draw(&mut editor_state.ruler_popover, i, dirty);
    let number_popover = panel_to_draw(&mut editor_state.number_popover, i, dirty);
    let arrow_popover = panel_to_draw(&mut editor_state.arrow_popover, i, dirty);
    let line_popover = panel_to_draw(&mut editor_state.line_popover, i, dirty);
    let filter_popover = panel_to_draw(&mut editor_state.filter_popover, i, dirty);

    let hidden = frame.annotations_hidden;

    renderer::render_frame(&mut renderer::RenderRequest {
        canvas: &mut editor_state.canvas[i],
        capture: &editor_state.captures[i],
        frozen_image: renderer::FrozenImage::from_parts(
            editor_state.file_canvas.as_ref(),
            &editor_state.captures,
            &editor_state.placements,
        ),
        dimmed: &mut editor_state.dimmed[i],
        selection: local_sel.as_ref(),
        prev_selection: prev_local.as_ref(),
        dirty_rect: dirty_rect.as_ref(),
        selection_dirty,
        selection_edges: edges.as_ref(),
        magnifier: editor_state.magnifier.current.as_ref(),
        mag_label: picking,
        is_mag_monitor,
        toolbar,
        action_toolbar,
        settings_panel,
        font_panel,
        current_color: frame.current_color,
        outline_color: frame.outline_color,
        color_picker,
        model_popover: model_list,
        marker_popover,
        ruler_popover,
        number_popover,
        arrow_popover,
        line_popover,
        filter_popover,
        text_context_menu: editor_state
            .text_context_menu
            .as_mut()
            .filter(|menu| menu.monitor_idx == i && !menu.native),
        icons_cache: &editor_state.icons_cache,
        annotations_layer: &editor_state.annotations_layer[i],
        offset,
        // Nothing is ever drawn into the layer without also
        // landing in `annotations` or bumping the baked-pen
        // counter, so this is exactly "the layer is blank" -
        // and it skips a full-canvas composite on every whole
        // frame.
        annotations_layer_empty: hidden
            || (editor_state.annotations.is_empty() && editor_state.pending_pen_baked == 0),
        pending: editor_state
            .text_session
            .as_ref()
            .map(|session| session.presented())
            .or(editor_state.pending.as_ref())
            .filter(|_| !hidden),
        pending_pen_baked: editor_state.pending_pen_baked,
        is_pending_selected: editor_state.ann_drag.is_some()
            || editor_state.text_session.as_ref().is_some_and(|session| {
                session.original_id.is_some() && editor_state.selected_tool != Tool::Text
            }),
        dragged_index,
        selected_annotation: editor_state
            .selected_annotation
            .filter(|_| !hidden && editor_state.text_session.is_none()),
        line_point: editor_state.line_session.selected_point.filter(|_| !hidden),
        annotations: &editor_state.annotations,
        filters: &editor_state.filters.images,
        text: renderer::TextContext {
            font_system: &mut editor_state.text.font_system,
            swash_cache: &mut editor_state.text.swash_cache,
            editors: &mut editor_state.text.editors,
        },
        active_text_id: frame.active_text_id,
        text_caret_visible,
        laser: (!hidden && editor_state.selection.zone.is_some()).then_some(&editor_state.laser),
        ruler_labels: if hidden {
            &[]
        } else {
            &editor_state.ruler.labels
        },
        ruler_hover: (!hidden && editor_state.selected_tool == Tool::Ruler)
            .then_some(editor_state.ruler.hover.as_ref())
            .flatten(),
        drawing_bounds: editor_state.ruler.label_bounds,

        ocr_view: (editor_state.selected_tool == Tool::Ocr && editor_state.ocr.view.is_active())
            .then_some(&editor_state.ocr.view),
        ocr_scan: frame.scan_badge,
        monitor_idx: i,
        toasts: &editor_state.toasts,
        size: (
            editor_state.placements[i].size.0 as f32,
            editor_state.placements[i].size.1 as f32,
        ),
        scale,
    });

    damage
}

/// Everything the next frame starts from a clean slate.
pub(super) fn clear_frame_state(
    editor_state: &mut EditorState,
    dirty_mask: &mut u32,
    pacing: &Pacing,
) {
    editor_state.selection.prev_zone = editor_state.selection.zone;
    editor_state.prev_pending = editor_state.pending.clone();
    // set per pointer move it could point at a spot never drawn (several moves per frame)
    editor_state.magnifier.prev = editor_state.magnifier.current;
    editor_state.annotations_dirty = false;
    editor_state.damage_rects.clear();
    editor_state.layer_damage_rects.clear();
    let drawn = |monitor_idx: usize| !pacing.owes_to(monitor_idx);
    if drawn(editor_state.toolbar.monitor_idx) {
        editor_state.toolbar.dirty = false;
    }
    if drawn(editor_state.action_toolbar.monitor_idx) {
        editor_state.action_toolbar.dirty = false;
    }
    if drawn(editor_state.settings_panel.monitor_idx) {
        editor_state.settings_panel.dirty = false;
    }
    if drawn(editor_state.font_panel.monitor_idx) {
        editor_state.font_panel.dirty = false;
    }
    if drawn(editor_state.color_popover.monitor_idx) {
        editor_state.color_popover.dirty = false;
    }
    if drawn(editor_state.model_popover.monitor_idx) {
        editor_state.model_popover.dirty = false;
    }
    if drawn(editor_state.marker_popover.monitor_idx) {
        editor_state.marker_popover.dirty = false;
    }
    if drawn(editor_state.ruler_popover.monitor_idx) {
        editor_state.ruler_popover.dirty = false;
    }
    if drawn(editor_state.number_popover.monitor_idx) {
        editor_state.number_popover.dirty = false;
    }
    if drawn(editor_state.arrow_popover.monitor_idx) {
        editor_state.arrow_popover.dirty = false;
    }
    if drawn(editor_state.line_popover.monitor_idx) {
        editor_state.line_popover.dirty = false;
    }
    if drawn(editor_state.filter_popover.monitor_idx) {
        editor_state.filter_popover.dirty = false;
    }
    *dirty_mask = 0;
}

// ************************* //
//      RENDER HELPERS       //
// ************************* //
