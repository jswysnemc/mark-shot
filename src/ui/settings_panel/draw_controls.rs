//! 【副工具栏】【基础控件绘制】统一绘制双环色板、胶囊开关、取色值芯片与下载进度条

use crate::{
    renderer::{Painter, paths::rounded_rect_path, text::TextStyle},
    theme::{Rgba, toolbar::Palette},
    ui::{language::Language, settings_panel::ToggleVisual},
};
use cosmic_text::Weight;
use tiny_skia::{Color, PathBuilder, Rect, Stroke};

const CHIP_RADIUS: f32 = 6.5;

/// 【副工具栏】【细边胶囊】在 p 中按 rect、radius、fill 与 border 绘制圆角背景及一像素内描边，无返回值
pub(super) fn draw_chip_frame(
    p: &mut Painter,
    rect: Rect,
    radius: f32,
    fill: Rgba,
    border: Rgba,
    border_width: f32,
) {
    let inset = border_width * 0.5;
    let Some(inner) = Rect::from_xywh(
        rect.left() + inset,
        rect.top() + inset,
        (rect.width() - border_width).max(0.5),
        (rect.height() - border_width).max(0.5),
    ) else {
        return;
    };
    p.fill_rounded(inner, (radius - inset).max(0.0), fill);
    if let Some(path) = rounded_rect_path(&inner, (radius - inset).max(0.0), true, true, true, true)
    {
        p.stroke_path(
            &path,
            border,
            &Stroke {
                width: border_width,
                ..Default::default()
            },
        );
    }
}

/// 【副工具栏】【透明棋盘格】在半透明颜色块底层绘制柔和明暗方格，帮助区分不透明度，无返回值
pub(crate) fn draw_alpha_checker(p: &mut Painter, rect: Rect, radius: f32) {
    let light = Rgba(226, 232, 240, 220);
    let dark = Rgba(148, 163, 184, 180);
    p.fill_rounded(rect, radius, light);
    let inset = 2.0;
    let Some(inner) = Rect::from_xywh(
        rect.left() + inset,
        rect.top() + inset,
        (rect.width() - inset * 2.0).max(1.0),
        (rect.height() - inset * 2.0).max(1.0),
    ) else {
        return;
    };
    let step: f32 = 4.0;
    let mut y = inner.top();
    let mut row = 0usize;
    while y < inner.bottom() {
        let h = step.min(inner.bottom() - y);
        let mut x = inner.left();
        let mut col = 0usize;
        while x < inner.right() {
            let w = step.min(inner.right() - x);
            if (row + col) % 2 == 1
                && let Some(cell) = Rect::from_xywh(x, y, w, h)
            {
                p.fill_rounded(cell, 0.5, dark);
            }
            x += step;
            col += 1;
        }
        y += step;
        row += 1;
    }
}

/// 【副工具栏】【色板绘制】绘制带外侧交互态与内侧对比环的圆角颜色块，支持半透明棋盘衬底，无返回值
pub(super) fn draw_color_swatch(
    p: &mut Painter,
    cell: Rect,
    is_hovered: bool,
    is_selected: bool,
    color: Color,
) {
    let palette = Palette::current();
    // 1. 悬停或展开取色器时绘制与主工具栏一致的柔和外框底座
    if is_hovered || is_selected {
        let (fill, border) = if is_selected {
            (palette.active_fill, palette.active_border)
        } else {
            (palette.soft_hover_fill, palette.soft_hover_border)
        };
        draw_chip_frame(p, cell, CHIP_RADIUS, fill, border, 1.0);
    }

    // 2. 绘制居中圆角方形色芯，半透明颜色自动叠加棋盘衬底与双层对比边环
    let edge = (cell.width().min(cell.height()) - 10.0).clamp(14.0, 22.0);
    let Some(core) = Rect::from_xywh(
        cell.left() + (cell.width() - edge) * 0.5,
        cell.top() + (cell.height() - edge) * 0.5,
        edge,
        edge,
    ) else {
        return;
    };
    let swatch_radius = 5.5;
    let rgba = color.to_color_u8();
    if rgba.alpha() < 255 {
        draw_alpha_checker(p, core, swatch_radius);
    }
    p.fill_rounded(
        core,
        swatch_radius,
        Rgba(rgba.red(), rgba.green(), rgba.blue(), rgba.alpha()),
    );

    if let Some(outer_ring) = rounded_rect_path(&core, swatch_radius, true, true, true, true) {
        p.stroke_path(
            &outer_ring,
            if is_hovered || is_selected {
                palette.hover_border
            } else {
                Rgba(15, 23, 42, 72)
            },
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
        );
    }
    if let Some(inner) = Rect::from_xywh(
        core.left() + 1.0,
        core.top() + 1.0,
        (core.width() - 2.0).max(1.0),
        (core.height() - 2.0).max(1.0),
    ) && let Some(inner_ring) = rounded_rect_path(
        &inner,
        (swatch_radius - 1.0).max(1.0),
        true,
        true,
        true,
        true,
    ) {
        p.stroke_path(
            &inner_ring,
            Rgba(255, 255, 255, 110),
            &Stroke {
                width: 0.75,
                ..Default::default()
            },
        );
    }
}

