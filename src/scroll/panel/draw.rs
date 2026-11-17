//! 【滚动截图】【极简面板绘制】去除全部面板与预览描边，保留语义状态头、拼接增量、主副双预览与轻量图标栏

use super::{
    PanelButton, PanelLayout,
    preview::{self, PreviewState},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    scroll::session::ScrollSession,
    theme::{Rgba, toolbar::Palette},
    ui::{controls, icons},
};
use cosmic_text::Weight;
use shot_stitch::Axis;

const CARD_RADIUS: f32 = 11.0;
const BUTTON_RADIUS: f32 = 8.5;

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

/// 【滚动截图】【拼接数值】按 session 当前总长度与最近新增像素生成读数文本，返回格式化字符串
pub fn metric_text(session: &ScrollSession) -> String {
    let base = session.length_label();
    if base.is_empty() {
        return base;
    }
    if session.frame_count > 1 && session.last_append() > 0 {
        format!("{base} +{}", session.last_append())
    } else {
        base
    }
}

/// 【滚动截图】【状态指示色】按 session 当前捕获、暂停或告警状态返回极简圆点颜色
fn status_dot_color(session: &ScrollSession, palette: &Palette) -> Rgba {
    if session.status == "Capture error" {
        Rgba(239, 68, 68, 255)
    } else if session.paused || session.status.starts_with("No overlap") {
        Rgba(245, 158, 11, 255)
    } else {
        palette.slider_end
    }
}

/// 【滚动截图】【完整面板】将 session 和 state 按无边框极简 layout 绘制到 p，hovered 与 pressed 表达按钮事务，无返回值
pub fn draw(
    p: &mut Painter,
    session: &mut ScrollSession,
    layout: &PanelLayout,
    hovered: Option<PanelButton>,
    pressed: Option<PanelButton>,
    state: &mut PreviewState,
) {
    let palette = Palette::current();
    // 1. 绘制无描边极简不透明圆角底板，保障无层协议下平色定位锚点稳定
    p.fill_rounded(layout.rect, CARD_RADIUS, palette.list_fill);

    // 2. 绘制顶部语义圆点、自适应截断状态文本与右对齐总长度及 +N 拼接增量读数
    let dot_cy = layout.status.top() + layout.status.height() * 0.5;
    p.fill_circle(
        layout.status.left() + 4.0,
        dot_cy,
        2.75,
        status_dot_color(session, &palette),
    );
    let raw_metric = metric_text(session);
    let max_metric_w = (layout.status.width() * 0.52).max(1.0);
    let metric = if raw_metric.is_empty() {
        raw_metric
    } else {
        controls::fit_label(p, &raw_metric, 11.5, max_metric_w)
    };
    let metric_text_w = if metric.is_empty() {
        0.0
    } else {
        p.measure(&metric, 11.5)
    };
    let metric_width = if metric_text_w > 0.0 {
        (metric_text_w + 8.0).min(max_metric_w + 6.0)
    } else {
        0.0
    };
    if let Some(status_rect) = tiny_skia::Rect::from_xywh(
        layout.status.left() + 13.0,
        layout.status.top(),
        (layout.status.width() - metric_width - 15.0).max(1.0),
        layout.status.height(),
    ) {
        let fitted =
            controls::fit_label(p, status_text(&session.status), 11.5, status_rect.width());
        p.text(
            status_rect,
            &fitted,
            TextStyle::new(11.5, palette.strong_text).weight(Weight::MEDIUM),
        );
    }
    if !metric.is_empty() {
        let metric_w = metric_text_w.ceil().max(1.0);
        if let Some(metric_rect) = tiny_skia::Rect::from_xywh(
            (layout.status.right() - metric_w).max(layout.status.left()),
            layout.status.top(),
            metric_w,
            layout.status.height(),
        ) {
            p.text(
                metric_rect,
                &metric,
                TextStyle::new(11.5, palette.muted_text),
            );
        }
    }

    // 3. 绘制无边框主细节预览与副全景概览（未入帧时保持双槽极简占位，避免跳变）
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

    // 4. 绘制底部无边框轻量操作胶囊按钮，保存按钮维持实心主色以兼顾平色识别样本
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

        let size = (rect.width() * 0.5).clamp(13.0, 19.0);
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
