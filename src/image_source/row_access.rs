//! 【图像原件】【分批行访问】仅借用相交行，读取预算与既有预览一致，不创建完整原件或中间图像

use super::ImageSource;
#[cfg(any(target_os = "linux", test))]
use super::MEMORY_BYTES;
use tiny_skia::IntRect;

/// 【图像原件】【行带访问】按source和region逐行执行visit，返回值为空；每批读取至多原件预算，回调不会持有跨批像素
#[cfg(any(target_os = "linux", test))]
pub(super) fn visit(source: &ImageSource, region: IntRect, visit: impl FnMut(u32, &[u8])) {
    visit_bounded(source, region, MEMORY_BYTES, visit);
}

/// 【图像原件】【读取预算】按source、region和budget逐行执行visit，无返回值；每批至多budget字节完整行，单行超出预算时仍保留完整行
pub(super) fn visit_bounded(
    source: &ImageSource,
    region: IntRect,
    budget: usize,
    mut visit: impl FnMut(u32, &[u8]),
) {
    let bounds = IntRect::from_xywh(0, 0, source.width(), source.height()).unwrap();
    let Some(region) = region.intersect(&bounds) else {
        return;
    };
    let stride = source.width() as usize * 4;
    let rows = (budget / stride).max(1);
    let width = region.width() as usize * 4;
    // 1. 【图像原件】【有界借用】完整视图只确定地址，实际只读取回调消费的相交行，不预读其他列或原件区域
    for first in (region.top() as usize..region.bottom() as usize).step_by(rows) {
        let last = (first + rows).min(region.bottom() as usize);
        for row in first..last {
            let start = row * stride + region.left() as usize * 4;
            visit(row as u32, &source.data()[start..start + width]);
        }
        // 2. 【图像原件】【批次回收】本批回调结束后归还只读映射及干净文件页，下一批继续借用原始坐标
        source.release_pages();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Pixmap;

    /// 【图像原件行测试】【裁切坐标】无参数，验证负边界裁切后的行号、完整预乘字节和相交列，无返回值
    #[test]
    fn clipped_rows_preserve_original_coordinates_and_bytes() {
        let mut pixels = Pixmap::new(33, 29).unwrap();
        for (index, pixel) in pixels.pixels_mut().iter_mut().enumerate() {
            *pixel = tiny_skia::ColorU8::from_rgba(
                index as u8,
                (index >> 3) as u8,
                91,
                (index >> 1) as u8,
            )
            .premultiply();
        }
        let original = pixels.clone();
        let source = ImageSource::memory(pixels);
        let mut visited = Vec::new();
        visit(
            &source,
            IntRect::from_xywh(-3, 19, 13, 19).unwrap(),
            |row, bytes| {
                visited.push(row);
                let start = row as usize * 33 * 4;
                assert_eq!(bytes, &original.data()[start..start + 10 * 4]);
            },
        );
        assert_eq!(visited, (19..29).collect::<Vec<_>>());
    }

    /// 【图像原件行测试】【磁盘借用】无参数，真实私有缓存的完整跨批行访问保持原字节，结束后仍能重读，无返回值
    #[test]
    fn cached_rows_preserve_pixels_across_reclaimed_batches() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/native-row-tests");
        std::fs::create_dir_all(&root).unwrap();
        let directory = tempfile::tempdir_in(root).unwrap();
        let mut pixels = Pixmap::new(1100, 1100).unwrap();
        for (index, pixel) in pixels.pixels_mut().iter_mut().enumerate() {
            *pixel = tiny_skia::ColorU8::from_rgba(
                index as u8,
                (index >> 3) as u8,
                91,
                (index >> 1) as u8,
            )
            .premultiply();
        }
        let original = pixels.clone();
        let source = ImageSource::store_in(pixels, directory.path());
        assert!(source.cached_bytes() > MEMORY_BYTES);
        for budget in [0, 13, 2 * 1024 * 1024, MEMORY_BYTES] {
            let mut rows = 0;
            source.visit_rows_with_budget(
                IntRect::from_xywh(19, 17, 977, 1083).unwrap(),
                budget,
                |row, bytes| {
                    let first = (row as usize * 1100 + 19) * 4;
                    assert_eq!(bytes, &original.data()[first..first + 977 * 4]);
                    rows += 1;
                },
            );
            assert_eq!(rows, 1083);
        }
        assert_eq!(source.data(), original.data());
    }
}
