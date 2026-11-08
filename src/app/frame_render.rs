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
    pub(super) source_residency: super::source_residency::Residency,
}

impl Pacing {
    pub(super) fn new(monitors: usize) -> Self {
        Self {
            owed: vec![None; monitors],
            dimmed_for: vec![None; monitors],
            source_residency: Default::default(),
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

/// 【截图绘制】【损坏提交】用editor_state和overlay绘制dirty_mask指定的输出，annotations_hidden控制正文，pacing控制提交节奏，返回协议结果
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
    super::selection_size::panels::prepare(editor_state);

    let mut rendered_any = false;
    for i in 0..editor_state.captures.len() {
        if is_dirty(dirty_mask, i) {
            rebuild_layer(editor_state, i, &frame);
            // 1. 【截图绘制】【首次完整场景】首帧尚未恢复背景，面板局部通知不能缩小其范围；等待原生可写时也保留完整损坏
            let area = if editor_state
                .navigation
                .frames
                .get(i)
                .is_none_or(Option::is_none)
            {
                Redraw::Whole
            } else {
                editor_state
                    .monitor_dirty_rect(i)
                    .map_or(Redraw::Whole, Redraw::Area)
            };
            pacing.owed[i] = Some(pacing.owed[i].map_or(area, |owed| owed.join(area)));
        }
        let Some(owed) = pacing.owed[i] else {
            continue;
        };
        // 2. 【截图绘制】【损坏保留】立即重建正文，合成与提交等待原生回调，清理损坏前保留尚未提交的范围
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
        super::frame_backdrop::stage(editor_state, overlay, i, damage)?;
        rendered_any = true;
    }
    overlay.flush()?;
    // 3. 【图像原件】【连续读取】文件原件沿用唯一映射，空闲回收由事件循环唤醒，避免缩放时逐帧重新读取长图
    if rendered_any
        && let Some(file) = &editor_state.file_canvas
        && file.original.cached_bytes() != 0
    {
        pacing.source_residency.touched(Instant::now());
    }
    for capture in &editor_state.captures {
        capture.pixmap.release_pages();
    }

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
    editor_state.prepare_region_layers();
    if (editor_state.region_rendering.enabled && !editor_state.region_rendering.cached_annotations)
        || editor_state.collection.drag.is_some()
    {
        return;
    }
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
    editor_state.prepare_region_layers();
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
    let view = editor_state.canvas_transform();
    let zone = editor_state.selection.zone.and_then(|rect| view.rect(rect));
    let (local_sel, prev_local, edges) =
        selection_render_info(&zone, dimmed_for, &editor_state.placements[i]);
    let selection_dirty = *dimmed_for != zone;
    *dimmed_for = zone;

    let offset = editor_state.placements[i].offset();
    let scale = editor_state.placements[i].scale;
    let radius = if editor_state.file_canvas.is_some() {
        crate::config::get().selection.border_radius
    } else {
        0.0
    };
    let key = crate::editor::canvas_navigation::state::FrameKey {
        view,
        selection: local_sel,
        offset,
        scale,
        radius,
        size: editor_state.placements[i].size,
    };
    editor_state
        .navigation
        .frames
        .resize(editor_state.placements.len(), None);
    let previous_key = editor_state.navigation.frames[i];
    let background_changed = previous_key != Some(key)
        && (!view.is_identity() || previous_key.is_some_and(|key| !key.view.is_identity()));
    editor_state.navigation.frames[i] = Some(key);
    let text_caret_visible = super::text_caret::visible(editor_state);

    let damage = dirty_rect.as_ref().and_then(|r| {
        renderer::rect_bounds(
            &renderer::scale_rect(r, scale)?,
            editor_state.canvas[i].width(),
            editor_state.canvas[i].height(),
        )
    });

    let dragged_index = editor_state.dragged_index();
    let hidden = frame.annotations_hidden;
    let selected_annotation = editor_state.selected_annotation_index().filter(|_| {
        !hidden && editor_state.text_session.is_none() && editor_state.selected_tool != Tool::Text
    });
    let collection_bounds = editor_state.collection_bounds();
    let drawing_space = crate::tools::drawing_space::DrawingSpace::from_editor(editor_state);
    let dirty = dirty_rect.as_ref();
    let toolbar = panel_to_draw(&mut editor_state.toolbar, i, dirty);
    let action_toolbar = panel_to_draw(&mut editor_state.action_toolbar, i, dirty);
    let settings_panel = panel_to_draw(&mut editor_state.settings_panel, i, dirty);
    let font_panel = panel_to_draw(&mut editor_state.font_panel, i, dirty);
    let external_panel = panel_to_draw(&mut editor_state.external_panel, i, dirty);
    let color_picker = panel_to_draw(&mut editor_state.color_popover, i, dirty);
    let model_list = panel_to_draw(&mut editor_state.model_popover, i, dirty);
    let marker_popover = panel_to_draw(&mut editor_state.marker_popover, i, dirty);
    let ruler_popover = panel_to_draw(&mut editor_state.ruler_popover, i, dirty);
    let number_popover = panel_to_draw(&mut editor_state.number_popover, i, dirty);
    let arrow_popover = panel_to_draw(&mut editor_state.arrow_popover, i, dirty);
    let line_popover = panel_to_draw(&mut editor_state.line_popover, i, dirty);
    let filter_popover = panel_to_draw(&mut editor_state.filter_popover, i, dirty);

    renderer::render_frame_with_navigation(
        &mut renderer::RenderRequest {
            canvas: &mut editor_state.canvas[i],
            capture: &editor_state.captures[i],
            frozen_image: renderer::FrozenImage::from_parts(
                editor_state.file_canvas.as_ref(),
                &editor_state.captures,
                &editor_state.placements,
            ),
            dimmed: &mut editor_state.dimmed[i],
            selection: local_sel.as_ref(),
            hovered_region: editor_state.region_detection.hovered.as_ref(),
            prev_selection: prev_local.as_ref(),
            dirty_rect: dirty_rect.as_ref(),
            selection_dirty,
            selection_edges: edges.as_ref(),
            selection_radius: if editor_state.file_canvas.is_some() {
                crate::config::get().selection.border_radius
            } else {
                0.0
            },
            magnifier: editor_state.magnifier.current.as_ref(),
            mag_label: picking,
            is_mag_monitor,
            toolbar,
            action_toolbar,
            settings_panel,
            font_panel,
            external_panel,
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
            selected_annotation,
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
            laser: (!hidden && editor_state.selection.zone.is_some())
                .then_some(&editor_state.laser),
            ruler_labels: if hidden {
                &[]
            } else {
                &editor_state.ruler.labels
            },
            ruler_hover: (!hidden && editor_state.selected_tool == Tool::Ruler)
                .then_some(editor_state.ruler.hover.as_ref())
                .flatten(),
            drawing_bounds: editor_state.ruler.label_bounds,

            ocr_view: (editor_state.selected_tool == Tool::Ocr
                && editor_state.ocr.view.is_active())
            .then_some(&editor_state.ocr.view),
            ocr_scan: frame.scan_badge,
            monitor_idx: i,
            toasts: &editor_state.toasts,
            size: (
                editor_state.placements[i].size.0 as f32,
                editor_state.placements[i].size.1 as f32,
            ),
            scale,
        },
        super::selection_size::panels::overlay(&editor_state.selection_size, i),
        editor_state
            .annotation_selection
            .cache
            .as_ref()
            .filter(|_| {
                editor_state
                    .annotation_selection
                    .frame
                    .is_some_and(|(monitor, _)| monitor == i && !hidden)
            }),
        drawing_space.filter(|_| !hidden).map(|space| {
            super::annotation_collection::chrome::Overlay {
                ids: editor_state.collection.selection.ids(),
                bounds: collection_bounds,
                marquee: editor_state
                    .collection
                    .marquee
                    .as_ref()
                    .and_then(|marquee| marquee.bounds),
                dragging: editor_state.collection.drag.is_some(),
                canvas: space.bounds,
            }
        }),
        view,
        background_changed,
        editor_state
            .text_session
            .as_ref()
            .and_then(|session| session.original_id),
        editor_state
            .region_rendering
            .enabled
            .then_some(&editor_state.region_rendering),
    );
    crate::ui::canvas_navigation::draw::draw(editor_state, i, dirty_rect);
    crate::ui::canvas_navigation::zoom::draw(editor_state, i, dirty_rect);
    super::selection_translation::panels::draw(editor_state, i, dirty_rect);
    super::selection_size::panels::draw_popup(editor_state, i, dirty_rect);
    crate::features::color_sample::inspector_draw::draw(editor_state, i, dirty_rect);

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
