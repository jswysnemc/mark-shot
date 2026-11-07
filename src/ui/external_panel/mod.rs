//! 【外部动作面板】【模块入口】原版打开方式和扩展共用紧凑侧边列表，绘制、布局、输入职责独立

pub mod draw;
mod icons;
pub mod layout;
pub mod state;
pub use state::{ExternalPanel, Kind};
