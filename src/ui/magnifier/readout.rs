//! 【光标放大区】【紧凑读数】共用信息条显示原图颜色、色块与像素坐标

use crate::{
    renderer::{Painter, paths::rounded_rect_path, text::TextStyle},
    theme::{font, radius},
    ui::settings_panel::ValueField,
};
use tiny_skia::{Color, Rect, Stroke};

/// 【光标放大区】【读数宽度】无参数，返回容纳色值和五位像素坐标的宽度；圆镜更宽时随圆镜扩展
pub(super) fn width() -> f32 {
    (super::size() as f32).max(120.0)
}

/// 【光标放大区】【紧凑行高】无参数，返回兼容旧配置的双行信息条单行高度
pub(super) fn line_height() -> f32 {
    crate::config::get().magnifier.label_height.min(20.0)
}

/// 【光标放大区】【读数绘制】按 bounds、原图坐标 native 和采样 color 向 p 绘制双行信息；无效颜色显示占位值，无返回值
pub(super) fn draw(painter: &mut Painter, bounds: Rect, native: (f64, f64), color: Option<Color>) {
    let config = &crate::config::get().magnifier;
    let height = line_height();
    let top = bounds.bottom() - height * 2.0;
    let plate = Rect::from_xywh(bounds.left(), top, bounds.width(), height * 2.0).unwrap();
    painter.fill_rounded(plate, radius::panel(), config.label_background.get());
    outline(painter, plate, radius::panel());

    // 1. 【光标放大区】【颜色读数】复用统一 HEX 格式，色块与色值共用原图采样结果
    let text = color.map_or_else(|| "—".into(), |value| ValueField::Hex.text(value));
    let group_width =
        config.swatch_size + config.swatch_gap + painter.measure(&text, font::small());
    let left = plate.left() + (plate.width() - group_width) / 2.0;
    let swatch = Rect::from_xywh(
        left,
        top + (height - config.swatch_size) / 2.0,
        config.swatch_size,
        config.swatch_size,
    )
    .unwrap();
    painter.fill_rounded(
        swatch,
        config.swatch_radius,
        color.unwrap_or(Color::TRANSPARENT),
    );
    outline(painter, swatch, config.swatch_radius);
    let text_left = swatch.right() + config.swatch_gap;
    let text_rect = Rect::from_xywh(text_left, top, plate.right() - text_left, height).unwrap();
    painter.text(
        text_rect,
        &text,
        TextStyle::new(font::small(), config.label_text.get().color()),
    );

    // 2. 【光标放大区】【坐标读数】第二行保留原图像素坐标，与色值使用相同取整规则
    let coordinates = Rect::from_xywh(plate.left(), top + height, plate.width(), height).unwrap();
    painter.text(
        coordinates,
        &format!(
            "X: {}  Y: {}",
            native.0.floor() as i32,
            native.1.floor() as i32
        ),
        TextStyle::new(font::small(), config.label_text.get().color()).center(),
    );
}

/// 【光标放大区】【读数边框】按 rect 和 radius 向 painter 绘制内缩边框，无返回值
fn outline(painter: &mut Painter, rect: Rect, radius: f32) {
    let config = &crate::config::get().magnifier;
    let inset = config.outline_width / 2.0;
    let Some(inner) = rect.inset(inset, inset) else {
        return;
    };
    let Some(path) = rounded_rect_path(&inner, (radius - inset).max(0.0), true, true, true, true)
    else {
        return;
    };
    painter.stroke_path(
        &path,
        config.outline.get(),
        &Stroke {
            width: config.outline_width,
            ..Stroke::default()
        },
    );
}
