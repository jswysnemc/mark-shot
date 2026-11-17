//! 【滚动截图】【面板帧】平台后端接收局部控制像素与独立选区外框参数，不依赖整屏编辑画布

use crate::renderer::Painter;
use tiny_skia::{Color, IntRect, Pixmap, Rect};

pub struct ScrollPanelFrame<'a> {
    pub pixels: &'a [u8],
    pub size: (u32, u32),
    pub bounds: IntRect,
    pub region: Rect,
    pub scale: f32,
    pub border: Color,
}

/// 【滚动截图】【选区外框边界】按 region 返回位于捕获区域外侧五逻辑像素的四条窄边矩形，避免污染捕获原图
pub fn edge_bounds(region: Rect) -> [Rect; 4] {
    let outer = region.outset(5.0, 5.0).unwrap_or(region);
    let left = outer.left() - 2.0;
    let top = outer.top() - 2.0;
    [
        Rect::from_xywh(left, top, outer.width() + 4.0, 4.0).unwrap(),
        Rect::from_xywh(left, outer.bottom() - 2.0, outer.width() + 4.0, 4.0).unwrap(),
        Rect::from_xywh(left, top + 4.0, 4.0, (outer.height() - 4.0).max(1.0)).unwrap(),
        Rect::from_xywh(
            outer.right() - 2.0,
            top + 4.0,
            4.0,
            (outer.height() - 4.0).max(1.0),
        )
        .unwrap(),
    ]
}

/// 【滚动截图】【边线局部光栅】按 bounds、region、scale 与 color 绘制窄缓冲选区描边，返回单条边像素
pub fn edge_pixels(bounds: IntRect, region: Rect, scale: f32, color: Color) -> Option<Pixmap> {
    let size = crate::types::px_size((bounds.width(), bounds.height()), scale);
    let mut pixels = Pixmap::new(size.0, size.1)?;
    Painter::new(&mut pixels)
        .with_scale(scale)
        .with_origin((bounds.x() as f32, bounds.y() as f32))
        .stroke_path(
            &tiny_skia::PathBuilder::from_rect(region.outset(5.0, 5.0)?),
            color,
            &tiny_skia::Stroke {
                width: 2.0,
                ..Default::default()
            },
        );
    Some(pixels)
}
