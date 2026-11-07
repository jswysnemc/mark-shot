//! 【截图历史】【窗口框架】原版深色面板、矢量操作栏、记录复选框和按需状态文字

use super::{
    layout::{Control, rect},
    window::{Focus, HistoryWindow},
};
use crate::{
    renderer::{Icons, Painter, text::TextStyle},
    theme::Rgba,
    ui::{
        controls::{self, ButtonStyle},
        icons,
    },
};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Weight};
use tiny_skia::Pixmap;

/// 【截图历史】【主题角色】返回当前主题的窗口底色，无参数
pub(super) fn background() -> Rgba {
    crate::theme::window::Palette::current().window
}
/// 【截图历史】【主题角色】返回当前主题的主要文字颜色，无参数
pub(super) fn foreground() -> Rgba {
    crate::theme::window::Palette::current().text
}
/// 【截图历史】【主题角色】返回当前主题的次要文字颜色，无参数
pub(super) fn muted() -> Rgba {
    crate::theme::window::Palette::current().muted
}
/// 【截图历史】【主题角色】返回当前主题的边框颜色，无参数
pub(super) fn border() -> Rgba {
    crate::theme::window::Palette::current().border
}
/// 【截图历史】【主题角色】返回当前主题的按钮底色，无参数
pub(super) fn hover() -> Rgba {
    crate::theme::window::Palette::current().alternate
}
/// 【截图历史】【主题角色】返回当前主题的选中底色，无参数
pub(super) fn selected() -> Rgba {
    crate::theme::window::Palette::current().pressed
}
/// 【截图历史】【主题角色】返回当前主题的强调颜色，无参数
pub(super) fn accent() -> Rgba {
    crate::theme::window::Palette::current().accent
}
const CLEAR: &str = include_str!("../../../assets/icons/mark-shot/clear.svg");

/// 【截图历史】【图标缓存】只解析历史窗口使用的原版图标，返回矢量缓存，不加载全部标注工具
pub(super) fn icons() -> Icons {
    [
        icons::COPY,
        icons::PEN,
        icons::PIN,
        icons::SAVE,
        icons::CLOSE,
        icons::CHECK,
        CLEAR,
    ]
    .into_iter()
    .map(|svg| {
        (
            svg,
            usvg::Tree::from_str(svg, &usvg::Options::default())
                .expect("embedded history icon must parse"),
        )
    })
    .collect()
}

/// 【截图历史】【状态排版】按 fonts、text 和 width 返回可换行状态缓冲，空文字不占据布局，不读取磁盘
pub(super) fn status_buffer(
    fonts: &mut FontSystem,
    text: Option<&str>,
    width: f32,
) -> Option<Buffer> {
    let text = text.filter(|text| !text.is_empty())?;
    let mut buffer = Buffer::new(fonts, Metrics::new(40.0 / 3.0, 19.0));
    buffer.set_size(Some(width.max(1.0)), None);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(fonts, false);
    Some(buffer)
}

/// 【截图历史】【按钮图标】返回 control 的原版矢量资源，复选框及确认按钮不使用图像操作图标
fn icon(control: Control) -> Option<&'static str> {
    match control {
        Control::Close => Some(icons::CLOSE),
        Control::Copy => Some(icons::COPY),
        Control::Edit => Some(icons::PEN),
        Control::Pin => Some(icons::PIN),
        Control::Save => Some(icons::SAVE),
        Control::Delete | Control::Clear => Some(CLEAR),
        _ => None,
    }
}

/// 【截图历史】【按钮样式】用 state 和 control 生成原版前景、背景及边框，不改变业务状态
fn button_style(state: &HistoryWindow, control: Control) -> ButtonStyle {
    let enabled = state.enabled(control);
    let hovered =
        state.hovered == Some(control) || (state.focused && state.focus == Focus::Control(control));
    let quiet = matches!(control, Control::Close | Control::Clear);
    let primary = control == Control::Copy;
    let fill = if !enabled {
        hover()
    } else if primary {
        accent()
    } else if quiet && !hovered {
        Rgba(0, 0, 0, 0)
    } else {
        hover()
    };
    let ink = if !enabled {
        muted()
    } else if primary {
        background()
    } else {
        foreground()
    };
    let border = if !enabled || (quiet && hovered) {
        border()
    } else if primary && hovered {
        foreground()
    } else if primary || hovered {
        accent()
    } else if quiet {
        Rgba(0, 0, 0, 0)
    } else {
        border()
    };
    ButtonStyle {
        fill,
        foreground: ink,
        border,
        font_size: 40.0 / 3.0,
        weight: Weight(if primary { 600 } else { 500 }),
    }
}

/// 【截图历史】【框架绘制】把 state 的摘要、操作栏与记录开关绘制到 canvas，scale 为物理比例，无磁盘读取
pub(super) fn draw(state: &mut HistoryWindow, canvas: &mut Pixmap, scale: f32) {
    let styles: Vec<_> = state
        .layout
        .buttons
        .iter()
        .map(|(control, _)| button_style(state, *control))
        .collect();
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_icons(&state.icons)
        .with_text(&mut state.fonts, &mut state.cache);
    painter.text(
        state.layout.summary,
        &format!("Screenshot History · {}", state.model.entries.len()),
        TextStyle::new(40.0 / 3.0, foreground()).weight(Weight(600)),
    );
    for ((control, bounds), style) in state.layout.buttons.iter().copied().zip(styles) {
        if control == Control::Enabled {
            let checked = state.settings.enabled;
            controls::checkbox(
                &mut painter,
                bounds,
                control.label(),
                checked,
                ButtonStyle {
                    fill: if checked { accent() } else { background() },
                    foreground: style.foreground,
                    border: if checked || state.hovered == Some(control) {
                        accent()
                    } else {
                        border()
                    },
                    font_size: 12.0,
                    weight: Weight::NORMAL,
                },
                background(),
            );
        } else {
            controls::icon_button(
                &mut painter,
                bounds,
                if control == Control::Close {
                    ""
                } else {
                    control.label()
                },
                icon(control),
                style,
            );
        }
    }
    if state.splitter_hovered || state.splitter_drag.is_some() {
        painter.fill_rounded(state.layout.splitter, 3.0, border());
    }
    if let Some(bounds) = state.layout.status
        && let Some(buffer) =
            status_buffer(&mut state.fonts, state.status.as_deref(), bounds.width())
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        painter.text_buffer(&buffer, (bounds.x(), bounds.y()), muted().color());
    }
}

/// 【截图历史】【行区域】返回 viewport 的 row 记录框；item_width 为扣除滚动条后的可见宽度
pub(super) fn row_rect(row: usize, item_width: f32) -> tiny_skia::Rect {
    rect(
        2.0,
        2.0 + row as f32 * super::layout::ROW,
        (item_width - 4.0).max(1.0),
        44.0,
    )
}
