//! 【滚动截图】【预览导航】按原图坐标浏览细节，概览标记与拖动共用同一映射

use crate::renderer::Painter;
use shot_stitch::Axis;
use tiny_skia::{Pixmap, PremultipliedColorU8, Rect};

#[derive(Default)]
pub struct PreviewState {
    pub scrub: f32,
    pub following: bool,
}

pub struct PreviewLayout {
    pub detail: Rect,
    pub overview: Option<Rect>,
    pub scale: f32,
    pub viewport: f32,
    pub max_scrub: f32,
    pub horizontal: bool,
}

impl PreviewState {
    /// 【滚动截图】【跟随构造】返回初始跟随最新内容的浏览状态，无参数
    pub fn new() -> Self {
        Self {
            scrub: 0.0,
            following: true,
        }
    }

    /// 【滚动截图】【内容同步】按 layout 限制 scrub，跟随状态始终定位到末端，无返回值
    pub fn sync(&mut self, layout: &PreviewLayout) {
        self.scrub = if self.following {
            layout.max_scrub
        } else {
            self.scrub.clamp(0.0, layout.max_scrub)
        };
    }

    /// 【滚动截图】【浏览定位】按 value 与 layout 更新原图坐标，返回是否变化，末端恢复跟随
    pub fn set(&mut self, value: f32, layout: &PreviewLayout) -> bool {
        let before = self.scrub;
        self.scrub = value.clamp(0.0, layout.max_scrub);
        self.following = self.scrub >= layout.max_scrub;
        self.scrub != before
    }
}

/// 【滚动截图】【细节布局】按 area、dimensions 与 axis 返回原版八二分栏或分行及可浏览长度
pub fn layout(area: Rect, dimensions: (i32, i32), axis: Axis) -> Option<PreviewLayout> {
    let (w, h) = dimensions;
    if w <= 0 || h <= 0 {
        return None;
    }
    let horizontal = axis == Axis::Horizontal;
    let overflow = if horizontal {
        w as f32 * area.height() / h as f32 > area.width()
    } else {
        h as f32 * area.width() / w as f32 > area.height()
    };
    let (detail, overview) = if !overflow {
        (area, None)
    } else if horizontal {
        let height = (area.height() - 10.0).max(1.0) * 0.8;
        (
            Rect::from_xywh(area.left(), area.top(), area.width(), height)?,
            Rect::from_xywh(
                area.left(),
                area.top() + height + 10.0,
                area.width(),
                (area.height() - height - 10.0).max(1.0),
            ),
        )
    } else {
        let width = (area.width() - 10.0).max(1.0) * 0.8;
        (
            Rect::from_xywh(area.left(), area.top(), width, area.height())?,
            Rect::from_xywh(
                area.left() + width + 10.0,
                area.top(),
                (area.width() - width - 10.0).max(1.0),
                area.height(),
            ),
        )
    };
    let scale = if horizontal {
        detail.height() / h as f32
    } else {
        detail.width() / w as f32
    };
    let length = if horizontal { w } else { h } as f32;
    let viewport = (if horizontal {
        detail.width()
    } else {
        detail.height()
    } / scale)
        .min(length);
    Some(PreviewLayout {
        detail,
        overview,
        scale,
        viewport,
        max_scrub: (length - viewport).max(0.0),
        horizontal,
    })
}

/// 【滚动截图】【概览矩形】按 layout 和 dimensions 等比适配完整长图，返回实际图像范围
pub fn overview_rect(layout: &PreviewLayout, dimensions: (i32, i32)) -> Option<Rect> {
    let area = layout.overview?;
    let (w, h) = (dimensions.0 as f32, dimensions.1 as f32);
    let factor = (area.width() / w).min(area.height() / h);
    Rect::from_xywh(
        area.left() + (area.width() - w * factor) / 2.0,
        area.top() + (area.height() - h * factor) / 2.0,
        w * factor,
        h * factor,
    )
}

/// 【滚动截图】【概览定位】将 position 映射到 layout 原图长度，offset 保留按下时相对可视窗口的位置，返回目标浏览坐标
pub fn scrub_at(position: (f64, f64), rect: Rect, layout: &PreviewLayout, offset: f32) -> f32 {
    let fraction = if layout.horizontal {
        (position.0 as f32 - rect.left()) / rect.width()
    } else {
        (position.1 as f32 - rect.top()) / rect.height()
    };
    fraction * (layout.max_scrub + layout.viewport) - offset
}

/// 【滚动截图】【预览绘制】按 image、layout 与 state 只分配可见像素，使用 p 的设备比例，不复制整幅长图
pub fn draw(
    p: &mut Painter,
    image: shot_stitch::StitchedView<'_>,
    layout: &PreviewLayout,
    state: &mut PreviewState,
    capture_range: Option<(i32, i32)>,
) {
    state.sync(layout);
    let origin = if layout.horizontal {
        (state.scrub, 0.0)
    } else {
        (0.0, state.scrub)
    };
    draw_image(p, image, layout.detail, layout.scale, origin);
    if let Some(rect) = overview_rect(layout, (image.width(), image.height())) {
        draw_image(
            p,
            image,
            rect,
            rect.width() / image.width() as f32,
            (0.0, 0.0),
        );
        let length = layout.max_scrub + layout.viewport;
        if let Some(range) = capture_range {
            super::capture_marker::draw(p, rect, length, layout.horizontal, range);
        }
        let marker = if layout.horizontal {
            Rect::from_xywh(
                rect.left() + rect.width() * state.scrub / length,
                rect.top(),
                rect.width() * layout.viewport / length,
                rect.height(),
            )
        } else {
            Rect::from_xywh(
                rect.left(),
                rect.top() + rect.height() * state.scrub / length,
                rect.width(),
                rect.height() * layout.viewport / length,
            )
        };
        if let Some(marker) = marker {
            p.fill_rounded(marker, 0.0, crate::theme::Rgba(45, 212, 191, 45));
            p.stroke_path(
                &tiny_skia::PathBuilder::from_rect(marker),
                crate::theme::Rgba(45, 212, 191, 235),
                &tiny_skia::Stroke {
                    width: 2.0,
                    ..Default::default()
                },
            );
        }
    }
}

/// 【滚动截图】【局部光栅】按 rect、factor 与 origin 从 image 采样可见区域并绘制到 p，无返回值
fn draw_image(
    p: &mut Painter,
    image: shot_stitch::StitchedView<'_>,
    rect: Rect,
    factor: f32,
    origin: (f32, f32),
) {
    let scale = p.scale();
    let width = (rect.width() * scale).round().max(1.0) as u32;
    let height = (rect.height() * scale).round().max(1.0) as u32;
    let Some(mut pixels) = Pixmap::new(width, height) else {
        return;
    };
    for y in 0..height {
        let sy = (origin.1 + y as f32 / (factor * scale)) as i32;
        if sy < 0 || sy >= image.height() {
            continue;
        }
        for x in 0..width {
            let sx = (origin.0 + x as f32 / (factor * scale)) as i32;
            if sx < 0 || sx >= image.width() {
                continue;
            }
            let value = image.pixel(sx, sy);
            pixels.pixels_mut()[(y * width + x) as usize] = PremultipliedColorU8::from_rgba(
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
                255,
            )
            .unwrap();
        }
    }
    let (x, y) = p.to_px(rect.left(), rect.top());
    p.draw_pixmap(x.round() as i32, y.round() as i32, &pixels);
}
