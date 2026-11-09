//! 【颜色控件】【实际预览】透明棋盘与实际颜色共用绘制，设置值不使用文本占位代替色块

use crate::{renderer::Painter, theme::Rgba};
use tiny_skia::Rect;

/// 【颜色控件】【色块绘制】按painter、bounds和color绘制透明底与实际颜色，无返回值
pub fn draw_swatch(painter: &mut Painter<'_>, bounds: Rect, color: Rgba) {
    let cell = 5.0;
    for row in 0..(bounds.height() / cell).ceil() as u32 {
        for column in 0..(bounds.width() / cell).ceil() as u32 {
            let left = bounds.left() + column as f32 * cell;
            let top = bounds.top() + row as f32 * cell;
            painter.fill_rounded(
                Rect::from_xywh(
                    left,
                    top,
                    cell.min(bounds.right() - left),
                    cell.min(bounds.bottom() - top),
                )
                .unwrap(),
                0.0,
                if (row + column) % 2 == 0 {
                    Rgba(225, 225, 225, 255)
                } else {
                    Rgba(170, 170, 170, 255)
                },
            );
        }
    }
    painter.fill_rounded(bounds, 0.0, color);
}
