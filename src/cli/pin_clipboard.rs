//! 【贴图】【剪贴板入口】将图片、颜色、文本及文件内容交接到独立置顶窗口

mod content;
mod render;
mod repeat;
mod source;
#[cfg(test)]
mod tests;

const IMAGE_LIMIT: u64 = 64 * 1024 * 1024;
const TEXT_LIMIT: u64 = 64 * 1024;

/// 【贴图】【内容交接】无参数，按剪贴板类型生成图片，成功启动后记录文件重复贴图状态，返回结果
pub(super) fn run() -> crate::AppResult {
    let clipboard = source::read()?;
    if clipboard.files.is_empty() {
        let png = content::image(
            &clipboard,
            false,
            crate::config::get().pinned.file_paths_to_image,
        )?;
        return crate::backend::pin::spawn(&png, 1.0);
    }
    let mut repeat = repeat::State::open()?;
    let fingerprint = clipboard.fingerprint();
    let repeated = repeat.previous() == Some(fingerprint) && !clipboard.files.is_empty();
    let png = content::image(
        &clipboard,
        repeated,
        crate::config::get().pinned.file_paths_to_image,
    )?;
    crate::backend::pin::spawn(&png, 1.0)?;
    repeat.save((!clipboard.files.is_empty()).then_some(fingerprint))?;
    Ok(())
}
