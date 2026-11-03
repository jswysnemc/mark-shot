//! 【界面控件】【基础按钮】独立窗口共用紧凑按钮与文字截断规则

use tiny_skia::Rect;

use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{color, font, radius},
};

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
