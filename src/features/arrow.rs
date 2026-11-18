//! 【箭头】【模块入口】把独立头部样式与共用线条骨架组合，不复用编号箭头计数

mod direct;
pub mod draw;
mod fletched;
pub mod metrics;
pub mod path;
mod style;
mod style_serde;
mod tail;
#[cfg(test)]
mod tail_tests;

pub use style::{ArrowHead, ArrowStyle, ArrowTail};
