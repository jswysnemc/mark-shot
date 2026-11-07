//! Linux Wayland与Windows原生截图、标注与离线文字识别

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("mark-shot supports Linux Wayland and Windows");

pub mod app;
pub mod backend;
pub mod cli;
pub mod config;
pub mod editor;
pub mod features;
pub mod fonts;
pub mod image_source;
pub mod interaction;
pub mod keys;
pub mod logging;
pub mod ocr;
pub mod profiler;
pub mod renderer;
pub mod scroll;
pub mod theme;
pub mod tools;
pub mod translation;
pub mod types;
pub mod ui;
pub mod upload;
pub mod utils;

pub type AppResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
