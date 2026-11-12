//! 【界面控件】【基础按钮】独立窗口共用紧凑按钮与文字截断规则

mod choice;
mod segmented;
mod select;
mod switch;

pub use choice::{Choice, ChoiceEvent};
pub use segmented::{segment, segmented};
pub use select::select;
pub use switch::{SWITCH_SIZE, switch};

use tiny_skia::{Rect, Stroke};

use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{Rgba, color, font, radius},
};

pub struct ButtonStyle {
    pub fill: Rgba,
    pub foreground: Rgba,
    pub border: Rgba,
    pub font_size: f32,
    pub weight: cosmic_text::Weight,
}

/// 【界面控件】【圆角边框】用 painter 按 fill、border 和 radius 绘制 bounds，无返回值；预览和按钮共享边框规则
pub fn frame(painter: &mut Painter<'_>, bounds: Rect, fill: Rgba, border: Rgba, radius: f32) {
    painter.fill_rounded(bounds, radius, fill);
    if let Some(inset) = Rect::from_xywh(
        bounds.x() + 0.5,
        bounds.y() + 0.5,
        bounds.width() - 1.0,
        bounds.height() - 1.0,
    ) && let Some(path) = crate::renderer::paths::rounded_rect_path(
        &inset,
        (radius - 0.5).max(0.0),
        true,
        true,
        true,
        true,
    ) {
        painter.stroke_path(
            &path,
            border.color(),
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
        );
    }
}

/// 【界面控件】【矢量按钮】用 painter 在 bounds 内按 style 绘制 label 和 icon，返回空值；图标与文字使用同一前景色
pub fn icon_button(
    painter: &mut Painter<'_>,
    bounds: Rect,
    label: &str,
    icon: Option<&'static str>,
    style: ButtonStyle,
) {
    frame(painter, bounds, style.fill, style.border, 6.0);
    let text_style = TextStyle::new(style.font_size, style.foreground).weight(style.weight);
    let text_width = painter.measure_styled(label, text_style);
    let icon_width = if icon.is_some() { 16.0 } else { 0.0 };
    let gap = if icon.is_some() && !label.is_empty() {
        4.0
    } else {
        0.0
    };
    let x = bounds.x() + (bounds.width() - icon_width - gap - text_width) * 0.5;
    if let Some(icon) = icon {
        painter.icon(
            icon,
            16.0,
            x,
            bounds.y() + (bounds.height() - 16.0) * 0.5,
            style.foreground,
        );
    }
    if !label.is_empty() {
        painter.text(
            Rect::from_xywh(
                x + icon_width + gap,
                bounds.y(),
                text_width.max(1.0),
                bounds.height(),
            )
            .unwrap(),
            label,
            text_style,
        );
    }
}

/// 【界面控件】【记录复选框】用 painter 在 bounds 内按 checked、style 和 mark 绘制 label 与指示器，无返回值
pub fn checkbox(
    painter: &mut Painter<'_>,
    bounds: Rect,
    label: &str,
    checked: bool,
    style: ButtonStyle,
    mark: Rgba,
) {
    let indicator = Rect::from_xywh(bounds.x(), bounds.y(), 18.0, 18.0).unwrap();
    frame(painter, indicator, style.fill, style.border, 4.0);
    if checked {
        painter.icon(
            crate::ui::icons::CHECK,
            12.0,
            bounds.x() + 3.0,
            bounds.y() + 3.0,
            mark,
        );
    }
    painter.text(
        Rect::from_xywh(
            bounds.x() + 26.0,
            bounds.y(),
            (bounds.width() - 26.0).max(1.0),
            bounds.height(),
        )
        .unwrap(),
        label,
        TextStyle::new(style.font_size, style.foreground).weight(style.weight),
    );
}

/// 【界面控件】【按钮绘制】按 rect 绘制 label；active、hovered、enabled 控制统一颜色状态
pub fn button(
    painter: &mut Painter<'_>,
    rect: Rect,
    label: &str,
    active: bool,
    hovered: bool,
    enabled: bool,
) {
    let fill = if active {
        color::active()
    } else if hovered && enabled {
        color::hover()
    } else {
        color::field()
    };
    painter.fill_rounded(rect, radius::item().min(6.0), fill);
    let foreground = if !enabled {
        color::muted()
    } else if active {
        color::on_active()
    } else if hovered {
        crate::theme::toolbar::Palette::current().hover_text
    } else {
        color::foreground()
    };
    painter.text(
        rect,
        label,
        TextStyle::new(font::label(), foreground).center(),
    );
}

/// 【界面控件】【文字截断】将 text 缩短到 width 内；通过字符边界保留 UTF-8，返回显示文字
pub fn fit_label(painter: &mut Painter<'_>, text: &str, size: f32, width: f32) -> String {
    if painter.measure(text, size) <= width {
        return text.into();
    }
    let boundaries: Vec<_> = text
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .collect();
    let (mut left, mut right) = (0, boundaries.len().saturating_sub(1));
    while left < right {
        let middle = (left + right).div_ceil(2);
        let candidate = format!("{}...", &text[..boundaries[middle]]);
        if painter.measure(&candidate, size) <= width {
            left = middle;
        } else {
            right = middle - 1;
        }
    }
    format!("{}...", &text[..boundaries[left]])
}
