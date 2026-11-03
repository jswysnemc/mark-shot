//! 【放大镜】【模块入口】持久双框放大镜，与光标取色放大镜相互独立

pub mod drag;
pub mod draw;
pub mod geometry;
mod model;

pub use model::{Magnifier, MagnifierShape, Point, clamped_zoom};
