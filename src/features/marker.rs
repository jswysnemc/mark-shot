//! 【形状标记】【入口】构造与绘制原版 24 种标记，按需缓存单位路径

mod boolean;
mod composite;
mod curves;
pub mod draw;
pub mod paths;
mod primitives;

pub use draw::{draw_icon, draw_marker};
pub use paths::path_for_shape;
