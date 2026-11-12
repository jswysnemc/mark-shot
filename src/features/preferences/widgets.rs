//! 【设置窗口】【统一控件】面板、开关与输入框共享克制的明暗视觉

use super::layout::rect;
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use tiny_skia::Rect;

/// 【设置窗口】【紧凑数值】按 bounds、label、focus 和边界绘制统一减号、输入、加号控件
pub(super) fn number(
    painter: &mut Painter<'_>,
    bounds: Rect,
    label: &str,
    focus: bool,
    limits: (bool, bool),
    hover: i8,
) {
    let palette = Palette::current();
    controls::frame(
        painter,
        bounds,
        palette.base,
        if focus {
            palette.accent
        } else {
            palette.border
        },
        6.0,
    );
    for (direction, enabled) in [(-1, limits.0), (1, limits.1)] {
        let button = super::layout::step_rect(bounds, direction);
        if hover == direction && enabled {
            painter.fill_rounded(
                rect(
                    button.left() + 2.0,
                    button.top() + 2.0,
                    24.0,
                    button.height() - 4.0,
                ),
                4.0,
                palette.window,
            );
        }
        let color = if enabled { palette.text } else { palette.muted };
        painter.fill_rounded(
            rect(
                button.left() + 10.0,
                button.top() + button.height() / 2.0 - 0.75,
                8.0,
                1.5,
            ),
            0.75,
            color,
        );
        if direction > 0 {
            painter.fill_rounded(
                rect(
                    button.left() + 13.25,
                    button.top() + button.height() / 2.0 - 4.0,
                    1.5,
                    8.0,
                ),
                0.75,
                color,
            );
        }
        painter.fill_rounded(
            rect(
                if direction < 0 {
                    button.right()
                } else {
                    button.left()
                },
                bounds.top() + 6.0,
                1.0,
                bounds.height() - 12.0,
            ),
            0.0,
            palette.border,
        );
    }
    painter.text(
        rect(
            bounds.left() + 28.0,
            bounds.top(),
            bounds.width() - 56.0,
            bounds.height(),
        ),
        label,
        TextStyle::new(12.0, palette.text).center(),
    );
}

/// 【设置窗口】【设置面板】按 bounds 和 title 绘制圆角面板与分组标题，painter 承担绘制，无返回值
pub(super) fn panel(painter: &mut Painter<'_>, bounds: Rect, title: &str) {
    let palette = Palette::current();
    controls::frame(painter, bounds, palette.base, palette.border, 10.0);
    painter.text(
        rect(
            bounds.left() + 16.0,
            bounds.top() + 4.0,
            bounds.width() - 32.0,
            26.0,
        ),
        title,
        TextStyle::new(12.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
    );
    painter.fill_rounded(
        rect(
            bounds.left() + 1.0,
            bounds.top() + 34.0,
            bounds.width() - 2.0,
            1.0,
        ),
        0.0,
        palette.alternate,
    );
}

/// 【设置窗口】【文字控件】按 bounds、label 和状态绘制统一轻边框按钮，文字超长时截断
pub(super) fn button(
    painter: &mut Painter<'_>,
    bounds: Rect,
    label: &str,
    active: bool,
    hovered: bool,
    enabled: bool,
) {
    let palette = Palette::current();
    controls::frame(
        painter,
        bounds,
        if active {
            palette.pressed
        } else if hovered && enabled {
            palette.window
        } else {
            palette.base
        },
        if active || hovered {
            palette.accent
        } else {
            palette.border
        },
        6.0,
    );
    let label = controls::fit_label(painter, label, 12.0, bounds.width() - 16.0);
    painter.text(
        bounds,
        &label,
        TextStyle::new(
            12.0,
            if !enabled {
                palette.muted
            } else if active {
                palette.accent
            } else {
                palette.text
            },
        )
        .center(),
    );
}

/// 【设置窗口】【输入显示】按 bounds、label、focus 和 choice 绘制文字输入或下拉值，不使用大面积激活底色
pub(super) fn field(
    painter: &mut Painter<'_>,
    bounds: Rect,
    label: &str,
    focus: bool,
    choice: bool,
) {
    let palette = Palette::current();
    controls::frame(
        painter,
        bounds,
        palette.base,
        if focus {
            palette.accent
        } else {
            palette.border
        },
        6.0,
    );
    let label = controls::fit_label(
        painter,
        label,
        12.0,
        bounds.width() - if choice { 34.0 } else { 18.0 },
    );
    painter.text(
        rect(
            bounds.left() + 9.0,
            bounds.top(),
            bounds.width() - 18.0,
            bounds.height(),
        ),
        &label,
        TextStyle::new(12.0, palette.text),
    );
    if choice {
        let mut path = tiny_skia::PathBuilder::new();
        path.move_to(bounds.right() - 20.0, bounds.top() + 13.0);
        path.line_to(bounds.right() - 16.0, bounds.top() + 17.0);
        path.line_to(bounds.right() - 12.0, bounds.top() + 13.0);
        painter.stroke_path(
            &path.finish().unwrap(),
            palette.muted.color(),
            &tiny_skia::Stroke {
                width: 1.4,
                ..Default::default()
            },
        );
    }
}

/// 【设置窗口】【布尔开关】按 bounds、checked 和 hovered 绘制共享开关，点击范围沿用 bounds，无返回值
pub(super) fn switch(painter: &mut Painter<'_>, bounds: Rect, checked: bool, hovered: bool) {
    controls::switch(painter, bounds, checked, hovered);
}

/// 【设置窗口】【下拉入口】按 bounds、label、hovered 和 expanded 绘制选择框，箭头随展开状态变化
pub(super) fn select(
    painter: &mut Painter<'_>,
    bounds: Rect,
    label: &str,
    hovered: bool,
    expanded: bool,
) {
    let palette = Palette::current();
    controls::frame(
        painter,
        bounds,
        palette.base,
        if hovered || expanded {
            palette.accent
        } else {
            palette.border
        },
        6.0,
    );
    let label = controls::fit_label(painter, label, 12.0, bounds.width() - 46.0);
    painter.text(
        rect(
            bounds.left() + 10.0,
            bounds.top(),
            bounds.width() - 44.0,
            bounds.height(),
        ),
        &label,
        TextStyle::new(12.0, palette.text),
    );
    painter.fill_rounded(
        rect(
            bounds.right() - 30.0,
            bounds.top() + 4.0,
            26.0,
            bounds.height() - 8.0,
        ),
        4.0,
        if expanded {
            palette.pressed
        } else {
            palette.window
        },
    );
    let middle = bounds.top() + bounds.height() / 2.0;
    let mut path = tiny_skia::PathBuilder::new();
    path.move_to(
        bounds.right() - 21.0,
        middle + if expanded { 2.0 } else { -2.0 },
    );
    path.line_to(
        bounds.right() - 17.0,
        middle + if expanded { -2.0 } else { 2.0 },
    );
    path.line_to(
        bounds.right() - 13.0,
        middle + if expanded { 2.0 } else { -2.0 },
    );
    painter.stroke_path(
        &path.finish().unwrap(),
        if expanded {
            palette.accent
        } else {
            palette.muted
        }
        .color(),
        &tiny_skia::Stroke {
            width: 1.4,
            ..Default::default()
        },
    );
}
