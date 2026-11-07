//! 【Windows平台】【原生后端】截图与窗口拥有系统资源，共用业务不持有HWND或设备上下文

pub mod capture;
pub mod clipboard;
mod input;
pub mod native;
pub mod overlay;
pub mod pin;
pub mod window;
