mod capture_composite;
mod clipped;
pub use capture_composite::{capture_scale, composite};

mod dimming;
pub(crate) mod editor_text;
pub mod frozen_image;
pub use dimming::init_dimming;
use dimming::{blit_rect, draw_selection_border, update_dimming_delta};

// Frame building and shared drawing stuff: paths, text, and annotations.
// UI component rendering lives in ui/<component>/draw.rs
// OCR overlay rendering lives in ocr/draw.rs alongside its state

pub mod annotation_stack;
mod annotations;
mod filter_handles;
pub mod fixed_path;
pub(crate) mod line_edit;
mod line_handles;
mod magnifier_handles;
mod number_handles;
pub(crate) mod number_text;
mod painter;
pub mod paths;
pub mod ruler_overlay;
pub(crate) mod ruler_text;
pub mod stroke_path;
pub mod text;
mod text_preedit;

pub use annotations::{
    draw_annotation, selection_chrome_pad, shadow_color_for, stroke_pen_segment, visual_pad,
};
pub use frozen_image::FrozenImage;
pub use painter::{Icons, Painter};
pub use paths::{rect_bounds, rounded_rect_path, scale_rect};
pub use text::measure_line_width;

use crate::types::SelectionEdges;
use crate::types::annotations::Annotation;
use crate::ui::color_popover::ColorPickerPopover;
use crate::ui::magnifier::MagnifierState;
use crate::ui::settings_panel::SettingsPanel;
use crate::ui::toolbar::Toolbar;
use cosmic_text::{Editor, FontSystem, SwashCache};
use std::collections::HashMap;
use tiny_skia::{Color, PathBuilder, Pixmap, Rect, Stroke, Transform};
use usvg::Tree;

pub struct TextContext<'a> {
    pub font_system: &'a mut FontSystem,
    pub swash_cache: &'a mut SwashCache,
    pub editors: &'a mut HashMap<u64, Editor<'static>>,
}

pub struct RenderRequest<'a> {
    // basic layers
    pub canvas: &'a mut Pixmap,
    pub capture: &'a crate::types::Capture,
    pub frozen_image: FrozenImage<'a>,
    pub dimmed: &'a mut Pixmap,
    // selection + magnifier + toolbar
    pub selection: Option<&'a Rect>,
    pub prev_selection: Option<&'a Rect>,
    pub dirty_rect: Option<&'a Rect>,
    pub selection_edges: Option<&'a SelectionEdges>,
    pub selection_dirty: bool,
    pub magnifier: Option<&'a MagnifierState>,
    pub is_mag_monitor: bool,
    /// Eyedropper only: the loupe also carries the colour plate under it.
    pub mag_label: bool,
    pub toolbar: Option<&'a mut Toolbar>,
    pub action_toolbar: Option<&'a mut Toolbar>,
    pub settings_panel: Option<&'a mut SettingsPanel>,
    pub font_panel: Option<&'a mut crate::ui::font_panel::FontPanel>,
    pub current_color: Color,
    pub outline_color: Color,
    pub color_picker: Option<&'a mut ColorPickerPopover>,
    pub model_popover: Option<&'a mut crate::ui::model_popover::ModelPopover>,
    /// 【形状标记】【选择绘制】与其他属性弹层共用帧提交和内容缓存
    pub marker_popover: Option<&'a mut crate::ui::choice_popover::ChoicePopover>,
    pub ruler_popover: Option<&'a mut crate::ui::choice_popover::ChoicePopover>,
    pub number_popover: Option<&'a mut crate::ui::choice_popover::ChoicePopover>,
    pub arrow_popover: Option<&'a mut crate::ui::choice_popover::ChoicePopover>,
    pub line_popover: Option<&'a mut crate::ui::choice_popover::ChoicePopover>,
    pub filter_popover: Option<&'a mut crate::ui::choice_popover::ChoicePopover>,
    pub text_context_menu: Option<&'a mut crate::ui::context_menu::ContextMenu>,
    pub icons_cache: &'a HashMap<&'static str, Tree>,
    pub offset: (f32, f32),
    // annotations
    pub annotations_layer: &'a Pixmap,
    pub annotations_layer_empty: bool,
    pub pending: Option<&'a Annotation>,
    pub pending_pen_baked: usize,
    pub is_pending_selected: bool,
    /// index of dragged annotation in `annotations`
    pub dragged_index: Option<usize>,
    pub selected_annotation: Option<usize>,
    pub line_point: Option<(u64, usize)>,
    pub annotations: &'a [Annotation],
    pub filters: &'a HashMap<u64, crate::features::filter::raster::Raster>,
    /// Everything needed to lay out and draw text.
    pub text: TextContext<'a>,
    pub active_text_id: Option<u64>,
    pub text_caret_visible: bool,
    /// 【激光】【临时绘制】独立笔迹只绘入屏幕动态层
    pub laser: Option<&'a crate::features::laser_trace::LaserTrace>,
    pub ruler_labels: &'a [crate::features::ruler::labels::InfoLabel],
    pub ruler_hover: Option<&'a crate::features::ruler::state::RulerHover>,
    pub drawing_bounds: Option<Rect>,
    // OCR tool: recognized lines + selection overlay
    pub ocr_view: Option<&'a crate::ocr::OcrView>,
    /// OCR tool: region being scanned (global) + spinner animation, while a
    /// recognition is working
    pub ocr_scan: Option<(Rect, f32)>,
    pub monitor_idx: usize,
    pub toasts: &'a crate::ui::toast::Toasts,
    /// logical size of the monitor; canvas / scale is off by a fraction,
    /// and the loupe would flip at another spot than its damage does
    pub size: (f32, f32),
    pub scale: f32,
}

