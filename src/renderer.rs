mod capture_composite;
mod clipped;
pub(crate) use capture_composite::composite_precise;
pub use capture_composite::{capture_scale, composite};
pub(crate) mod pixel_bounds;

mod annotation_tiles;
mod background_restore;
mod canvas_navigation;
mod dimming;
pub(crate) mod editor_text;
mod frame_scene;
pub mod frozen_image;
pub(crate) mod opaque_backdrop;
pub use dimming::init_dimming;
use dimming::{blit_rect, draw_selection_border, update_dimming_delta};

// Frame building and shared drawing stuff: paths, text, and annotations.
// UI component rendering lives in ui/<component>/draw.rs
// OCR overlay rendering lives in ocr/draw.rs alongside its state

mod annotation_rotation;
pub mod annotation_stack;
mod annotation_style;
mod annotations;
mod collection_stack;
mod filter_handles;
pub mod fixed_path;
mod glyph_run;
pub(crate) mod line_edit;
mod line_handles;
mod magnifier_handles;
mod number_handles;
pub(crate) mod number_text;
mod painter;
pub mod paths;
mod rotated_text;
mod rotation_tiles;
pub(crate) mod ruler_chrome;
mod ruler_hover_label;
mod ruler_hover_tiles;
mod ruler_labels;
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
    /// 【截图区域识别】【独立候选】全局逻辑矩形，不参与正式选区、工具栏与导出
    pub hovered_region: Option<&'a Rect>,
    pub prev_selection: Option<&'a Rect>,
    pub dirty_rect: Option<&'a Rect>,
    pub selection_edges: Option<&'a SelectionEdges>,
    /// 【截图选框】【绘制半径】截图范围使用零，图片预览使用其独立配置，不影响标注组件选框
    pub selection_radius: f32,
    pub selection_dirty: bool,
    pub magnifier: Option<&'a MagnifierState>,
    pub is_mag_monitor: bool,
    /// Eyedropper only: the loupe also carries the colour plate under it.
    pub mag_label: bool,
    pub toolbar: Option<&'a mut Toolbar>,
    pub action_toolbar: Option<&'a mut Toolbar>,
    pub settings_panel: Option<&'a mut SettingsPanel>,
    pub font_panel: Option<&'a mut crate::ui::font_panel::FontPanel>,
    pub external_panel: Option<&'a mut crate::ui::external_panel::ExternalPanel>,
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
    render_frame_with_selection_size(req, None, None);
}

/// 【截图绘制】【尺寸叠层】按 req 和 selection_size 合成屏幕，尺寸面板按原版位于工具栏上方或下方，无返回值
pub(crate) fn render_frame_with_selection_size(
    req: &mut RenderRequest,
    selection_size: Option<crate::ui::selection_size::cache::Overlay<'_>>,
    annotation_delete: Option<&crate::ui::annotation_selection::delete_cache::Cache>,
) {
    render_frame_with_collection(req, selection_size, annotation_delete, None);
}

/// 【标注多选】【场景合成】按req、尺寸叠层、删除按钮与collection合成动态场景，共同拖动绕过缓存正文，无返回值
pub(crate) fn render_frame_with_collection(
    req: &mut RenderRequest,
    selection_size: Option<crate::ui::selection_size::cache::Overlay<'_>>,
    annotation_delete: Option<&crate::ui::annotation_selection::delete_cache::Cache>,
    collection: Option<crate::app::annotation_collection::chrome::Overlay<'_>>,
) {
    render_frame_with_navigation(
        req,
        selection_size,
        annotation_delete,
        collection,
        Default::default(),
        false,
        None,
        None,
    );
}

/// 【画布导航】【帧合成】按req与view绘制画布，叠层参数提供控件；background_changed重采样背景，original_text_id排除编辑原件，无返回值
pub(crate) fn render_frame_with_navigation(
    req: &mut RenderRequest,
    selection_size: Option<crate::ui::selection_size::cache::Overlay<'_>>,
    annotation_delete: Option<&crate::ui::annotation_selection::delete_cache::Cache>,
    collection: Option<crate::app::annotation_collection::chrome::Overlay<'_>>,
    view: crate::editor::canvas_navigation::transform::ViewTransform,
    background_changed: bool,
    original_text_id: Option<u64>,
    region: Option<&crate::editor::region_rendering::State>,
) {
    let s = req.scale;
    let dirty_rect = req.dirty_rect;
    if view.is_identity() {
        if region.is_none() && background_changed {
            init_dimming(req.dimmed, req.selection, req.selection_edges, s);
            if let Some(selection) = req.selection {
                dimming::dim_hole_corners(
                    req.dimmed,
                    selection,
                    req.selection_edges,
                    s,
                    req.selection_radius,
                );
            }
            req.selection_dirty = false;
        }
        frame_scene::draw(
            req,
            annotation_delete,
            collection.as_ref(),
            original_text_id,
            region,
        );
    } else {
        canvas_navigation::draw(
            req,
            view,
            background_changed,
            original_text_id,
            annotation_delete,
            collection.as_ref(),
            region,
        );
    }

    let mut p = Painter::new(req.canvas)
        .with_scale(s)
        .with_icons(req.icons_cache)
        .with_text(req.text.font_system, req.text.swash_cache);

    if let Some(overlay) = selection_size
        .as_ref()
        .filter(|overlay| overlay.under_toolbars)
    {
        crate::ui::selection_size::cache::composite(&mut p, overlay, dirty_rect.copied());
    }

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
    if let Some(overlay) = selection_size
        .as_ref()
        .filter(|overlay| !overlay.under_toolbars)
    {
        crate::ui::selection_size::cache::composite(&mut p, overlay, dirty_rect.copied());
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
    if let Some(panel) = req.external_panel.as_deref_mut() {
        p.clipped_scene(dirty_rect.copied(), |p| {
            crate::ui::external_panel::draw::draw(p, panel)
        });
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
        crate::utils::transparent_pixmap::clear(p.canvas);
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
        let bounds = annotation_style::damage(
            ann,
            tmp_painter.window_units(),
            tmp_painter.drawing_bounds(),
            tmp_painter.ruler_projection(),
        );
        let l = bounds.left() - offset.0;
        let t = bounds.top() - offset.1;
        let r = bounds.right() - offset.0;
        let b = bounds.bottom() - offset.1;

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
