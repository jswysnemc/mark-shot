use crate::renderer::Painter;
use crate::renderer::text::TextStyle;
use crate::theme::{color, font, radius, size, stroke};
use crate::ui::color_popover::ColorTarget;
use crate::ui::panel::UiPanel;
use crate::ui::settings_panel::{SettingsPanel, SettingsWidget, StepperArrow, ToggleVisual};
use cosmic_text::FontSystem;
use tiny_skia::{Color, PathBuilder, Rect, Stroke};

pub fn draw_settings_panel(
    p: &mut Painter,
    panel: &mut SettingsPanel,
    current_color: Color,
    outline_color: Color,
) {
    if !panel.visible {
        return;
    }

    let Some(panel_rect) = panel.rect() else {
        return;
    };

    let mut cache = panel.panel_pixmap.take();
    if panel.text_layout.is_some() {
        p.panel_unframed(
            &mut cache,
            (panel_rect.left(), panel_rect.top()),
            panel.size,
            panel.opacity,
            true,
            |p| draw_settings_content(p, panel, current_color, outline_color),
        );
    } else {
        p.panel(
            &mut cache,
            (panel_rect.left(), panel_rect.top()),
            panel.size,
            panel.opacity,
            true,
            |p| draw_settings_content(p, panel, current_color, outline_color),
        );
    }
    panel.panel_pixmap = cache;
}

fn draw_settings_content(
    p: &mut Painter,
    panel: &SettingsPanel,
    current_color: Color,
    outline_color: Color,
) {
    if panel.text_layout.is_some() {
        crate::ui::text_properties::draw::draw(p, panel, current_color);
        return;
    }
    let cfg = &crate::config::get().settings_panel;
    let theme = crate::theme::color::theme();
    let (w, h) = panel.size;
    let Some(rect) = Rect::from_xywh(0.0, 0.0, w, h) else {
        return;
    };

    p.fill_rounded(rect, radius::panel(), cfg.background.get());

    let text_color = cfg.text.get().color();

    for (index, item) in panel.widgets.iter().enumerate() {
        let Some(cell) = panel.cell(index) else {
            continue;
        };
        let current_x = cell.left();
        let item_y = cell.top();
        let item_w = cell.width();
        let item_h = cell.height();
        let is_hovered = panel.hovered == Some(index);
        let is_selected = panel.selected == Some(index);

        match item {
            SettingsWidget::TextControl(_) => {}
            SettingsWidget::ColorSwatch(target) => {
                let color = match target {
                    ColorTarget::Main => current_color,
                    ColorTarget::Outline => outline_color,
                    ColorTarget::TextBackground => panel.text_background,
                };
                draw_color_swatch(
                    p,
                    current_x,
                    item_y,
                    item_w,
                    item_h,
                    is_hovered,
                    is_selected,
                    color,
                );
            }
            SettingsWidget::Action { action, svg } => {
                p.item_border(
                    current_x,
                    item_y,
                    item_w,
                    item_h,
                    radius::item(),
                    stroke::border(),
                    is_hovered,
                    is_selected,
                );
                let tint = if is_hovered {
                    cfg.icon_hovered.get()
                } else {
                    cfg.icon.get()
                };
                if let Some(rect) = Rect::from_xywh(current_x, item_y, item_w, item_h) {
                    super::action_icon::draw(p, panel, *action, svg, rect, tint);
                }
            }
            SettingsWidget::Stepper { label, unit, .. } => {
                let display_text = stepper_display_text(panel, index, label, unit);
                draw_stepper(
                    p,
                    current_x,
                    item_y,
                    item_w,
                    item_h,
                    panel,
                    index,
                    &display_text,
                    is_hovered,
                    is_selected,
                    text_color,
                );
            }
            SettingsWidget::Toggle { visual, field } => {
                let is_on = panel.is_toggled(index);
                draw_toggle(
                    p,
                    current_x,
                    item_y,
                    item_w,
                    item_h,
                    visual,
                    field.icon_size() * crate::ui::toolbar::metrics::properties_scale(),
                    is_on,
                    is_hovered,
                    text_color,
                );
            }
            SettingsWidget::Label(text) => {
                if let Some(label_rect) = Rect::from_xywh(current_x, item_y, item_w, item_h) {
                    p.text(
                        label_rect,
                        text,
                        TextStyle::new(crate::ui::toolbar::metrics::font(), text_color).center(),
                    );
                }
            }
            SettingsWidget::Value { field } => {
                if let Some(rect) = Rect::from_xywh(current_x, item_y, item_w, item_h) {
                    draw_value_field(p, rect, &field.text(current_color), is_hovered, text_color);
                }
            }
            SettingsWidget::Download => {
                draw_download(
                    p,
                    current_x,
                    item_y,
                    item_w,
                    item_h,
                    panel.download.unwrap_or(0),
                    text_color,
                );
            }
            SettingsWidget::Separator => {
                let sep_w = theme.separator_width;
                let sep_h = item_h * theme.separator_length;
                let sep_x = current_x + (item_w - sep_w) / 2.0;
                let sep_y = item_y + (item_h - sep_h) / 2.0;

                if let Some(sep_rect) = Rect::from_xywh(sep_x, sep_y, sep_w, sep_h) {
                    p.fill_rounded(sep_rect, radius::separator(), cfg.separator.get());
                }
            }
        }
    }
}