pub fn render_frame(req: &mut RenderRequest) {
    let s = req.scale;
    if req.selection_dirty {
        update_dimming_delta(
            req.dimmed,
            req.prev_selection,
            req.selection,
            req.selection_edges,
            s,
        );
    }
    let dirty_rect = req.dirty_rect;

    if let Some(dirty) = dirty_rect {
        blit_rect(req.dimmed, req.canvas, dirty, s);
    } else {
        req.canvas.data_mut().copy_from_slice(req.dimmed.data());
    }

    annotation_stack::draw_layers(req);

    ruler_overlay::draw_clipped(
        req.canvas,
        req.ruler_labels,
        req.ruler_hover,
        req.offset,
        s,
        dirty_rect,
        req.text.font_system,
        req.text.swash_cache,
    );

    if let Some(laser) = req.laser {
        crate::features::laser_trace::draw(
            req.canvas,
            laser,
            req.offset,
            dirty_rect,
            s,
            std::time::Instant::now(),
        );
    }

    if let Some(sel) = req.selection {
        Painter::new(req.canvas)
            .with_scale(s)
            .clipped(dirty_rect.copied(), |painter| {
                draw_selection_border(painter, sel, req.selection_edges);
            });
    }

    if let Some(pending) = req.pending
        && req.is_pending_selected
    {
        Painter::new(req.canvas)
            .with_scale(s)
            .with_origin(req.offset)
            .clipped(dirty_rect.copied(), |painter| {
                annotations::draw_annotation_handles_only(painter, pending);
                if let Some((id, index)) = req.line_point
                    && pending.id == id
                {
                    line_handles::highlight(painter, pending, index);
                }
            });
    }

    // Dynamic selection chrome: handles for the currently selected annotation
    if let Some(idx) = req.selected_annotation
        && let Some(ann) = req.annotations.get(idx)
    {
        Painter::new(req.canvas)
            .with_scale(s)
            .with_origin(req.offset)
            .clipped(dirty_rect.copied(), |painter| {
                annotations::draw_annotation_handles_only(painter, ann);
                if let Some((id, index)) = req.line_point
                    && ann.id == id
                {
                    line_handles::highlight(painter, ann, index);
                }
            });
    }

    // Clipped to the dirty rect: everything the overlay paints is translucent,
    // so anything drawn outside the area just restored from `dimmed` would
    // stack a second layer on top of last frame's.
    if let Some(view) = req.ocr_view {
        crate::ocr::draw::draw_ocr_overlay(req.canvas, view, req.offset, dirty_rect, s);
    }

    if let Some((region, phase)) = req.ocr_scan {
        crate::ocr::draw::draw_ocr_scan(
            req.canvas,
            region,
            phase,
            req.offset,
            req.icons_cache,
            dirty_rect,
            s,
        );
    }

    let mut p = Painter::new(req.canvas)
        .with_scale(s)
        .with_icons(req.icons_cache)
        .with_text(req.text.font_system, req.text.swash_cache);

    if req.is_mag_monitor
        && let Some(mag) = req.magnifier
    {
        crate::ui::magnifier::draw_magnifier(
            &mut p,
            req.capture,
            (mag.pos.0 as f32, mag.pos.1 as f32),
            req.size,
            req.mag_label,
        );
    }

    if let Some(tb) = req.toolbar.as_deref_mut()
        && tb.dirty
    {
        crate::ui::toolbar::draw_toolbar(&mut p, tb);
    }
    if let Some(tb) = req.action_toolbar.as_deref_mut()
        && tb.dirty
    {
        crate::ui::toolbar::draw_toolbar(&mut p, tb);
    }

    if let Some(settings) = req.settings_panel.as_deref_mut()
        && settings.dirty
    {
        crate::ui::settings_panel::draw_settings_panel(
            &mut p,
            settings,
            req.current_color,
            req.outline_color,
        );
    }
    if let Some(color_picker) = req.color_picker.as_deref_mut()
        && color_picker.dirty
    {
        crate::ui::color_popover::draw_color_popover(&mut p, color_picker);
    }
    if let Some(panel) = req.font_panel.as_deref_mut()
        && panel.dirty
    {
        crate::ui::font_panel::draw::draw(&mut p, panel);
    }
    if let Some(model_popover) = req.model_popover.as_deref_mut()
        && model_popover.dirty
    {
        crate::ui::model_popover::draw_model_popover(&mut p, model_popover);
    }
    if let Some(marker_popover) = req.marker_popover.as_deref_mut()
        && marker_popover.dirty
    {
        crate::ui::choice_popover::draw_choice_popover(
            &mut p,
            marker_popover,
            |painter, index, rect, color| {
                if let Some(&shape) = crate::types::MarkerShape::ALL.get(index) {
                    crate::features::marker::draw_icon(painter, shape, rect, color, true);
                }
            },
        );
    }

    if let Some(ruler_popover) = req.ruler_popover.as_deref_mut()
        && ruler_popover.dirty
    {
        crate::ui::choice_popover::draw_choice_popover(
            &mut p,
            ruler_popover,
            |painter, index, rect, color| {
                let svg = if index == 0 {
                    crate::ui::icons::RULER
                } else {
                    crate::ui::icons::RULER_ANGLE
                };
                let size = rect.width().min(rect.height()) * 0.65;
                let color = color.to_color_u8();
                painter.icon(
                    svg,
                    size,
                    rect.left() + (rect.width() - size) / 2.0,
                    rect.top() + (rect.height() - size) / 2.0,
                    crate::theme::Rgba(color.red(), color.green(), color.blue(), color.alpha()),
                );
            },
        );
    }

    if let Some(number_popover) = req.number_popover.as_deref_mut()
        && number_popover.dirty
    {
        crate::ui::choice_popover::draw_choice_popover(
            &mut p,
            number_popover,
            |painter, index, rect, color| {
                if let Some(style) = crate::features::number::NumberStyle::ALL.get(index) {
                    number_text::draw_preview(painter, rect, *style, color);
                }
            },
        );
    }

    for (panel, kind) in [
        (
            req.arrow_popover.as_deref_mut(),
            crate::ui::line_properties::LineChoice::Heads,
        ),
        (
            req.line_popover.as_deref_mut(),
            crate::ui::line_properties::LineChoice::Pattern,
        ),
    ] {
        if let Some(panel) = panel
            && panel.dirty
        {
            crate::ui::line_properties::draw_popover(&mut p, panel, kind);
        }
    }

    if let Some(panel) = req.filter_popover.as_deref_mut()
        && panel.dirty
    {
        crate::ui::filter_properties::draw_popover(&mut p, panel);
    }

    if let Some(menu) = req.text_context_menu.as_deref_mut() {
        crate::ui::context_menu::draw::draw(&mut p, menu, dirty_rect);
    }
    if !req.toasts.items.is_empty() {
        crate::ui::toast::draw_toasts(&mut p, req.toasts, req.monitor_idx, dirty_rect);
    }
}
// ***************************/
/// SELECTION + DIMMING  ////
// **************************/
//
// The dim layer is `base` layer but darkened everywhere except inside the selection
//
// The bright rectangle's corners are rounded to the *inner* edge of the
// selection border, so the border doesn't have a hard-edged hole.
// rounded corners are only visual, the screenshot result won't have such corners
//
pub fn paste(canvas: &mut Pixmap, pixmap: &Pixmap, x: i32, y: i32) {
    let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
    let (x0, x1) = (x.max(0), (x + pixmap.width() as i32).min(cw));
    if x1 <= x0 {
        return;
    }
    let len = (x1 - x0) as usize * 4;
    let src_len = pixmap.width() as usize * 4;
    let skip = (x0 - x) as usize * 4;
    for row in y.max(0)..(y + pixmap.height() as i32).min(ch) {
        let src = &pixmap.data()[(row - y) as usize * src_len + skip..][..len];
        let at = (row * cw + x0) as usize * 4;
        canvas.data_mut()[at..at + len].copy_from_slice(src);
    }
}

