// Scrolling capture preview panel: thumbnail, status text, and controls.
//
// Layout is vertically divided into thumbnail, status text, length counter, and buttons.

use tiny_skia::{Pixmap, Rect};

use crate::renderer::Painter;
use crate::renderer::text::TextStyle;
use crate::scroll::session::ScrollSession;
use crate::theme::{color, font, radius, size};
use crate::ui::icons;
use shot_stitch::Axis;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelButton {
    TogglePause,
    ToggleAxis,
    Finish,
    Cancel,
}

pub struct PanelLayout {
    pub rect: Rect,
    pub preview: Option<Rect>,
    pub status: Rect,
    pub length: Rect,
    pub buttons: Vec<(PanelButton, Rect)>,
}

const PANEL_WIDTH: f32 = 168.0;
const BUTTON_SIZE: f32 = 26.0;
const BUTTON_GAP: f32 = 6.0;
const LINE_HEIGHT: f32 = 16.0;
const PREVIEW_MAX_HEIGHT: f32 = 220.0;
const SHEET: &[PanelButton] = &[
    PanelButton::TogglePause,
    PanelButton::ToggleAxis,
    PanelButton::Finish,
    PanelButton::Cancel,
];

/// Computes panel layout and geometry for sub-elements in monitor local coordinates.
pub fn layout(
    local_region: Rect,
    monitor_size: (f32, f32),
    preview_size: Option<(u32, u32)>,
) -> PanelLayout {
    let padding = size::padding();
    let gap = size::margin();

    let preview_h = preview_size
        .map(|(_, h)| (h as f32).min(PREVIEW_MAX_HEIGHT))
        .unwrap_or(0.0);
    let preview_gap = if preview_h > 0.0 { gap } else { 0.0 };
    let height = padding * 2.0 + preview_h + preview_gap + LINE_HEIGHT * 2.0 + gap + BUTTON_SIZE;

    // prefer placing panel on the right side of the selection; flip to left if space is tight
    let (mon_w, mon_h) = monitor_size;
    let right_room = mon_w - local_region.right() - gap * 2.0;
    let preferred_x = if right_room >= PANEL_WIDTH {
        local_region.right() + gap
    } else {
        local_region.left() - gap - PANEL_WIDTH
    };
    let x_limit = (mon_w - PANEL_WIDTH - gap).max(gap);
    let x = preferred_x.clamp(gap, x_limit);
    let y_limit = (mon_h - height - gap).max(gap);
    let y = local_region.top().clamp(gap, y_limit);

    let Some(rect) = Rect::from_xywh(x, y, PANEL_WIDTH, height) else {
        return PanelLayout {
            rect: Rect::from_xywh(0.0, 0.0, PANEL_WIDTH, height).unwrap(),
            preview: None,
            status: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
            length: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
            buttons: Vec::new(),
        };
    };

    let inner_w = rect.width() - padding * 2.0;
    let mut cursor_y = rect.top() + padding;

    // thumbnail sits at the top of the panel
    let preview = (preview_h > 0.0)
        .then(|| Rect::from_xywh(rect.left() + padding, cursor_y, inner_w, preview_h))
        .flatten();
    if preview.is_some() {
        cursor_y += preview_h + preview_gap;
    }

    // separate lines for status and length so labels don't collide
    let status = Rect::from_xywh(rect.left() + padding, cursor_y, inner_w, LINE_HEIGHT);
    cursor_y += LINE_HEIGHT;
    let length = Rect::from_xywh(rect.left() + padding, cursor_y, inner_w, LINE_HEIGHT);
    cursor_y += LINE_HEIGHT + gap;

    // horizontal button row
    let mut buttons = Vec::with_capacity(SHEET.len());
    let mut bx = rect.left() + padding;
    for button in SHEET {
        if let Some(btn_rect) = Rect::from_xywh(bx, cursor_y, BUTTON_SIZE, BUTTON_SIZE) {
            buttons.push((*button, btn_rect));
        }
        bx += BUTTON_SIZE + BUTTON_GAP;
    }

    PanelLayout {
        rect,
        preview,
        status: status.unwrap_or(rect),
        length: length.unwrap_or(rect),
        buttons,
    }
}

/// Checks if pointer falls on a button.
pub fn hit_test(layout: &PanelLayout, local: (f64, f64)) -> Option<PanelButton> {
    let (px, py) = (local.0 as f32, local.1 as f32);
    for (button, rect) in &layout.buttons {
        if px >= rect.left() && px <= rect.right() && py >= rect.top() && py <= rect.bottom() {
            return Some(*button);
        }
    }
    None
}