/// looks like the input field, but on click just copies the value to clipboard
fn draw_value_field(p: &mut Painter, rect: Rect, text: &str, is_hovered: bool, text_color: Color) {
    p.fill_rounded(rect, radius::item(), color::field());
    p.item_border(
        rect.left(),
        rect.top(),
        rect.width(),
        rect.height(),
        radius::item(),
        stroke::border(),
        is_hovered,
        false,
    );

    p.text(
        rect,
        text,
        TextStyle::new(crate::ui::toolbar::metrics::font(), text_color).center(),
    );
}

fn draw_download(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, percent: u8, text_color: Color) {
    let cfg = &crate::config::get().settings_panel;
    let (label_w, percent_w) = (cfg.download_label_width, cfg.download_percent_width);
    if let Some(label) = Rect::from_xywh(x, y, label_w, h) {
        p.text(
            label,
            "Downloading",
            TextStyle::new(crate::ui::toolbar::metrics::font(), text_color),
        );
    }

    let bar_w = (w - label_w - percent_w).max(0.0);
    p.progress_bar(
        x + label_w,
        y + (h - stroke::progress()) / 2.0,
        bar_w,
        percent,
    );

    if let Some(label) = Rect::from_xywh(x + w - percent_w, y, percent_w, h) {
        p.text(
            label,
            &format!("{percent}%"),
            TextStyle::new(crate::ui::toolbar::metrics::font(), text_color).center(),
        );
    }
}

fn draw_color_swatch(
    p: &mut Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    is_hovered: bool,
    is_selected: bool,
    color: Color,
) {
    p.item_border(
        x,
        y,
        w,
        h,
        radius::item(),
        stroke::border(),
        is_hovered,
        is_selected,
    );

    let circle_r = (h.min(w) * crate::config::get().settings_panel.swatch_dot).max(4.0);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;

    p.fill_circle(cx, cy, circle_r, color);
}

fn draw_stepper(
    p: &mut Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    panel: &SettingsPanel,
    index: usize,
    text: &str,
    is_hovered: bool,
    is_selected: bool,
    text_color: Color,
) {
    let cfg = &crate::config::get().settings_panel;
    p.item_border(
        x,
        y,
        w,
        h,
        radius::item(),
        stroke::border(),
        is_hovered,
        is_selected,
    );

    let Some(label_rect) = Rect::from_xywh(
        x + size::padding(),
        y,
        (w - cfg.stepper_arrow_zone - size::padding()).max(0.0),
        h,
    ) else {
        let hovered_arrow = panel
            .hovered_arrow
            .filter(|(idx, _)| *idx == index)
            .map(|(_, arrow)| arrow);
        draw_stepper_arrows(p, x, y, w, h, hovered_arrow);
        return;
    };

    let editing = panel
        .fields
        .editing
        .as_ref()
        .filter(|e| e.key == index)
        .map(|e| &e.field);

    p.line_edit(
        label_rect,
        text,
        editing,
        crate::ui::toolbar::metrics::font(),
        text_color,
        if cfg.stepper_bold {
            cosmic_text::Weight::BOLD
        } else {
            cosmic_text::Weight::NORMAL
        },
    );

    let hovered_arrow = panel
        .hovered_arrow
        .filter(|(idx, _)| *idx == index)
        .map(|(_, arrow)| arrow);
    draw_stepper_arrows(p, x, y, w, h, hovered_arrow);
}

