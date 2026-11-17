//! 【滚动截图】【面板绘制】参考 LumineCapture 呈现顶部主副双预览、三行状态指标与紧凑操作按钮栏

use super::{
    PanelButton, PanelLayout,
    preview::{self, PreviewState},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    scroll::session::ScrollSession,
    theme::{Rgba, radius, toolbar::Palette},
    ui::{controls, icons},
};
use cosmic_text::Weight;
use shot_stitch::Axis;

const BUTTON_RADIUS: f32 = 8.0;
const META_FONT: f32 = 11.5;

/// 【滚动截图】【方向文字】按 axis 返回当前界面语言的滚动方向描述，无其他参数
fn axis_text(axis: Axis) -> &'static str {
    let chinese =
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    match (axis, chinese) {
        (Axis::Vertical, true) => "纵向滚动",
        (Axis::Horizontal, true) => "横向滚动",
        (Axis::Vertical, false) => "Vertical scrolling",
        (Axis::Horizontal, false) => "Horizontal scrolling",
    }
}

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
        "No overlap; scroll back slowly" => "重叠不足，请缓慢回退",
        "Dragging region" => "正在移动区域",
        "Region adjusted" => "区域已调整",
        _ => status,
    }
}

/// 【滚动截图】【状态点配色】按 session 当前捕获、暂停或错误状态返回语义圆点色
fn status_dot_color(session: &ScrollSession, palette: &Palette) -> Rgba {
    if session.status == "Capture error" {
        Rgba(239, 68, 68, 255)
    } else if session.paused || session.status.starts_with("No overlap") {
        Rgba(245, 158, 11, 255)
    } else {
        palette.slider_end
    }
}

/// 【滚动截图】【完整面板】将 session 和 state 按 layout 绘制到 p，hovered 与 pressed 表达按钮交互状态，无返回值
pub fn draw(
    p: &mut Painter,
    session: &mut ScrollSession,
    layout: &PanelLayout,
    hovered: Option<PanelButton>,
    pressed: Option<PanelButton>,
    state: &mut PreviewState,
) {
    let palette = Palette::current();
    let card_radius = radius::panel();
    // 1. 绘制不透明面板圆角底色（保持平色定位采样可靠）
    p.fill_rounded(layout.rect, card_radius, palette.list_fill);

    // 2. 顶部绘制主细节预览与副全景概览，未入帧时保留双槽占位
    if let Some(area) = layout.preview {
        let horizontal = session.axis() == Axis::Horizontal;
        if let Some(view) = preview::layout(area, session.stitcher.dimensions(), session.axis())
            && let Some(image) = session.source()
        {
            preview::draw(p, image, &view, state, session.capture_range);
        } else if let Some((detail, overview)) = preview::split_slots(area, horizontal) {
            preview::draw_tracks(p, detail, overview);
        }
    }

    // 3. 参考 LumineCapture 绘制互不挤压的方向、状态和长度增量三行信息
    let axis = controls::fit_label(p, axis_text(session.axis()), META_FONT, layout.axis.width());
    p.text(
        layout.axis,
        &axis,
        TextStyle::new(META_FONT, palette.strong_text).weight(Weight::MEDIUM),
    );

    let dot_cy = layout.status.top() + layout.status.height() * 0.5;
    p.fill_circle(
        layout.status.left() + 3.5,
        dot_cy,
        2.6,
        status_dot_color(session, &palette),
    );
    if let Some(status_rect) = tiny_skia::Rect::from_xywh(
        layout.status.left() + 11.0,
        layout.status.top(),
        (layout.status.width() - 11.0).max(1.0),
        layout.status.height(),
    ) {
        let status = controls::fit_label(
            p,
            status_text(&session.status),
            META_FONT,
            status_rect.width(),
        );
        p.text(
            status_rect,
            &status,
            TextStyle::new(META_FONT, palette.muted_text),
        );
    }

    let mut length = session.length_label();
    if length.is_empty() {
        length = format!(
            "{} × {} px",
            session.region.width().round() as i32,
            session.region.height().round() as i32
        );
    } else if session.last_append() > 0 {
        length = format!("{length}  +{}", session.last_append());
    }
    let fitted_len = controls::fit_label(p, &length, META_FONT, layout.length.width());
    p.text(
        layout.length,
        &fitted_len,
        TextStyle::new(META_FONT, palette.strong_text),
    );

    // 4. 底部绘制六枚操作按钮并叠加一像素面板细边
    for (button, rect) in &layout.buttons {
        let enabled = is_button_enabled(*button, session);
        let active = enabled && hovered == Some(*button);
        let down = active && pressed == Some(*button);
        let tint = if !enabled {
            if *button == PanelButton::Save {
                palette.primary_text.with_alpha(190)
            } else {
                palette.muted_text.with_alpha(92)
            }
        } else if *button == PanelButton::Save {
            palette.primary_text
        } else if active && *button == PanelButton::Cancel {
            palette.danger_hover_text
        } else if *button == PanelButton::TogglePause && session.paused {
            palette.slider_end
        } else {
            palette.text
        };

        if *button == PanelButton::Save {
            p.fill_rounded(
                *rect,
                BUTTON_RADIUS,
                if active {
                    palette.primary_hover_top
                } else {
                    palette.primary_top
                },
            );
        } else if down {
            p.fill_rounded(*rect, BUTTON_RADIUS, palette.pressed_fill);
        } else if active {
            p.fill_rounded(
                *rect,
                BUTTON_RADIUS,
                if *button == PanelButton::Cancel {
                    palette.danger_hover_fill
                } else {
                    palette.soft_hover_fill
                },
            );
        } else if *button == PanelButton::TogglePause && session.paused {
            p.fill_rounded(*rect, BUTTON_RADIUS, palette.active_fill);
        }

        let size = (rect.width() * 0.5).clamp(13.0, 20.0);
        p.icon(
            button_icon(*button, session),
            size,
            rect.left() + (rect.width() - size) / 2.0,
            rect.top() + (rect.height() - size) / 2.0,
            tint,
        );
    }

    p.panel_border(
        layout.rect.left(),
        layout.rect.top(),
        layout.rect.width(),
        layout.rect.height(),
        card_radius,
        1.0,
    );
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
