//! 【工具提示】【组件入口】组合延迟状态、边缘布局与统一面板绘制

mod draw;
mod layout;
mod state;

pub(crate) use draw::draw;
pub(crate) use layout::place;
pub use state::State;
pub(crate) use state::{Target, View};

#[cfg(test)]
pub(crate) use state::DELAY;

#[cfg(test)]
mod tests;
