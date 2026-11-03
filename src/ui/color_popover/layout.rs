//! 【属性取色】【统一几何】坐标来自实际 Qt 控件渲染，绘制与命中不使用旧方形取色配置

use tiny_skia::Rect;
pub const WIDTH: f32 = 200.0;
pub const HEIGHT: f32 = 228.0;
pub const SWATCH_SIZE: f32 = 26.0;
pub const HISTORY_X: [f32; 8] = [8.0, 32.0, 56.0, 79.0, 103.0, 127.0, 150.0, 174.0];

/// 【属性取色】【矩形构造】返回 origin 平移后的 rect，本地几何不受分数缩放改变
fn shifted(origin: (f32, f32), rect: (f32, f32, f32, f32)) -> Rect {
    Rect::from_xywh(origin.0 + rect.0, origin.1 + rect.1, rect.2, rect.3).unwrap()
}
/// 【属性取色】【明度区域】返回 origin 对应的 184×112 饱和度与明度矩形
pub fn sv(origin: (f32, f32)) -> Rect {
    shifted(origin, (8.0, 8.0, 184.0, 112.0))
}
/// 【属性取色】【色相轨道】返回 origin 对应的横向十二像素高轨道
pub fn hue(origin: (f32, f32)) -> Rect {
    shifted(origin, (8.0, 126.0, 184.0, 12.0))
}
/// 【属性取色】【透明轨道】返回 origin 对应的横向透明度轨道
pub fn alpha(origin: (f32, f32)) -> Rect {
    shifted(origin, (8.0, 144.0, 184.0, 12.0))
}
/// 【属性取色】【当前预览】返回 origin 对应的当前颜色方块
pub fn current(origin: (f32, f32)) -> Rect {
    shifted(origin, (8.0, 162.0, 26.0, 26.0))
}
/// 【属性取色】【十六进制】返回 origin 对应的唯一文本字段，不添加 RGBA 数值行
pub fn hex(origin: (f32, f32)) -> Rect {
    shifted(origin, (40.0, 162.0, 152.0, 26.0))
}
/// 【属性取色】【历史颜色】返回 origin、index 对应实际矩形；Qt 的固定色块布局存在两像素重叠
pub fn history(origin: (f32, f32), index: usize) -> Option<Rect> {
    HISTORY_X
        .get(index)
        .map(|x| shifted(origin, (*x, 194.0, 26.0, 26.0)))
}
/// 【属性取色】【控件裁切】返回 origin 对应的取色器子控件边界，最后一个历史色块按原版裁切
pub fn content(origin: (f32, f32)) -> Rect {
    shifted(origin, (8.0, 8.0, 184.0, 212.0))
}
/// 【属性取色】【横向取值】按 Qt QPoint 先取整 point 的 x，再映射为 maximum 整数值；轨道边界使用 width-1
pub fn value(rect: Rect, point: (f64, f64), maximum: u16) -> u16 {
    (((point.0.round() - rect.left() as f64) / (rect.width() - 1.0) as f64) * f64::from(maximum))
        .round()
        .clamp(0.0, f64::from(maximum)) as u16
}

/// 【属性取色】【弹层锚点】按 anchor、properties、size、bounds 返回 Qt 色块中心下方位置；超出底边时移到属性栏上方
pub fn position(
    anchor: (f32, f32),
    properties: Rect,
    size: (f32, f32),
    bounds: (i32, i32),
) -> (f32, f32) {
    let mut y = anchor.1.round() + 14.0;
    if y + size.1 > bounds.1 as f32 - 8.0 {
        y = properties.top() - size.1 - 8.0;
    }
    (
        (anchor.0.round() - (size.0 / 2.0).floor())
            .clamp(8.0, (bounds.0 as f32 - size.0 - 8.0).max(8.0)),
        y.clamp(8.0, (bounds.1 as f32 - size.1 - 8.0).max(8.0)),
    )
}
