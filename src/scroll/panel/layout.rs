//! 【滚动截图】【面板布局】固定控制尺寸并优先避让选区，空间不足时明确报告重叠

use super::PanelButton;
use tiny_skia::Rect;

pub struct PanelLayout {
    pub rect: Rect,
    pub preview: Option<Rect>,
    pub status: Rect,
    pub buttons: Vec<(PanelButton, Rect)>,
    pub fits_without_overlap: bool,
}

pub const PANEL_WIDTH: f32 = 340.0;
const PADDING: f32 = 12.0;
const GAP: f32 = 12.0;
const STATUS_HEIGHT: f32 = 22.0;
const CONTROL_HEIGHT: f32 = 54.0;
const BUTTON_WIDTH: f32 = 40.0;
const BUTTON_HEIGHT: f32 = 36.0;
const SHEET: [PanelButton; 6] = [
    PanelButton::ToggleAxis,
    PanelButton::TogglePause,
    PanelButton::Annotate,
    PanelButton::Save,
    PanelButton::Copy,
    PanelButton::Cancel,
];

/// 【滚动截图】【面板定位】按 local_region、monitor_size 和 preview_size 返回布局，使用左右上下四处候选避让
pub fn layout(
    local_region: Rect,
    monitor_size: (f32, f32),
    preview_size: Option<(u32, u32)>,
) -> PanelLayout {
    constrained_layout(local_region, monitor_size, preview_size, None)
}

/// 【滚动截图】【受限布局】按原输出定位 local_region，limits 仅限制控制区域尺寸，preview_size 保持原图比例，返回绘制与输入共用布局
pub fn constrained_layout(
    local_region: Rect,
    monitor_size: (f32, f32),
    preview_size: Option<(u32, u32)>,
    limits: Option<(u32, u32)>,
) -> PanelLayout {
    let (mon_w, mon_h) = monitor_size;
    let available_width = limits
        .map_or(mon_w, |(width, _)| mon_w.min(width as f32))
        .max(1.0);
    let available_height = limits
        .map_or(mon_h, |(_, height)| mon_h.min(height as f32))
        .max(1.0);
    let width = PANEL_WIDTH.min(available_width);
    let chrome = STATUS_HEIGHT + CONTROL_HEIGHT + PADDING * 4.0;
    let image_height = preview_size
        .map_or(96.0, |(w, h)| {
            (h as f32 * (width - PADDING * 2.0) / w.max(1) as f32).clamp(96.0, 420.0)
        })
        .min((available_height - chrome - GAP * 2.0).max(1.0));
    let height = (chrome + image_height).min(available_height);
    let clamp = |x: f32, y: f32| {
        Rect::from_xywh(
            x.clamp(0.0, (mon_w - width).max(0.0)),
            y.clamp(0.0, (mon_h - height).max(0.0)),
            width,
            height,
        )
        .unwrap()
    };
    // 1. 【滚动截图】【捕获避让】仅接受完整位于输出内且不与选区相交的候选
    let candidates = [
        (local_region.right() + GAP, local_region.top()),
        (local_region.left() - GAP - width, local_region.top()),
        (local_region.left(), local_region.bottom() + GAP),
        (local_region.left(), local_region.top() - GAP - height),
    ];
    let safe = candidates
        .into_iter()
        .map(|(x, y)| clamp(x, y))
        .find(|rect| rect.intersect(&local_region).is_none());
    let fits_without_overlap = safe.is_some();
    let rect = safe.unwrap_or_else(|| clamp(local_region.right() + GAP, local_region.top()));
    let inner_width = (width - PADDING * 2.0).max(1.0);
    let status = Rect::from_xywh(
        rect.left() + PADDING,
        rect.top() + PADDING,
        inner_width,
        STATUS_HEIGHT,
    )
    .unwrap();
    let preview = Rect::from_xywh(
        rect.left() + PADDING,
        status.bottom() + PADDING,
        inner_width,
        image_height,
    );
    let row_y = rect.bottom() - PADDING - CONTROL_HEIGHT;
    // 2. 【滚动截图】【紧凑控件】普通输出保持原版四十乘三十六尺寸，极窄输出按可用宽度缩小
    let row_scale = (inner_width / 316.0).min(1.0);
    let mut x = rect.left() + PADDING + 7.0 * row_scale;
    let buttons = SHEET
        .into_iter()
        .map(|button| {
            let button_rect = Rect::from_xywh(
                x,
                row_y + 9.0,
                BUTTON_WIDTH * row_scale,
                BUTTON_HEIGHT.min((height - 18.0).max(1.0)),
            )
            .unwrap();
            x += (BUTTON_WIDTH
                + 7.0
                + if button == PanelButton::ToggleAxis {
                    4.0
                } else {
                    0.0
                })
                * row_scale;
            (button, button_rect)
        })
        .collect();
    PanelLayout {
        rect,
        preview,
        status,
        buttons,
        fits_without_overlap,
    }
}

/// 【滚动截图】【按钮命中】按 layout 和 local 返回当前鼠标所指按钮，按钮外返回空值
pub fn hit_test(layout: &PanelLayout, local: (f64, f64)) -> Option<PanelButton> {
    layout
        .buttons
        .iter()
        .find(|(_, rect)| super::contains(*rect, local))
        .map(|(button, _)| *button)
}