/// Clears a rectangle to transparent in the persistent layer without anti-aliasing
/// by writing zeroes directly to the slice.
fn clear_rect_transparent(layer: &mut Pixmap, rect: &Rect) {
    let (w, h) = (layer.width(), layer.height());
    let Some((x, y, rw, rh)) = rect_bounds(rect, w, h) else {
        return;
    };
    let stride = (layer.width() * 4) as usize;
    let row_bytes = rw as usize * 4;
    let data = layer.data_mut();

    for row in 0..rh {
        let off = (y + row) as usize * stride + x as usize * 4;
        data[off..off + row_bytes].fill(0);
    }
}

/// Rebuilds the annotations layer.
///
/// If `dirty_rect` is provided (in local layer coordinates), clears and
/// redraws ONLY annotations whose visual bbox intersects with this area.
///
/// `None` indicates a full rebuild (first frame, resize, safety fallback).
/// 【文字事务】【持久图层】将 annotations 绘入 p，text_editors 和 filters 提供缓存，active_text_id 排除内联编辑的原始标注，dirty_rect 限制更新区域，无返回值
pub fn rebuild_annotations_layer(
    p: &mut Painter,
    annotations: &[Annotation],
    text_editors: &mut HashMap<u64, Editor<'static>>,
    filters: &HashMap<u64, crate::features::filter::raster::Raster>,
    active_text_id: Option<u64>,
    dirty_rect: Option<Rect>,
) {
    let offset = p.origin;
    let s = p.scale();
    let (lw, lh) = p.size();

    let Some(base_rect) = dirty_rect.or_else(|| Rect::from_xywh(0.0, 0.0, lw, lh)) else {
        return;
    };
    let Some(px_rect) = scale_rect(&base_rect, s) else {
        return;
    };

    if dirty_rect.is_none() {
        clear_rect_transparent(p.canvas, &px_rect);
        for ann in annotations {
            if active_text_id == Some(ann.id) {
                continue;
            }
            draw_annotation(p, ann, false, text_editors, filters, active_text_id);
        }
        return;
    }

    clear_rect_transparent(p.canvas, &px_rect);

    // Determine the integer bounds of the dirty rect within the layer.
    let lw = p.canvas.width();
    let lh = p.canvas.height();
    let x0 = (px_rect.left().floor() as i32).clamp(0, lw as i32) as u32;
    let y0 = (px_rect.top().floor() as i32).clamp(0, lh as i32) as u32;
    let x1 = (px_rect.right().ceil() as i32).clamp(0, lw as i32) as u32;
    let y1 = (px_rect.bottom().ceil() as i32).clamp(0, lh as i32) as u32;
    let tw = x1.saturating_sub(x0);
    let th = y1.saturating_sub(y0);

    // Allocate a clean temporary pixmap the size of the dirty rect.
    // Annotations are drawn into it with an offset that maps global
    // coordinates into temp-pixmap-local space, keeping shadow blending
    // identical to a full-rebuild (always transparent background).
    let Some(mut tmp) = Pixmap::new(tw, th) else {
        return;
    };
    let tmp_offset = (offset.0 + x0 as f32 / s, offset.1 + y0 as f32 / s);
    let mut tmp_painter = p.sub(&mut tmp).with_origin(tmp_offset);

    for ann in annotations {
        if active_text_id == Some(ann.id) {
            continue;
        }
        let pad = ann.visual_pad();
        let l = ann.bbox.left() - offset.0 - pad;
        let t = ann.bbox.top() - offset.1 - pad;
        let r = ann.bbox.right() - offset.0 + pad;
        let b = ann.bbox.bottom() - offset.1 + pad;

        if l < base_rect.right()
            && r > base_rect.left()
            && t < base_rect.bottom()
            && b > base_rect.top()
        {
            draw_annotation(
                &mut tmp_painter,
                ann,
                false,
                text_editors,
                filters,
                active_text_id,
            );
        }
    }

    // Composite the temp pixmap back into the real layer at the correct position.
    paste(p.canvas, &tmp, x0 as i32, y0 as i32);
}
