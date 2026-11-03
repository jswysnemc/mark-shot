//! 【文字识别】【结果窗口】可编辑原文、原图预览与表格导出

pub mod document;
mod draw;
pub mod export;
mod layout;
pub mod table;
mod table_geometry;
mod table_view;
pub mod transport;
mod window;

pub use window::run;

/// 【文字识别】【结果开关】读取结果窗口环境变量或配置；未设置环境变量时使用 result_panel
pub fn enabled() -> bool {
    for name in ["MARK_SHOT_OCR_RESULT_WINDOW", "MARK_SHOT_OCR_RESULT_PANEL"] {
        if let Ok(value) = std::env::var(name) {
            match value.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => return true,
                "0" | "false" | "no" | "off" => return false,
                _ => log::info!("【文字识别】【结果开关】忽略无效开关 {name}"),
            }
        }
    }
    crate::config::get().ocr.result_panel
}
