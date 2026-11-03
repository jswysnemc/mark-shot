//! Linux Wayland 截图、标注与离线文字识别

#[cfg(not(target_os = "linux"))]
compile_error!("mark-shot currently supports Linux Wayland only");

pub mod app;
pub mod backend;
pub mod cli;
pub mod config;
pub mod editor;
pub mod features;
pub mod fonts;
pub mod interaction;
pub mod keys;
pub mod logging;
pub mod ocr;
pub mod profiler;
pub mod renderer;
pub mod scroll;
pub mod theme;
pub mod tools;
pub mod types;
pub mod ui;
pub mod utils;

pub type AppResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
