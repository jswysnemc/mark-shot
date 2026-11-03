//! 【序号】【模块入口】独立序号的样式、计数和几何，不复用旧编号箭头

pub mod drag;
pub mod draw;
mod format;
mod geometry;
mod model;

pub use model::{NumberMark, NumberSequence, NumberStyle, WIDTH, clamped_width};
