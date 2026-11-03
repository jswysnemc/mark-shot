//! 【截图另存为】【入口】独立原生文件选择器与已冻结的最终图像

mod jobs;
pub mod snapshot;
mod window;

pub use snapshot::Snapshot;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Cancelled,
    Saved(PathBuf),
    Failed(String),
}

/// 【截图另存为】【对话框入口】用 snapshot 与 initial_path 打开文件选择器；返回取消、实际保存或保存失败
pub fn run(snapshot: Snapshot, initial_path: &Path) -> crate::AppResult<Outcome> {
    window::run(snapshot, initial_path)
}

#[cfg(test)]
mod tests;
