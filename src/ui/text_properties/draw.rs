//! 【文字属性栏】【原版绘制】绘制属性图标、点字号偏移、透明度和五类按钮，不复用步进输入外观

use super::{
    icons,
    layout::{Control, TextLayout},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::Rgba,
    ui::settings_panel::SettingsPanel,
};
use tiny_skia::{Color, Rect, Stroke};

/// 【文字属性栏】【细边绘制】将 rect 按 radius、width、color 绘入 p，无返回值
pub fn border(p: &mut Painter, rect: Rect, radius: f32, width: f32, color: Rgba) {
    if let Some(path) = crate::renderer::rounded_rect_path(&rect, radius, true, true, true, true) {
        p.stroke_path(
            &path,
            color,
            &Stroke {
                width,
                ..Default::default()
            },
        );
    }
}

/// 【文字属性栏】【内容绘制】将 panel 属性与主色 current_color 绘入 p 的本地面板缓冲，无返回值
pub fn draw(p: &mut Painter, panel: &SettingsPanel, current_color: Color) {
    let Some(layout) = panel.text_layout else {
        return;
    };
    let rect = Rect::from_xywh(0.5, 0.5, layout.size.0 - 1.0, layout.size.1 - 1.0).unwrap();
    p.fill_rounded(
        rect,
        11.0,
        crate::theme::toolbar::Palette::current().panel_fill,
    );
    border(
        p,
        rect,
        11.0,
        1.0,
        crate::theme::toolbar::Palette::current().panel_border,
    );
    crate::ui::annotation_collection::draw::title(p, panel);
    for (index, svg) in [icons::SIZE, icons::OPACITY].into_iter().enumerate() {
        icon(
            p,
            layout.glyphs[index],
            svg,
            layout.glyphs[index].width(),
            crate::theme::toolbar::Palette::current().text,
        );
        let control = if index == 0 {
            Control::Size
        } else {
            Control::Opacity
        };
        let value = if index == 0 {
            panel.text_points.round() as i32 - 19
        } else {
            panel.text_alpha
        };
        let (min, max) = if index == 0 {
            if matches!(
                panel.active_source,
                Some(crate::ui::settings_panel::SettingsSource::Collection { .. })
            ) {
                (1, 24)
            } else {
                (-11, 281)
            }
        } else {
            (0, 100)
        };
        slider(
            p,
            layout.controls[index].unwrap(),
            value,
            (min, max),
            panel.hovered == Some(control.index()),
        );
        let text = if index == 0 {
            value.to_string()
        } else {
            format!("{value}%")
        };
        let mut style = TextStyle::new(
            layout.value_font_size,
            crate::theme::toolbar::Palette::current().strong_text,
        )
        .weight(cosmic_text::Weight::BOLD);
        style.align = crate::renderer::text::HAlign::Right;
        p.text(layout.values[index], &text, style);
    }
    let palette = crate::theme::toolbar::Palette::current();
    // 1. 在同一行内为字号滑块组、透明度滑块组与右侧样式按钮之间补充柔和纵向分隔细线
    for (left_rect, right_rect) in [
        (Some(layout.values[0]), Some(layout.glyphs[1])),
        (
            Some(layout.values[1]),
            layout.controls[Control::Color.index()],
        ),
    ] {
        if let (Some(l), Some(r)) = (left_rect, right_rect)
            && (l.top() - r.top()).abs() < 12.0
            && r.left() > l.right()
        {
            let sep_x = ((l.right() + r.left()) * 0.5).floor();
            let sep_h = 16.0;
            let sep_y = (l.top() + l.bottom() - sep_h) * 0.5;
            if let Some(sep) = Rect::from_xywh(sep_x, sep_y, 1.0, sep_h) {
                p.fill_rounded(sep, 0.5, palette.separator.with_alpha(52));
            }
        }
    }

    for (control, svg) in [
        (Control::Color, icons::COLOR),
        (Control::Background, icons::BACKGROUND),
        (Control::Font, icons::FONT),
        (Control::Edit, icons::EDIT),
    ] {
        let Some(cell) = layout.controls[control.index()] else {
            continue;
        };
        let hovered = panel.hovered == Some(control.index());
        let selected = panel.selected == Some(control.index());
        let mut ink = palette.text;
        if matches!(control, Control::Color | Control::Background) {
            let color = if control == Control::Color {
                current_color
            } else {
                panel.text_background
            };
            let color = color.to_color_u8();
            let inset = Rect::from_xywh(
                cell.left() + 1.5,
                cell.top() + 1.5,
                cell.width() - 3.0,
                cell.height() - 3.0,
            )
            .unwrap();
            // 2. 半透明文字背景色自动叠加微棋盘衬底，避免透明态与面板底色混淆
            if color.alpha() < 255 {
                crate::ui::settings_panel::draw_controls::draw_alpha_checker(p, inset, 7.5);
            }
            p.fill_rounded(
                inset,
                7.5,
                Rgba(color.red(), color.green(), color.blue(), color.alpha()),
            );
            border(
                p,
                inset,
                7.5,
                if hovered || selected { 1.5 } else { 1.0 },
                if hovered || selected {
                    palette.hover_border
                } else {
                    palette.control_border
                },
            );
            let luma = (u32::from(color.red()) * 299
                + u32::from(color.green()) * 587
                + u32::from(color.blue()) * 114)
                / 1000;
            ink = if color.alpha() <= 80 {
                Rgba(30, 41, 59, 255)
            } else if luma > 150 && color.alpha() > 120 {
                Rgba(11, 17, 32, 255)
            } else {
                Rgba(248, 250, 252, 255)
            };
        } else if hovered || selected {
            let inset = Rect::from_xywh(
                cell.left() + 0.5,
                cell.top() + 0.5,
                cell.width() - 1.0,
                cell.height() - 1.0,
            )
            .unwrap();
            p.fill_rounded(
                inset,
                7.0,
                if hovered {
                    crate::theme::toolbar::Palette::current().hover_fill
                } else {
                    crate::theme::toolbar::Palette::current().active_fill
                },
            );
            border(
                p,
                inset,
                7.0,
                1.0,
                if hovered {
                    crate::theme::toolbar::Palette::current().hover_border
                } else {
                    crate::theme::toolbar::Palette::current().active_border
                },
            );
            if hovered {
                ink = crate::theme::toolbar::Palette::current().hover_icon;
            }
        }
        icon(p, cell, svg, layout.icon_size, ink);
    }
}

