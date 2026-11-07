//! 【文字识别】【结果窗口】可编辑原文、原图预览与表格导出

mod control_input;
pub mod document;
mod draw;
pub mod export;
mod feedback;
mod input_method;
mod labels;
mod layout;
mod more_menu;
mod palette;
mod source_image_menu;
mod source_menu;
mod source_menu_actions;
mod source_preview;
mod source_preview_draw;
mod stacking;
pub mod table;
mod table_geometry;
mod table_grid_draw;
mod table_grid_layout;
mod table_header_layout;
mod table_input;
mod table_menu;
mod table_scroll_draw;
mod table_scroll_input;
mod table_scroll_layout;
mod table_scroll_model;
mod table_view;
mod titlebar_draw;
mod titlebar_layout;
mod translation;
pub mod transport;
mod window;
mod window_input;

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
