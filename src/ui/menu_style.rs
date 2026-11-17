//! 【下拉与菜单】【统一样式】统一应用内全部下拉框与弹出菜单的悬停胶囊、选中态及行间留白

use crate::{
    renderer::{Painter, paths::rounded_rect_path},
    theme::{Mode, Rgba},
};
use tiny_skia::{Rect, Stroke};

pub const POPUP_RADIUS: f32 = 9.5;
pub const ITEM_RADIUS: f32 = 6.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuStyle {
    pub popup_fill: Rgba,
    pub popup_border: Rgba,
    pub popup_shadow: Rgba,
    pub text: Rgba,
    pub muted_text: Rgba,
    pub disabled_text: Rgba,
    pub accent: Rgba,
    pub selected_fill: Rgba,
    pub selected_border: Rgba,
    pub selected_text: Rgba,
    pub hover_fill: Rgba,
    pub hover_border: Rgba,
    pub hover_text: Rgba,
    pub selected_hover_fill: Rgba,
    pub selected_hover_border: Rgba,
    pub danger_text: Rgba,
    pub danger_hover_fill: Rgba,
    pub danger_hover_border: Rgba,
    pub danger_hover_text: Rgba,
    pub separator: Rgba,
}

impl MenuStyle {
    /// 【下拉与菜单】【当前主题】无参数，按当前解析后的明暗模式返回统一菜单配色令牌
    pub fn current() -> Self {
        Self::new(Mode::current())
    }

    /// 【下拉与菜单】【主题令牌】按 mode 生成选中态与悬停态明确分离的配色表，返回 MenuStyle
    pub fn new(mode: Mode) -> Self {
        match mode.resolved() {
            Mode::Dark => Self {
                popup_fill: Rgba(18, 22, 31, 248),
                popup_border: Rgba(255, 255, 255, 26),
                popup_shadow: Rgba(2, 6, 23, 88),
                text: Rgba(226, 232, 240, 255),
                muted_text: Rgba(148, 163, 184, 255),
                disabled_text: Rgba(100, 116, 139, 255),
                accent: Rgba(94, 234, 212, 255),
                selected_fill: Rgba(45, 212, 191, 34),
                selected_border: Rgba(45, 212, 191, 86),
                selected_text: Rgba(94, 234, 212, 255),
                hover_fill: Rgba(148, 163, 184, 34),
                hover_border: Rgba(255, 255, 255, 24),
                hover_text: Rgba(248, 250, 252, 255),
                selected_hover_fill: Rgba(45, 212, 191, 56),
                selected_hover_border: Rgba(94, 234, 212, 132),
                danger_text: Rgba(248, 113, 113, 255),
                danger_hover_fill: Rgba(239, 68, 68, 52),
                danger_hover_border: Rgba(248, 113, 113, 112),
                danger_hover_text: Rgba(252, 165, 165, 255),
                separator: Rgba(255, 255, 255, 22),
            },
            _ => Self {
                popup_fill: Rgba(255, 255, 255, 252),
                popup_border: Rgba(15, 23, 42, 28),
                popup_shadow: Rgba(15, 23, 42, 18),
                text: Rgba(30, 41, 59, 255),
                muted_text: Rgba(100, 116, 139, 255),
                disabled_text: Rgba(148, 163, 184, 255),
                accent: Rgba(13, 148, 136, 255),
                selected_fill: Rgba(13, 148, 136, 24),
                selected_border: Rgba(13, 148, 136, 76),
                selected_text: Rgba(15, 118, 110, 255),
                hover_fill: Rgba(15, 23, 42, 15),
                hover_border: Rgba(15, 23, 42, 24),
                hover_text: Rgba(15, 23, 42, 255),
                selected_hover_fill: Rgba(13, 148, 136, 42),
                selected_hover_border: Rgba(13, 148, 136, 118),
                danger_text: Rgba(220, 38, 38, 255),
                danger_hover_fill: Rgba(239, 68, 68, 26),
                danger_hover_border: Rgba(220, 38, 38, 84),
                danger_hover_text: Rgba(185, 28, 28, 255),
                separator: Rgba(15, 23, 42, 20),
            },
        }
    }
}

/// 【下拉与菜单】【胶囊内缩】按 row 与 v_inset 返回上下留出清晰空隙的视觉胶囊矩形，保持点击热区不变
pub fn pill_rect(row: Rect, v_inset: f32) -> Rect {
    let inset = v_inset.clamp(0.0, (row.height() - 2.0).max(0.0) * 0.5);
    Rect::from_xywh(
        row.left() + 0.5,
        row.top() + inset,
        (row.width() - 1.0).max(1.0),
        (row.height() - inset * 2.0).max(1.0),
    )
    .unwrap_or(row)
}

/// 【下拉与菜单】【细边描边】在 painter 中对 rect 绘制 radius 圆角的一像素边框，无返回值
pub fn stroke_rounded(painter: &mut Painter<'_>, rect: Rect, radius: f32, color: Rgba) {
    if let Some(path) = rounded_rect_path(&rect, radius, true, true, true, true) {
        painter.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
        );
    }
}

/// 【下拉与菜单】【浮层外框】在 bounds 内绘制统一的圆角浮层卡片与一像素外描边，无返回值
pub fn draw_popup_frame(painter: &mut Painter<'_>, bounds: Rect, radius: f32) {
    let style = MenuStyle::current();
    let Some(frame) = Rect::from_xywh(
        bounds.left() + 0.5,
        bounds.top() + 0.5,
        (bounds.width() - 1.0).max(1.0),
        (bounds.height() - 1.0).max(1.0),
    ) else {
        return;
    };
    painter.fill_rounded(frame, radius, style.popup_fill);
    stroke_rounded(painter, frame, radius, style.popup_border);
}

/// 【下拉与菜单】【条目胶囊】按 selected、hovered 与 danger 绘制带上下空隙的条目背景，返回对应前景文字颜色
pub fn draw_row_pill(
    painter: &mut Painter<'_>,
    row: Rect,
    v_inset: f32,
    radius: f32,
    selected: bool,
    hovered: bool,
    danger: bool,
) -> Rgba {
    let style = MenuStyle::current();
    let pill = pill_rect(row, v_inset);
    // 1. 危险项悬停、已选叠加悬停、纯已选与纯悬停四态分离，避免相邻两项同色粘连
    if danger && hovered {
        painter.fill_rounded(pill, radius, style.danger_hover_fill);
        stroke_rounded(painter, pill, radius, style.danger_hover_border);
        style.danger_hover_text
    } else if danger {
        style.danger_text
    } else if selected && hovered {
        painter.fill_rounded(pill, radius, style.selected_hover_fill);
        stroke_rounded(painter, pill, radius, style.selected_hover_border);
        style.selected_text
    } else if selected {
        painter.fill_rounded(pill, radius, style.selected_fill);
        stroke_rounded(painter, pill, radius, style.selected_border);
        style.selected_text
    } else if hovered {
        painter.fill_rounded(pill, radius, style.hover_fill);
        stroke_rounded(painter, pill, radius, style.hover_border);
        style.hover_text
    } else {
        style.text
    }
}
