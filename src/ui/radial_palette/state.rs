//! 【快捷色板】【按钮几何】复现 Qt 的十色排列、实际二十八像素按钮和笔宽预览

use crate::theme::Rgba;
use tiny_skia::Rect;

pub const SIZE: f32 = 178.0;
pub const COLORS: [Rgba; 10] = [
    Rgba(239, 68, 68, 255),
    Rgba(249, 115, 22, 255),
    Rgba(234, 179, 8, 255),
    Rgba(34, 197, 94, 255),
    Rgba(6, 182, 212, 255),
    Rgba(59, 130, 246, 255),
    Rgba(139, 92, 246, 255),
    Rgba(236, 72, 153, 255),
    Rgba(248, 250, 252, 255),
    Rgba(15, 23, 42, 255),
];

#[derive(Default)]
pub struct RadialPalette {
    pub anchor: (f32, f32),
    pub monitor: usize,
    pub armed: Option<usize>,
    pub hovered: Option<usize>,
    pub preview_size: f32,
}

impl RadialPalette {
    /// 【快捷色板】【按钮位置】按 index 返回本地矩形；原版请求三十像素，但样式约束使实际大小为二十八
    pub fn button(index: usize) -> Option<Rect> {
        if index >= COLORS.len() {
            return None;
        }
        let angle = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::TAU * index as f64 / COLORS.len() as f64;
        Rect::from_xywh(
            (89.0 + angle.cos() * 68.0 - 15.0).round() as f32,
            (89.0 + angle.sin() * 68.0 - 15.0).round() as f32,
            28.0,
            28.0,
        )
    }

    /// 【快捷色板】【控件命中】由 point 返回按钮索引，中心预览与空白不执行取色
    pub fn hit(point: (f64, f64)) -> Option<usize> {
        (0..COLORS.len()).find(|index| {
            Self::button(*index).is_some_and(|rect| {
                point.0 >= rect.left() as f64
                    && point.0 < rect.right() as f64
                    && point.1 >= rect.top() as f64
                    && point.1 < rect.bottom() as f64
            })
        })
    }

    /// 【快捷色板】【锚点约束】按 bounds 返回以 anchor 为中心的位置；四边保留八像素，不跟随工具栏
    pub fn position(&self, bounds: (i32, i32)) -> (f32, f32) {
        (
            (self.anchor.0.round() - 89.0).clamp(8.0, (bounds.0 as f32 - SIZE - 8.0).max(8.0)),
            (self.anchor.1.round() - 89.0).clamp(8.0, (bounds.1 as f32 - SIZE - 8.0).max(8.0)),
        )
    }

    /// 【快捷色板】【尺寸预览】返回 width 对应的中心方块，显示范围为八至三十四像素
    pub fn preview(width: f32) -> Rect {
        let size = width.round().clamp(8.0, 34.0);
        Rect::from_xywh(
            89.0 - (size / 2.0).floor(),
            89.0 - (size / 2.0).floor(),
            size,
            size,
        )
        .unwrap()
    }
}
