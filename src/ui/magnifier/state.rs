use tiny_skia::Color;

// honestly my magnifier implementation sucks a bit
// im not sure how to properly change these constants without making it look crooked
pub fn zoom() -> f32 {
    crate::config::get().magnifier.zoom
}
pub fn cells() -> u32 {
    crate::config::get().magnifier.cells
}
pub fn size() -> u32 {
    crate::config::get().magnifier.size()
}
pub fn offset() -> f32 {
    crate::config::get().magnifier.offset
}

#[derive(Debug, Clone, Copy)]
pub struct MagnifierState {
    pub monitor_idx: usize,
    pub pos: (f64, f64),
}

/// 【像素采样】【指针颜色】按source与point读取实际原件颜色，返回不透明显示颜色，原件外返回空值，不复制或修改原件
pub fn sample_pixel(
    source: &impl crate::image_source::PixelView,
    point: (f64, f64),
) -> Option<Color> {
    let source = source.pixels_view();
    let (x, y) = (point.0.floor(), point.1.floor());
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let (x, y) = (x as u32, y as u32);
    if x >= source.width() || y >= source.height() {
        return None;
    }
    let px = source.pixels()[(y * source.width() + x) as usize].demultiply();
    Some(Color::from_rgba8(px.red(), px.green(), px.blue(), 255))
}
