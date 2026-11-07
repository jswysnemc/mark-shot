//! 【显示画布】【主题底色】仅用于原图之外和透明像素背后的显示，不参与原件或导出

use super::{Mode, Rgba};

/// 【显示画布】【模式配色】按 mode 返回不透明底色；深色保留既有外观，浅色使用中性浅灰
pub fn background(mode: Mode) -> Rgba {
    match mode {
        Mode::Light => Rgba(238, 240, 242, 255),
        Mode::Dark => Rgba(28, 31, 34, 255),
    }
}

/// 【显示画布】【当前底色】返回当前主题的显示底色，不读取或修改图像内容
pub fn current() -> Rgba {
    background(Mode::current())
}
