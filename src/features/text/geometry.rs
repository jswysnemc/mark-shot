//! 【文字标注】【字号与框】复用原版点字号取整、对称内边距和背景框收紧规则

use tiny_skia::Rect;
pub const MIN_POINTS: f32 = 8.0;
pub const MAX_POINTS: f32 = 300.0;
pub const DEFAULT_POINTS: f32 = 20.0;
pub const POINT_OFFSET: f32 = 19.0;

/// 【文字标注】【点字号】规范 points 到原版八至三百点整数范围；非有限值返回默认二十点
pub fn points(points: f32) -> f32 {
    if points.is_finite() {
        points.round().clamp(MIN_POINTS, MAX_POINTS)
    } else {
        DEFAULT_POINTS
    }
}

/// 【文字标注】【渲染字号】按 points 和 size_scale 先取整点字号，再按九十六 DPI 转换并取整像素，与原版字体引擎一致
pub fn pixels(points: f32, size_scale: f32) -> f32 {
    ((points * size_scale).round().max(1.0) * 96.0 / 72.0).round()
}

/// 【文字标注】【内边距】返回 size_scale 对应左右、上下内边距；透明背景同样保留布局内边距
pub fn padding(size_scale: f32) -> (f32, f32) {
    (6.0 * size_scale, 4.0 * size_scale)
}

/// 【文字标注】【背景边界】根据 start、文字 width/height 和 size_scale 返回向上取整的对称背景框
pub fn bounds(start: (f32, f32), width: f32, height: f32, size_scale: f32) -> Rect {
    let pad = padding(size_scale);
    Rect::from_xywh(
        start.0,
        start.1,
        (width + pad.0 * 2.0).ceil().max(1.0),
        (height + pad.1 * 2.0).ceil().max(1.0),
    )
    .unwrap()
}
