//! 【贴图】【剪贴板入口】将图片、颜色、文本及文件内容交接到独立置顶窗口

mod content;
mod html;
mod html_style;
mod render;
mod repeat;
mod rich_text;
mod rtf;
mod source;
mod spans;
#[cfg(test)]
mod tests;

const IMAGE_LIMIT: u64 = 64 * 1024 * 1024;
const TEXT_LIMIT: u64 = 64 * 1024;
const MARKUP_LIMIT: u64 = 16 * 1024 * 1024;

/// 【贴图】【内容交接】无参数，按剪贴板类型生成图片，成功启动后在支持序号的平台记录文件重复贴图状态，返回结果
pub(super) fn run() -> crate::AppResult {
    let clipboard = source::read()?;
    let text_scale = crate::features::pinned_image::density::text_scale();
    if clipboard.files.is_empty() {
        let content = content::image(
            &clipboard,
            false,
            crate::config::get().pinned.file_paths_to_image,
            text_scale,
        )?;
        return crate::backend::pin::spawn(&content.png, content.pin_scale());
    }
    // 1. 【贴图】【重复检测】只有平台提供剪贴板序号时才区分新复制，避免 Wayland 误判同一路径
    let mut repeat = if clipboard.has_sequence() {
        Some(repeat::State::open()?)
    } else {
        None
    };
    let fingerprint = clipboard.fingerprint();
    let repeated = repeat
        .as_ref()
        .is_some_and(|state| state.previous() == Some(fingerprint));
    let content = content::image(
        &clipboard,
        repeated,
        crate::config::get().pinned.file_paths_to_image,
        text_scale,
    )?;
    crate::backend::pin::spawn(&content.png, content.pin_scale())?;
    if let Some(repeat) = repeat.as_mut() {
        repeat.save(Some(fingerprint))?;
    }
    Ok(())
}
