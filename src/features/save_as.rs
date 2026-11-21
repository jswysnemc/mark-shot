//! 【截图另存为】【入口】通过桌面文件选择器门户与安全落盘器保存冻结的最终图像

pub(crate) mod portal;
#[cfg(target_os = "linux")]
mod runtime;
pub mod snapshot;
pub(crate) mod writer;

pub use snapshot::Snapshot;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Cancelled,
    Saved(PathBuf),
    Failed(String),
}

/// 【截图另存为】【字节保存入口】用 initial_path 打开桌面文件门户并将 png 写入选定路径，不修改历史记录
pub fn save_bytes(png: &[u8], initial_path: &Path) -> crate::AppResult<Outcome> {
    let Some(target) = portal::choose_save_path(initial_path)? else {
        return Ok(Outcome::Cancelled);
    };
    match writer::write_png(png, &target) {
        Ok(()) => Ok(Outcome::Saved(target)),
        Err(err) => {
            log::error!(
                "【截图另存为】【门户保存】写入文件失败 path={}: {err}",
                target.display()
            );
            Ok(Outcome::Failed(err.to_string()))
        }
    }
}

/// 【截图另存为】【对话框入口】用 snapshot 与 initial_path 调用桌面文件门户；保存成功后登记截图历史
pub fn run(snapshot: Snapshot, initial_path: &Path) -> crate::AppResult<Outcome> {
    let outcome = save_bytes(&snapshot.png, initial_path)?;
    if matches!(outcome, Outcome::Saved(_)) {
        snapshot.remember();
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests;
