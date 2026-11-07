//! 【滚动截图】【面板绘制】原版状态行和六按钮使用共用主题令牌

use super::{
    PanelButton, PanelLayout,
    preview::{self, PreviewState},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    scroll::session::ScrollSession,
    theme::toolbar::Palette,
    ui::icons,
};
use shot_stitch::Axis;

/// 【滚动截图】【状态文字】按 status 返回当前界面语言文案，无其他参数
pub fn status_text(status: &str) -> &str {
    if crate::ui::language::Language::current() != crate::ui::language::Language::Chinese {
        return status;
    }
    match status {
        "Waiting for scroll" => "等待滚动",
        "Capturing" => "正在捕获",
        "Paused" => "已暂停",
        "Capture paused" => "捕获已暂停",
        "Capture error" => "捕获错误",
        "No overlap match" => "未找到重叠区域",
        "Dragging region" => "正在移动区域",
        "Region adjusted" => "区域已调整",
        _ => status,
    }
}

/// 【滚动截图】【完整面板】将 session 和 state 按 layout 绘制到 p，hovered 与 pressed 表达按钮事务，无返回值
pub fn draw(
    p: &mut Painter,
    session: &mut ScrollSession,
    layout: &PanelLayout,
    hovered: Option<PanelButton>,
    pressed: Option<PanelButton>,
    state: &mut PreviewState,
) {
    let palette = Palette::current();
    p.fill_rounded(layout.rect, 10.0, palette.list_fill);
    p.panel_border(
        layout.rect.left(),
        layout.rect.top(),
        layout.rect.width(),
        layout.rect.height(),
        10.0,
        1.0,
    );
    p.fill_circle(
        layout.status.left() + 4.0,
        layout.status.top() + 11.5,
        3.0,
        palette.slider_end,
    );
    let metric = session.length_label();
    let metric_width = p.measure(&metric, 12.0) + 10.0;
    let status_rect = tiny_skia::Rect::from_xywh(
        layout.status.left() + 15.0,
        layout.status.top(),
        (layout.status.width() - metric_width - 15.0).max(1.0),
        layout.status.height(),
    )
    .unwrap();
    let label = status_text(&session.status);
    p.text(
        status_rect,
        label,
        TextStyle::new(12.0, palette.strong_text),
    );
    let metric_rect = tiny_skia::Rect::from_xywh(
        layout.status.right() - metric_width,
        layout.status.top(),
        metric_width,
        layout.status.height(),
    )
    .unwrap();
    p.text(
        metric_rect,
        &metric,
        TextStyle::new(12.0, palette.muted_text),
    );
    if let Some(area) = layout.preview {
        p.fill_rounded(area, 0.0, palette.list_fill);
        if let Some(view) = preview::layout(area, session.stitcher.dimensions(), session.axis())
            && let Some(image) = session.source()
        {
            preview::draw(p, image, &view, state);
        }
        super::border(p, area, palette.separator);
    }
    for (button, rect) in &layout.buttons {
        let enabled = is_button_enabled(*button, session);
        let active = enabled && hovered == Some(*button);
        let tint = if !enabled {
            palette.muted_text.with_alpha(90)
        } else if *button == PanelButton::Save {
            palette.primary_text
        } else if active && *button == PanelButton::Cancel {
            palette.danger_hover_text
        } else {
            palette.text
        };
        if *button == PanelButton::Save {
            p.fill_rounded(
                *rect,
                9.0,
                if active {
                    palette.primary_hover_top
                } else {
                    palette.primary_top
                },
            );
        } else if active {
            p.fill_rounded(
                *rect,
                9.0,
                if pressed == Some(*button) {
                    palette.pressed_fill
                } else if *button == PanelButton::Cancel {
                    palette.danger_hover_fill
                } else {
                    palette.soft_hover_fill
                },
            );
        }
        let size = rect.width().min(22.0);
        p.icon(
            button_icon(*button, session),
            size,
            rect.left() + (rect.width() - size) / 2.0,
            rect.top() + (rect.height() - size) / 2.0,
            tint,
        );
    }
}

/// 【滚动截图】【按钮启用】按 button 和 session 返回操作可用状态，结果操作需要至少一帧
pub fn is_button_enabled(button: PanelButton, session: &ScrollSession) -> bool {
    match button {
        PanelButton::Annotate | PanelButton::Save | PanelButton::Copy => session.frame_count > 0,
        PanelButton::ToggleAxis | PanelButton::TogglePause | PanelButton::Cancel => true,
    }
}

/// 【滚动截图】【按钮图标】按 button 和 session 返回项目内 SVG 图标，无分配
pub fn button_icon(button: PanelButton, session: &ScrollSession) -> &'static str {
    match button {
        PanelButton::ToggleAxis => {
            if session.axis() == Axis::Vertical {
                icons::SCROLL
            } else {
                icons::SCROLL_HORIZONTAL
            }
        }
        PanelButton::TogglePause => {
            if session.paused {
                icons::SCROLL_RESUME
            } else {
                icons::SCROLL_PAUSE
            }
        }
        PanelButton::Annotate => icons::PEN,
        PanelButton::Save => icons::SAVE,
        PanelButton::Copy => icons::COPY,
        PanelButton::Cancel => icons::CLOSE,
    }
}
