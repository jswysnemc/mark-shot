//! 【Windows独立窗口】【运行入口】共享内容绘制到原生表面，消息、任务和内容期限共同等待

mod popup;
mod runtime;
mod theme_input;
pub use runtime::run;
pub type WindowWake = super::native::Wake;
