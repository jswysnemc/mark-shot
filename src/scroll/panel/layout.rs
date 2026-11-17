//! 【滚动截图】【面板布局】参考 LumineCapture 采用顶部主副双预览、中部三行状态及底部按钮布局

use super::PanelButton;
use tiny_skia::Rect;

pub struct PanelLayout {
    pub rect: Rect,
    pub preview: Option<Rect>,
    pub axis: Rect,
    pub status: Rect,
    pub length: Rect,
    pub buttons: Vec<(PanelButton, Rect)>,
    pub fits_without_overlap: bool,
}

pub const PANEL_WIDTH: f32 = 340.0;
const PADDING: f32 = 12.0;
const GAP: f32 = 12.0;
const SECTION_GAP: f32 = 10.0;
const LINE_HEIGHT: f32 = 17.0;
const META_HEIGHT: f32 = LINE_HEIGHT * 3.0;
const CONTROL_HEIGHT: f32 = 46.0;
const BUTTON_WIDTH: f32 = 40.0;
const BUTTON_HEIGHT: f32 = 34.0;
const PREVIEW_MIN_HEIGHT: f32 = 96.0;
const PREVIEW_MAX_HEIGHT: f32 = 240.0;
const SHEET: [PanelButton; 6] = [
    PanelButton::ToggleAxis,
    PanelButton::TogglePause,
    PanelButton::Annotate,
    PanelButton::Save,
    PanelButton::Copy,
    PanelButton::Cancel,
];

/// 【滚动截图】【面板定位】按 local_region、monitor_size 和 preview_size 返回布局，优先使用四处非重叠候选位
pub fn layout(
    local_region: Rect,
    monitor_size: (f32, f32),
    preview_size: Option<(u32, u32)>,
) -> PanelLayout {
    constrained_layout(local_region, monitor_size, preview_size, None)
}

/// 【滚动截图】【受限布局】按原输出定位 local_region，limits 限制控制区域尺寸，顶部预览、中部三行信息与底部按钮依次排列
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
    let inner_width = (width - PADDING * 2.0).max(1.0);
    let chrome = PADDING * 2.0 + SECTION_GAP * 2.0 + META_HEIGHT + CONTROL_HEIGHT;
    let image_height = preview_size
        .map_or(PREVIEW_MIN_HEIGHT, |(w, h)| {
            (h as f32 * inner_width * 0.78 / w.max(1) as f32)
                .clamp(PREVIEW_MIN_HEIGHT, PREVIEW_MAX_HEIGHT)
        })
        .min((available_height - chrome - GAP).max(28.0));
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

    // 1. 优先选择位于输出内且不与识别区域相交的四向候选位置
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

    // 2. 顶部放置主副双预览区，中部依次放置方向、状态与尺寸增量三行，底部放置操作按钮行
    let row_y = (rect.bottom() - PADDING - CONTROL_HEIGHT).max(rect.top() + PADDING);
    let meta_bottom = (row_y - SECTION_GAP).max(rect.top() + PADDING + 4.0);
    let meta_top = (meta_bottom - META_HEIGHT).max(rect.top() + PADDING + 4.0);
    let preview_h = (meta_top - SECTION_GAP - (rect.top() + PADDING)).max(1.0);
    let preview = Rect::from_xywh(
        rect.left() + PADDING,
        rect.top() + PADDING,
        inner_width,
        preview_h,
    );

    let line_h = ((meta_bottom - meta_top) / 3.0).max(1.0);
    let axis = Rect::from_xywh(rect.left() + PADDING, meta_top, inner_width, line_h).unwrap();
    let status = Rect::from_xywh(
        rect.left() + PADDING,
        meta_top + line_h,
        inner_width,
        line_h,
    )
    .unwrap();
    let length = Rect::from_xywh(
        rect.left() + PADDING,
        meta_top + line_h * 2.0,
        inner_width,
        line_h,
    )
    .unwrap();

    // 3. 保持六按钮水平中心节距稳定，极窄合成器下按比例等比缩放
    let row_scale = (inner_width / 316.0).min(1.0);
    let mut x = rect.left() + PADDING + 7.0 * row_scale;
    let buttons = SHEET
        .into_iter()
        .map(|button| {
            let button_rect = Rect::from_xywh(
                x,
                row_y + 6.0,
                BUTTON_WIDTH * row_scale,
                BUTTON_HEIGHT.min((height - 16.0).max(1.0)),
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
        axis,
        status,
        length,
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
