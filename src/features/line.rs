//! 【线条】【模块入口】共用直线和箭头骨架、平滑路径、线型及控制点

pub mod drag;
pub mod draw;
pub mod geometry;
mod model;
pub mod polyline;

pub use model::{LineSkeleton, LineStyle, MAX_CONTROLS, Point, WIDTH, clamped_width};
