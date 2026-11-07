//! 【命令行】【入口】参数解析、任务分发与无界面输出

mod capture_output;
mod capture_runtime;
mod dispatch;
mod headless;
mod help;
mod pin_clipboard;
#[cfg(target_os = "linux")]
mod single_instance;
#[cfg(target_os = "windows")]
#[path = "single_instance_windows.rs"]
mod single_instance;
mod tool_names;

pub mod arguments;
pub use dispatch::run;
