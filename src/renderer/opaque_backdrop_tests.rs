//! 【截图绘制回归】【透明合成】验证单表面与原有前景覆盖背景具有相同预乘像素，局部恢复不重复绘制

use crate::image_source::ImageSource;
use tiny_skia::{ColorU8, IntRect, Pixmap, PixmapPaint, Transform};

/// 【截图绘制回归】【冻结纹理】按width、height和opaque生成独立预乘纹理，返回像素，包含完整通道及透明度
fn texture(width: u32, height: u32, opaque: bool) -> Pixmap {
    let mut pixels = Pixmap::new(width, height).unwrap();
    for (index, pixel) in pixels.pixels_mut().iter_mut().enumerate() {
        let hash = (index as u32).wrapping_mul(2_654_435_761).rotate_left(7);
        *pixel = ColorU8::from_rgba(
            hash as u8,
            (hash >> 8) as u8,
            (hash >> 16) as u8,
            if opaque { 255 } else { index as u8 },
        )
        .premultiply();
    }
    pixels
}

/// 【截图绘制回归】【私有目录】无参数，返回测试磁盘目录，避免修改用户缓存或使用内存文件系统
fn directory() -> tempfile::TempDir {
    let root = std::env::current_dir()
        .unwrap()
        .join("target/opaque-backdrop-tests");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// 【截图绘制回归】【前后景等价】无参数，真实缓存背景与完整透明前景合成必须逐像素等于原有覆盖顺序，无返回值
#[test]
fn full_backdrop_matches_source_over_with_all_alpha_values() {
    let directory = directory();
    let original = texture(1100, 1100, true);
    let foreground = texture(1100, 1100, false);
    let mut expected = original.clone();
    expected.draw_pixmap(
        0,
        0,
        foreground.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let source = ImageSource::store_in(original, directory.path());
    let mut actual = foreground;
    super::compose(&source, &mut actual, None);
    assert_eq!(actual.data(), expected.data());
}

/// 【截图绘制回归】【局部恢复】无参数，替换前景损坏后只合成对应背景，区域外和未改动的既有画面保持不变，无返回值
#[test]
fn damaged_backdrop_restores_only_the_changed_pixels() {
    let original = texture(250, 180, true);
    let source = ImageSource::memory(original.clone());
    let mut actual = texture(250, 180, false);
    super::compose(&source, &mut actual, None);
    for (x, y, width, height) in [(17, 33, 99, 80), (245, 175, 5, 5), (0, 0, 250, 2)] {
        let patch = texture(width, height, false);
        let mut complete = original.clone();
        complete.draw_pixmap(
            x,
            y,
            patch.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        let expected_region = complete
            .clone_rect(IntRect::from_xywh(x, y, width, height).unwrap())
            .unwrap();
        let mut expected = actual.clone();
        crate::renderer::paste(&mut expected, &expected_region, x, y);
        crate::renderer::paste(&mut actual, &patch, x, y);
        super::compose(
            &source,
            &mut actual,
            Some((x as u32, y as u32, width, height)),
        );
        assert_eq!(actual.data(), expected.data());
    }
}
