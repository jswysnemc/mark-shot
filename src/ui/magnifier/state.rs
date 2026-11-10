use tiny_skia::Color;

/// 【光标放大区】【像素倍率】无参数，按实际圆镜直径与采样格数返回倍率，兼容旧配置
pub fn zoom() -> f32 {
    size() as f32 / cells() as f32
}
/// 【光标放大区】【采样格数】无参数，返回配置中的采样格数
pub fn cells() -> u32 {
    crate::config::get().magnifier.cells
}
/// 【光标放大区】【紧凑直径】无参数，将旧配置的二百一十像素圆镜限制到一百零五像素
pub fn size() -> u32 {
    crate::config::get().magnifier.size().min(105)
}
/// 【光标放大区】【指针间距】无参数，返回圆镜与指针的间距
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