/// 【文字属性栏】【图标居中】将 svg 按 size 与 ink 居中绘入 p 的 cell，无返回值
fn icon(p: &mut Painter, cell: Rect, svg: &'static str, size: f32, ink: Rgba) {
    p.icon(
        svg,
        size,
        cell.left() + (cell.width() - size) / 2.0,
        cell.top() + (cell.height() - size) / 2.0,
        ink,
    );
}

/// 【文字属性栏】【滑条绘制】按 value、range 与 hovered 绘入 p 的 cell，保留实际四像素轨道和十四像素手柄宽度
fn slider(p: &mut Painter, cell: Rect, value: i32, range: (i32, i32), hovered: bool) {
    let groove = Rect::from_xywh(
        cell.left(),
        cell.top() + ((cell.height() - 4.0) / 2.0).floor(),
        cell.width(),
        4.0,
    )
    .unwrap();
    p.fill_rounded(
        groove,
        2.0,
        crate::theme::toolbar::Palette::current().slider_groove,
    );
    let handle = TextLayout::slider_handle(cell, value, range);
    if let Some(filled) = Rect::from_xywh(
        groove.left(),
        groove.top(),
        handle.left() - groove.left() + 7.0,
        4.0,
    ) {
        // 1. 【文字属性栏】【横向渐变】原版滑条填充从暗青色逐渐过渡到亮青色
        let mut paint = tiny_skia::Paint::default();
        paint.shader = tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(filled.left(), filled.top()),
            tiny_skia::Point::from_xy(filled.right(), filled.top()),
            vec![
                tiny_skia::GradientStop::new(
                    0.0,
                    crate::theme::toolbar::Palette::current()
                        .slider_start
                        .color(),
                ),
                tiny_skia::GradientStop::new(
                    1.0,
                    crate::theme::toolbar::Palette::current().slider_end.color(),
                ),
            ],
            tiny_skia::SpreadMode::Pad,
            tiny_skia::Transform::identity(),
        )
        .unwrap();
        if let Some(path) = crate::renderer::rounded_rect_path(&filled, 2.0, true, true, true, true)
        {
            let transform = p.transform();
            p.canvas()
                .fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
        }
    }
    let inset = Rect::from_xywh(
        handle.left() + 0.5,
        handle.top() + 0.5,
        handle.width() - 1.0,
        handle.height() - 1.0,
    )
    .unwrap();
    p.fill_rounded(inset, 6.0, Rgba(255, 255, 255, 255));
    border(
        p,
        inset,
        6.0,
        1.0,
        if hovered {
            crate::theme::toolbar::Palette::current().hover_border
        } else {
            crate::theme::toolbar::Palette::current().slider_handle_border
        },
    );
}
