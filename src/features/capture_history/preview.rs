//! 【截图历史】【流式预览】校验历史文件后使用共享的有界 PNG 缩略解码

use std::{
    fs::File,
    io::{self, BufReader, Seek, SeekFrom},
    sync::atomic::AtomicBool,
};

/// 【截图历史】【预览读取】验证 file 并按 bounds 缩小，cancelled 可中止读取；返回预乘像素或文件错误
pub(super) fn load(
    mut file: File,
    bounds: (u32, u32),
    cancelled: &AtomicBool,
) -> io::Result<tiny_skia::Pixmap> {
    super::png_metadata::inspect(&mut file)?;
    file.seek(SeekFrom::Start(0))?;
    crate::utils::png_preview::load(BufReader::new(file), bounds, cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【截图历史测试】【长图预览】预览限制尺寸，透明颜色使用预乘平均；取消任务立即停止
    #[test]
    fn streamed_preview_preserves_alpha_and_bounds_long_images() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        file.write_all(&super::super::tests::png(16, 4096)).unwrap();
        let image = load(file.reopen().unwrap(), (800, 600), &AtomicBool::new(false)).unwrap();
        assert_eq!((image.width(), image.height()), (2, 600));
        assert!(image.pixels().iter().all(|pixel| pixel.alpha() == 91
            && pixel.red() == 4
            && pixel.green() == 29
            && pixel.blue() == 61));
        assert_eq!(
            load(file.reopen().unwrap(), (800, 600), &AtomicBool::new(true))
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
    }
}
