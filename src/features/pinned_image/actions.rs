//! 【贴图】【图像交接】编辑和 OCR 使用匿名快照，保存使用统一原生文件选择器

use crate::AppResult;

pub const OCR_ARG: &str = "--pin-ocr-snapshot";
pub const OCR_COPY_ARG: &str = "--pin-ocr-copy-snapshot";

/// 【贴图】【编辑交接】将 png 交给独立编辑或 OCR 进程；ocr 决定模式，返回启动结果
pub fn open(png: &[u8], ocr: bool) -> AppResult {
    crate::utils::spawn_self(
        &[if ocr {
            OCR_ARG
        } else {
            "--history-edit-snapshot"
        }],
        png,
    )
}

/// 【贴图】【另存为】以原始 png 打开统一对话框；返回保存、取消或失败结果，不写入重复历史
pub fn save(png: Vec<u8>, density: f32) -> AppResult {
    let path = crate::utils::screenshot_path::planned(None)?;
    let mut snapshot = crate::features::save_as::Snapshot::new(png, density, String::new())?;
    snapshot.record_history = false;
    match crate::features::save_as::run(snapshot, &path)? {
        crate::features::save_as::Outcome::Failed(error) => Err(error.into()),
        _ => Ok(()),
    }
}

/// 【贴图】【文字复制交接】识别 png 并保留完成后复制的意图；首次模型下载引导不丢失该动作
pub fn copy_text(png: &[u8]) -> AppResult {
    crate::utils::spawn_self(&[OCR_COPY_ARG], png)
}
