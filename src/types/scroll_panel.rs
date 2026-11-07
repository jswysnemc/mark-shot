//! 【滚动截图】【面板帧】平台后端接收局部控制像素和独立原图区域，不依赖整屏编辑画布

pub struct ScrollPanelFrame<'a> {
    pub pixels: &'a [u8],
    pub size: (u32, u32),
    pub bounds: tiny_skia::IntRect,
    pub region: tiny_skia::Rect,
    pub scale: f32,
    pub border: tiny_skia::Color,
}
