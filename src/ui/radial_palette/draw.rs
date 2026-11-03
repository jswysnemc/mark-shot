//! 【快捷色板】【透明绘制】只画十个色块与中心预览，不添加 LumineCapture 面板底板

use super::{COLORS, RadialPalette};
use crate::{renderer::Painter, theme::Rgba};
use tiny_skia::{Rect, Stroke};

/// 【快捷色板】【圆角按钮】按 p、rect、fill、border 绘制原版按钮；背景不延伸到半透明边框外沿
fn button(p: &mut Painter, rect: Rect, fill: Rgba, border: Rgba) {
    let inner = Rect::from_xywh(
        rect.left() + 1.0,
        rect.top() + 1.0,
        rect.width() - 2.0,
        rect.height() - 2.0,
    )
    .unwrap();
    p.fill_rounded(inner, 11.0, fill);
    if let Some(path) =
        crate::renderer::paths::rounded_rect_path(&inner, 11.0, true, true, true, true)
    {
        p.stroke_path(
            &path,
            border,
            &Stroke {
                width: 2.0,
                ..Default::default()
            },
        );
    }
}

/// 【快捷色板】【内容绘制】将 palette 和 color 绘入本地 p，控件与命中共用相同矩形
pub fn draw(p: &mut Painter, palette: &RadialPalette, color: Rgba) {
    for (index, fill) in COLORS.iter().copied().enumerate() {
        let border = if palette.hovered == Some(index) {
            Rgba(45, 212, 191, 255)
        } else {
            Rgba(15, 17, 23, 220)
        };
        button(p, RadialPalette::button(index).unwrap(), fill, border);
    }
    p.fill_rounded(
        RadialPalette::preview(palette.preview_size),
        3.0,
        Rgba(color.0, color.1, color.2, 255),
    );
}
