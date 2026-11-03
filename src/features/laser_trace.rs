//! 【激光】【临时笔迹】有容量限制的手势状态和消散绘制，不进入持久标注与导出

pub mod draw;
mod state;

pub use draw::draw;
pub use state::{
    LIFETIME, LaserStroke, LaserTrace, MAX_POINTS_PER_STROKE, MAX_STROKES, MAX_TOTAL_POINTS,
    REFRESH_INTERVAL,
};