/// 【副工具栏】【开关文案】按当前界面语言转换内置开关标签，返回显示字符串
pub(super) fn localized_toggle_label(label: &'static str) -> &'static str {
    if Language::current() == Language::Chinese {
        match label {
            "Fill" => "填充",
            _ => Language::Chinese.text(label),
        }
    } else {
        label
    }
}

/// 【副工具栏】【开关绘制】统一绘制图标开关与紧凑胶囊文字开关，无返回值
pub(super) fn draw_toggle(
    p: &mut Painter,
    cell: Rect,
    visual: &ToggleVisual,
    caption: Option<&str>,
    icon_size: f32,
    is_on: bool,
    is_hovered: bool,
) {
    let palette = Palette::current();
    let cfg = &crate::config::get().settings_panel;
    match visual {
        ToggleVisual::Icon { svg } => {
            if is_hovered || is_on {
                let (fill, border) = if is_hovered {
                    (palette.hover_fill, palette.hover_border)
                } else {
                    (palette.active_fill, palette.active_border)
                };
                draw_chip_frame(p, cell, CHIP_RADIUS, fill, border, 1.0);
            }
            let tint = if is_hovered {
                palette.hover_icon
            } else if is_on {
                palette.strong_text
            } else {
                cfg.icon.get()
            };
            let size = icon_size.min(cell.width() - 6.0).min(cell.height() - 6.0);
            p.icon(
                svg,
                size,
                cell.left() + (cell.width() - size) * 0.5,
                cell.top() + (cell.height() - size) * 0.5,
                tint,
            );
        }
        ToggleVisual::Checkbox { label } => {
            // 1. 采用现代胶囊芯片替代旧式方框勾选框，通过柔和底色与状态点表达启用状态
            let (fill, border, ink, dot) = if is_on && is_hovered {
                (
                    palette.accent_hover_fill,
                    palette.accent_hover_border,
                    palette.strong_text,
                    palette.slider_end,
                )
            } else if is_on {
                (
                    palette.active_fill,
                    palette.active_border,
                    palette.strong_text,
                    palette.slider_end,
                )
            } else if is_hovered {
                (
                    palette.soft_hover_fill,
                    palette.soft_hover_border,
                    palette.strong_text,
                    palette.muted_text,
                )
            } else {
                (
                    palette.control_fill,
                    palette.control_border,
                    palette.text,
                    palette.muted_text.with_alpha(120),
                )
            };
            draw_chip_frame(p, cell, CHIP_RADIUS, fill, border, 1.0);

            // 2. 左侧绘制紧凑圆点或勾选态标记，右侧居中绘制开关文字
            let dot_x = cell.left() + 10.0;
            let dot_y = cell.top() + cell.height() * 0.5;
            if is_on {
                p.fill_circle(dot_x, dot_y, 3.2, dot.color());
                let mut check = PathBuilder::new();
                check.move_to(dot_x - 1.6, dot_y + 0.1);
                check.line_to(dot_x - 0.3, dot_y + 1.4);
                check.line_to(dot_x + 1.9, dot_y - 1.2);
                if let Some(path) = check.finish() {
                    p.stroke_path(
                        &path,
                        palette.primary_text,
                        &Stroke {
                            width: 1.2,
                            line_cap: tiny_skia::LineCap::Round,
                            line_join: tiny_skia::LineJoin::Round,
                            ..Default::default()
                        },
                    );
                }
            } else {
                let mut ring = PathBuilder::new();
                ring.push_circle(dot_x, dot_y, 2.75);
                if let Some(path) = ring.finish() {
                    p.stroke_path(
                        &path,
                        dot,
                        &Stroke {
                            width: 1.1,
                            ..Default::default()
                        },
                    );
                }
            }

            if let Some(label_rect) = Rect::from_xywh(
                cell.left() + 17.0,
                cell.top(),
                (cell.width() - 22.0).max(1.0),
                cell.height(),
            ) {
                p.text(
                    label_rect,
                    caption.unwrap_or_else(|| localized_toggle_label(label)),
                    TextStyle::new(crate::ui::toolbar::metrics::font(), ink.color())
                        .weight(if is_on {
                            Weight::SEMIBOLD
                        } else {
                            Weight::MEDIUM
                        })
                        .center(),
                );
            }
        }
    }
}

/// 【取色工具条】【颜色预览】在 rect 内绘制可复制的 text；swatch 有值时在文字左侧显示同色样本，is_hovered 控制高亮，无返回值
pub(super) fn draw_value_field(
    p: &mut Painter,
    rect: Rect,
    text: &str,
    swatch: Option<Color>,
    is_hovered: bool,
) {
    let palette = Palette::current();
    let (fill, border, ink) = if is_hovered {
        (
            palette.soft_hover_fill,
            palette.soft_hover_border,
            palette.strong_text,
        )
    } else {
        (palette.control_fill, palette.control_border, palette.text)
    };
    draw_chip_frame(p, rect, CHIP_RADIUS, fill, border, 1.0);
    let mut text_rect = rect;
    if let Some(color) = swatch {
        let width = (crate::config::get().settings_panel.swatch_size
            * crate::ui::toolbar::metrics::properties_scale())
        .min(rect.width() * 0.35);
        if let (Some(sample), Some(rest)) = (
            Rect::from_xywh(rect.left(), rect.top(), width, rect.height()),
            Rect::from_ltrb(rect.left() + width, rect.top(), rect.right(), rect.bottom()),
        ) {
            draw_color_swatch(p, sample, false, false, color);
            text_rect = rest;
        }
    }
    p.text(
        text_rect,
        text,
        TextStyle::new(crate::ui::toolbar::metrics::font(), ink.color())
            .weight(Weight::MEDIUM)
            .center(),
    );
}

/// 【副工具栏】【下载进度】在属性栏内绘制本地化的模型下载进度条与百分比，无返回值
pub(super) fn draw_download(p: &mut Painter, cell: Rect, percent: u8) {
    let palette = Palette::current();
    let cfg = &crate::config::get().settings_panel;
    let label_text = if Language::current() == Language::Chinese {
        "正在下载"
    } else {
        "Downloading"
    };
    let (label_w, percent_w) = (cfg.download_label_width, cfg.download_percent_width);
    let font_size = crate::ui::toolbar::metrics::font();
    if let Some(label) = Rect::from_xywh(cell.left(), cell.top(), label_w, cell.height()) {
        p.text(
            label,
            label_text,
            TextStyle::new(font_size, palette.muted_text.color()),
        );
    }

    let bar_h = 5.0;
    let bar_w = (cell.width() - label_w - percent_w).max(0.0);
    let bar_x = cell.left() + label_w;
    let bar_y = cell.top() + (cell.height() - bar_h) * 0.5;
    if bar_w > 0.0
        && let Some(track) = Rect::from_xywh(bar_x, bar_y, bar_w, bar_h)
    {
        p.fill_rounded(track, bar_h * 0.5, palette.slider_groove);
        let filled = (bar_w * f32::from(percent.min(100)) / 100.0).max(if percent > 0 {
            bar_h
        } else {
            0.0
        });
        if filled > 0.0
            && let Some(active) = Rect::from_xywh(bar_x, bar_y, filled.min(bar_w), bar_h)
        {
            p.fill_rounded(active, bar_h * 0.5, palette.slider_end);
        }
    }

    if let Some(label) = Rect::from_xywh(
        cell.right() - percent_w,
        cell.top(),
        percent_w,
        cell.height(),
    ) {
        p.text(
            label,
            &format!("{percent}%"),
            TextStyle::new(font_size, palette.strong_text.color())
                .weight(Weight::MEDIUM)
                .center(),
        );
    }
}
