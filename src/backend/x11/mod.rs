//! 【X11后端】【模块入口】原生连接、捕获与窗口分别管理生命周期

pub mod capture;
pub mod clipboard;
pub mod connection;
mod cursor;
mod input;
mod input_method;
pub mod overlay;
pub mod pixels;
mod popup_grab;
mod surface;
mod wait;
pub mod window;
pub mod windows;