/// Renders the floating control panel onto the monitor canvas.
pub fn draw(
    p: &mut Painter,
    session: &mut ScrollSession,
    layout: &PanelLayout,
    hovered: Option<PanelButton>,
) {
    let cfg = &crate::config::get().toolbar;

    p.fill_rounded(layout.rect, radius::panel(), color::panel());

    // thumbnail clipped to bottom so latest stitched frames remain visible
    if let Some(rect) = layout.preview {
        draw_preview(p, session, rect);
    }

    // draw status and length lines
    let small = font::small();
    let status = fit_text(p, &session.status, layout.status.width(), small);
    p.text(
        layout.status,
        &status,
        TextStyle::new(small, color::muted()),
    );
    let mut length = session.length_label();
    if session.last_append() > 0 {
        length = format!("{length}  +{}", session.last_append());
    }
    if !length.is_empty() {
        let length = fit_text(p, &length, layout.length.width(), small);
        p.text(
            layout.length,
            &length,
            TextStyle::new(small, color::foreground()),
        );
    }

    // buttons with hover highlight
    for (button, rect) in &layout.buttons {
        let enabled = is_button_enabled(*button, session);
        if enabled && hovered == Some(*button) {
            p.fill_rounded(*rect, radius::item(), color::hover());
        }

        let svg = button_icon(*button, session);
        let tint = if enabled {
            cfg.icon.get()
        } else {
            color::muted().with_alpha(90)
        };
        let icon_size = (rect.width() * 0.62).round();
        let x = rect.left() + (rect.width() - icon_size) / 2.0;
        let y = rect.top() + (rect.height() - icon_size) / 2.0;
        p.icon(svg, icon_size, x, y, tint);
    }

    p.panel_border(
        layout.rect.left(),
        layout.rect.top(),
        layout.rect.width(),
        layout.rect.height(),
        radius::panel(),
        1.0,
    );
}

/// Truncates text with an ellipsis if it exceeds the available width.
fn fit_text(p: &mut Painter, text: &str, width: f32, font_size: f32) -> String {
    if text.is_empty() || width <= 0.0 {
        return String::new();
    }
    if p.measure(text, font_size) <= width {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_w = p.measure(ellipsis, font_size);
    let mut out = String::new();
    let mut used = 0.0;
    for ch in text.chars() {
        let probe = ch.to_string();
        let w = p.measure(&probe, font_size);
        if used + w + ellipsis_w > width {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push_str(ellipsis);
    out
}

/// Draws thumbnail preview aligned to the bottom of rect.
fn draw_preview(p: &mut Painter, session: &mut ScrollSession, rect: Rect) {
    let Some(preview) = session.preview() else {
        return;
    };
    let pw = preview.width() as f32;
    let ph = preview.height() as f32;
    if pw <= 0.0 || ph <= 0.0 {
        return;
    }

    let s = p.scale();
    let (left, top) = p.to_px(rect.left(), rect.top());
    let Some(rect) = Rect::from_xywh(left, top, rect.width() * s, rect.height() * s) else {
        return;
    };
    let scale = rect.width() / pw;
    let drawn_h = ph * scale;

    let clip_h = drawn_h.min(rect.height()).round().max(1.0) as u32;
    let target_w = rect.width().round().max(1.0) as u32;
    let Some(mut scaled) = Pixmap::new(target_w, clip_h) else {
        return;
    };

    let dst = scaled.pixels_mut();
    let source_top = (preview.height() as f32 - clip_h as f32 / scale).max(0.0);
    for y in 0..clip_h {
        let sy = (source_top + y as f32 / scale) as i32;
        let sy = sy.clamp(0, preview.height() as i32 - 1);
        for x in 0..target_w {
            let sx = ((x as f32 / scale) as i32).clamp(0, preview.width() as i32 - 1);
            let pixel = preview.pixel(sx as u32, sy as u32);
            if let Some(pixel) = pixel {
                dst[(y * target_w + x) as usize] = pixel;
            }
        }
    }

    p.draw_pixmap(rect.left() as i32, rect.top() as i32, &scaled);
}

/// Returns whether a panel button is currently interactable.
pub fn is_button_enabled(button: PanelButton, session: &ScrollSession) -> bool {
    match button {
        PanelButton::ToggleAxis => session.can_toggle_axis(),
        PanelButton::TogglePause | PanelButton::Finish | PanelButton::Cancel => true,
    }
}

/// Returns the icon SVG string corresponding to a panel button and session state.
pub fn button_icon(button: PanelButton, session: &ScrollSession) -> &'static str {
    match button {
        PanelButton::TogglePause if session.paused => icons::SCROLL_RESUME,
        PanelButton::TogglePause => icons::SCROLL_PAUSE,
        PanelButton::ToggleAxis => match session.axis() {
            Axis::Vertical => icons::SCROLL,
            Axis::Horizontal => icons::SCROLL_HORIZONTAL,
        },
        PanelButton::Finish => icons::CHECK,
        PanelButton::Cancel => icons::CLOSE,
    }
}