fn draw_stepper_arrows(
    p: &mut Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    hovered: Option<StepperArrow>,
) {
    let cfg = &crate::config::get().settings_panel;
    let (arrow_w, arrow_h) = (
        cfg.stepper_arrow_width.min(7.0),
        cfg.stepper_arrow_height.min(3.5),
    );
    let cx = x + w - cfg.stepper_arrow_zone / 2.0;
    let mid_y = y + h / 2.0;

    let up_cy = mid_y - cfg.stepper_arrow_gap.min(5.0) / 2.0 - arrow_h / 2.0;
    let down_cy = mid_y + cfg.stepper_arrow_gap.min(5.0) / 2.0 + arrow_h / 2.0;

    let arrow_color = |arrow| {
        if hovered == Some(arrow) {
            cfg.icon_hovered.get()
        } else {
            cfg.icon.get()
        }
    };
    let up_color = arrow_color(StepperArrow::Up);
    let down_color = arrow_color(StepperArrow::Down);

    let mut stroke = Stroke::default();
    stroke.width = cfg.stepper_arrow_stroke.min(1.25);
    stroke.line_cap = tiny_skia::LineCap::Round;
    stroke.line_join = tiny_skia::LineJoin::Round;

    let mut pb = PathBuilder::new();
    pb.move_to(cx - arrow_w / 2.0, up_cy + arrow_h / 2.0);
    pb.line_to(cx, up_cy - arrow_h / 2.0);
    pb.line_to(cx + arrow_w / 2.0, up_cy + arrow_h / 2.0);
    if let Some(path) = pb.finish() {
        p.stroke_path(&path, up_color, &stroke);
    }

    let mut pb = PathBuilder::new();
    pb.move_to(cx - arrow_w / 2.0, down_cy - arrow_h / 2.0);
    pb.line_to(cx, down_cy + arrow_h / 2.0);
    pb.line_to(cx + arrow_w / 2.0, down_cy - arrow_h / 2.0);
    if let Some(path) = pb.finish() {
        p.stroke_path(&path, down_color, &stroke);
    }
}

fn stepper_display_text(panel: &SettingsPanel, index: usize, label: &str, unit: &str) -> String {
    if let Some(edit) = panel.fields.editing.as_ref().filter(|e| e.key == index) {
        return edit.field.text.clone();
    }

    let value = panel.fields.value(index).map(String::as_str).unwrap_or("");

    if label.is_empty() {
        format!("{value}{unit}")
    } else {
        format!("{label}: {value}{unit}")
    }
}

pub fn char_index_for_x(
    text: &str,
    target_x: f32,
    font_size: f32,
    font_system: &mut FontSystem,
) -> usize {
    if text.is_empty() || target_x <= 0.0 {
        return 0;
    }

    let metrics = cosmic_text::Metrics::new(font_size, font_size * font::line_height());
    let mut buffer = cosmic_text::Buffer::new_empty(metrics);
    buffer.set_size(None, None);
    buffer.set_text(
        text,
        &cosmic_text::Attrs::new().family(cosmic_text::Family::SansSerif),
        cosmic_text::Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);

    if let Some(run) = buffer.layout_runs().next() {
        let mut last_byte = 0usize;
        for glyph in run.glyphs.iter() {
            let glyph_mid = glyph.x + glyph.w / 2.0;
            if target_x < glyph_mid {
                return byte_to_char_index(text, glyph.start);
            }
            last_byte = glyph.end;
        }
        return byte_to_char_index(text, last_byte);
    }
    text.chars().count()
}

fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx.min(text.len())].chars().count()
}

fn draw_toggle(
    p: &mut Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    visual: &ToggleVisual,
    icon_size: f32,
    is_on: bool,
    is_hovered: bool,
    text_color: Color,
) {
    let cfg = &crate::config::get().settings_panel;
    match visual {
        ToggleVisual::Icon { svg } => {
            p.item_border(
                x,
                y,
                w,
                h,
                radius::item(),
                stroke::border(),
                is_hovered,
                is_on,
            );
            let tint = if is_on {
                cfg.icon_active.get()
            } else if is_hovered {
                cfg.icon_hovered.get()
            } else {
                cfg.icon.get()
            };

            let icon_x = x + (w - icon_size) / 2.0;
            let icon_y = y + (h - icon_size) / 2.0;
            p.icon(svg, icon_size, icon_x, icon_y, tint);
        }
        ToggleVisual::Checkbox { label } => {
            let box_size = cfg.checkbox_size;
            let box_x = x + w - box_size;
            let box_y = y + (h - box_size) / 2.0;

            if let Some(label_rect) =
                Rect::from_xywh(x, y, (w - box_size - cfg.checkbox_gap).max(0.0), h)
            {
                p.text(
                    label_rect,
                    label,
                    TextStyle::new(crate::ui::toolbar::metrics::font(), text_color),
                );
            }

            p.item_border(
                box_x,
                box_y,
                box_size,
                box_size,
                cfg.checkbox_radius,
                stroke::border(),
                is_hovered,
                is_on,
            );

            if is_on {
                draw_checkmark(p, box_x, box_y, box_size, text_color);
            }
        }
    }
}

fn draw_checkmark(p: &mut Painter, x: f32, y: f32, size: f32, color: Color) {
    let mut pb = PathBuilder::new();
    pb.move_to(x + size * 0.22, y + size * 0.55);
    pb.line_to(x + size * 0.42, y + size * 0.75);
    pb.line_to(x + size * 0.80, y + size * 0.28);
    let Some(path) = pb.finish() else { return };

    let stroke = Stroke {
        width: crate::config::get().settings_panel.checkmark_width,
        line_cap: tiny_skia::LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        ..Default::default()
    };
    p.stroke_path(&path, color, &stroke);
}
