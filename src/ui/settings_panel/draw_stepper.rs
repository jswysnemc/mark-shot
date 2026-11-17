//! 【副工具栏】【步进器绘制】提供现代内嵌胶囊步进输入框、聚焦描边、上下独立悬停区与字符命中换算

use super::draw_controls::draw_chip_frame;
use crate::{
    renderer::Painter,
    theme::{font, size, toolbar::Palette},
    ui::settings_panel::{SettingsPanel, StepperArrow},
};
use cosmic_text::{FontSystem, Weight};
use tiny_skia::{PathBuilder, Rect, Stroke};

const STEPPER_RADIUS: f32 = 6.5;

/// 【副工具栏】【步进文本】按当前编辑状态或确认数值拼接标签与单位，返回显示文本
pub(super) fn stepper_display_text(
    panel: &SettingsPanel,
    index: usize,
    label: &str,
    unit: &str,
) -> String {
    if let Some(edit) = panel.fields.editing.as_ref().filter(|e| e.key == index) {
        return edit.field.text.clone();
    }
    let value = panel.fields.value(index).map(String::as_str).unwrap_or("");
    if label.is_empty() {
        format!("{value}{unit}")
    } else {
        format!("{label} {value}{unit}")
    }
}

/// 【副工具栏】【步进胶囊】绘制带聚焦态、微分隔线与上下箭头独立悬停背景的数值步进控件，无返回值
pub(super) fn draw_stepper(
    p: &mut Painter,
    cell: Rect,
    panel: &SettingsPanel,
    index: usize,
    text: &str,
    is_hovered: bool,
    is_selected: bool,
) {
    let cfg = &crate::config::get().settings_panel;
    let palette = Palette::current();
    let editing = panel
        .fields
        .editing
        .as_ref()
        .filter(|e| e.key == index)
        .map(|e| &e.field);

    // 1. 区分键盘编辑聚焦态、鼠标悬停态与常规静止态的胶囊底色及描边
    let (fill, border, border_w) = if editing.is_some() {
        (palette.list_fill, palette.focus_border, 1.25)
    } else if is_selected {
        (palette.active_fill, palette.active_border, 1.0)
    } else if is_hovered {
        (palette.control_hover_fill, palette.soft_hover_border, 1.0)
    } else {
        (palette.control_fill, palette.control_border, 1.0)
    };
    draw_chip_frame(p, cell, STEPPER_RADIUS, fill, border, border_w);

    let hovered_arrow = panel
        .hovered_arrow
        .filter(|(idx, _)| *idx == index)
        .map(|(_, arrow)| arrow);

    // 2. 在右侧步进箭头区域前绘制柔和纵向细分隔线，并为悬停的半区绘制微高亮
    let arrow_zone = cfg.stepper_arrow_zone.min(cell.width() * 0.45);
    let divider_x = cell.right() - arrow_zone;
    if let Some(arrow) = hovered_arrow {
        let half_h = (cell.height() - 4.0) * 0.5;
        let zone_y = match arrow {
            StepperArrow::Up => cell.top() + 2.0,
            StepperArrow::Down => cell.top() + 2.0 + half_h,
        };
        if let Some(zone_rect) =
            Rect::from_xywh(divider_x + 1.5, zone_y, (arrow_zone - 3.5).max(1.0), half_h)
        {
            p.fill_rounded(zone_rect, 4.0, palette.soft_hover_fill);
        }
    }
    if let Some(sep) = Rect::from_xywh(
        divider_x,
        cell.top() + cell.height() * 0.22,
        1.0,
        cell.height() * 0.56,
    ) {
        p.fill_rounded(sep, 0.5, palette.separator.with_alpha(48));
    }

    // 3. 绘制左侧可编辑文本区域，几何边距与字段输入法光标定位严格对齐
    if let Some(label_rect) = Rect::from_xywh(
        cell.left() + size::padding(),
        cell.top(),
        (cell.width() - cfg.stepper_arrow_zone - size::padding()).max(0.0),
        cell.height(),
    ) {
        p.line_edit(
            label_rect,
            text,
            editing,
            crate::ui::toolbar::metrics::font(),
            palette.strong_text.color(),
            if cfg.stepper_bold {
                Weight::SEMIBOLD
            } else {
                Weight::NORMAL
            },
        );
    }

    draw_stepper_arrows(p, cell, arrow_zone, hovered_arrow);
}

/// 【副工具栏】【步进箭头】在 cell 右侧步进区内绘制圆角上下微折线，无返回值
fn draw_stepper_arrows(
    p: &mut Painter,
    cell: Rect,
    arrow_zone: f32,
    hovered: Option<StepperArrow>,
) {
    let cfg = &crate::config::get().settings_panel;
    let palette = Palette::current();
    let arrow_w = cfg.stepper_arrow_width.min(6.5);
    let arrow_h = cfg.stepper_arrow_height.min(3.2);
    let cx = cell.right() - arrow_zone * 0.5;
    let mid_y = cell.top() + cell.height() * 0.5;
    let gap = cfg.stepper_arrow_gap.min(4.5);

    let up_cy = mid_y - gap * 0.5 - arrow_h * 0.5;
    let down_cy = mid_y + gap * 0.5 + arrow_h * 0.5;

    let arrow_color = |arrow| {
        if hovered == Some(arrow) {
            palette.slider_end
        } else {
            palette.muted_text
        }
    };

    let stroke = Stroke {
        width: cfg.stepper_arrow_stroke.min(1.35),
        line_cap: tiny_skia::LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        ..Default::default()
    };

    let mut pb = PathBuilder::new();
    pb.move_to(cx - arrow_w * 0.5, up_cy + arrow_h * 0.5);
    pb.line_to(cx, up_cy - arrow_h * 0.5);
    pb.line_to(cx + arrow_w * 0.5, up_cy + arrow_h * 0.5);
    if let Some(path) = pb.finish() {
        p.stroke_path(&path, arrow_color(StepperArrow::Up), &stroke);
    }

    let mut pb = PathBuilder::new();
    pb.move_to(cx - arrow_w * 0.5, down_cy - arrow_h * 0.5);
    pb.line_to(cx, down_cy + arrow_h * 0.5);
    pb.line_to(cx + arrow_w * 0.5, down_cy - arrow_h * 0.5);
    if let Some(path) = pb.finish() {
        p.stroke_path(&path, arrow_color(StepperArrow::Down), &stroke);
    }
}

/// 【副工具栏】【字符定位】按文本宽度与点击偏移 target_x 计算字符索引，返回光标插入下标
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
            let glyph_mid = glyph.x + glyph.w * 0.5;
            if target_x < glyph_mid {
                return byte_to_char_index(text, glyph.start);
            }
            last_byte = glyph.end;
        }
        return byte_to_char_index(text, last_byte);
    }
    text.chars().count()
}

/// 【副工具栏】【字节转字符】将 UTF-8 字节偏移安全映射为字符下标，返回字符数量
fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx.min(text.len())].chars().count()
}
