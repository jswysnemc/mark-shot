//! 【截图历史】【入口】完成截图的私有记录与独立历史工作流

mod actions;
mod chrome;
mod clear_dialog;
mod clear_dialog_draw;
mod draw;
pub mod identity;
mod input;
mod jobs;
mod layout;
mod list_draw;
mod model;
pub mod png_metadata;
mod preview;
mod scrollbar;
mod splitter;
pub mod transport;
mod window;

pub use window::run;
#[cfg(target_os = "linux")]
mod storage;
#[cfg(target_os = "windows")]
#[path = "capture_history/storage/windows.rs"]
mod storage;
pub mod store;

pub use store::{Entry, Store};

/// 【截图历史】【自动记录】按最新设置保存已完成 png，scale 为设备比例、search 为可选搜索文字；失败仅记录错误
pub fn remember(png: &[u8], scale: f32, search: &str) {
    let settings = crate::config::history::current();
    if !settings.enabled {
        return;
    }
    let result = settings.path().and_then(|path| {
        Store::new(path, settings.byte_limit()).append(
            png,
            scale,
            if settings.index_annotations {
                search
            } else {
                ""
            },
            settings.limit,
        )
    });
    match result {
        Ok(entry) => log::info!("【截图历史】【自动记录】已保存 {}", entry.name),
        Err(error) => log::error!("【截图历史】【自动记录】保存失败: {error}"),
    }
}

#[cfg(test)]
mod tests;
