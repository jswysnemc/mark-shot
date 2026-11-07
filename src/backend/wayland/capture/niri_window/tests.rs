//! 【niri 回归】【图像导入】尺寸、透明颜色与受限解码

use super::frame;

/// 【niri 回归】【透明窗口】使用已预乘的 PNG 模拟合成器，导入与再次导出必须保持同一预乘颜色，无参数、无返回值
#[test]
fn premultiplied_window_png_is_not_multiplied_twice() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("window.png");
    let pixels = vec![
        224, 96, 32, 255, 112, 48, 16, 128, 56, 24, 8, 64, 0, 0, 0, 0,
    ];
    image::save_buffer(&path, &pixels, 2, 2, image::ColorType::Rgba8).unwrap();
    let capture = frame::read(&path, 1.5).unwrap();
    assert_eq!(capture.scale, 1.5);
    assert_eq!((capture.pixmap.width(), capture.pixmap.height()), (2, 2));
    assert_eq!(capture.pixmap.data(), pixels);
    let encoded = crate::utils::encode_png_with_alpha(capture.pixmap.into_pixmap());
    let mut exported = image::load_from_memory(&encoded)
        .unwrap()
        .into_rgba8()
        .into_raw();
    crate::utils::premultiply(&mut exported);
    assert_eq!(exported, pixels);
}

/// 【niri 回归】【颜色约束】合成器图像的颜色通道不得超出透明度，无参数、无返回值
#[test]
fn unexpected_straight_alpha_png_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("window.png");
    image::save_buffer(&path, &[224, 96, 32, 128], 1, 1, image::ColorType::Rgba8).unwrap();
    assert!(
        frame::read(&path, 1.0)
            .err()
            .unwrap()
            .to_string()
            .contains("premultiplied")
    );
}

/// 【niri 回归】【损坏图片】非 PNG 或超大文件必须拒绝，不分配整张超限图像，无参数、无返回值
#[test]
fn corrupt_and_oversized_pngs_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("window.png");
    std::fs::write(&path, b"not a PNG").unwrap();
    assert!(frame::read(&path, 1.0).is_err());
    std::fs::File::create(&path)
        .unwrap()
        .set_len(256 * 1024 * 1024 + 1)
        .unwrap();
    assert!(
        frame::read(&path, 1.0)
            .err()
            .unwrap()
            .to_string()
            .contains("size limit")
    );
}

/// 【niri 回归】【颜色扩展上限】灰度 PNG 的 RGBA 扩展尺寸必须在像素解码前检查，无参数、无返回值
#[test]
fn grayscale_dimensions_cannot_exceed_the_rgba_memory_limit() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("window.png");
    image::save_buffer(&path, &[0], 1, 1, image::ColorType::L8).unwrap();
    let mut encoded = std::fs::read(&path).unwrap();
    encoded[16..20].copy_from_slice(&16384u32.to_be_bytes());
    encoded[20..24].copy_from_slice(&16384u32.to_be_bytes());
    let checksum = crc32fast::hash(&encoded[12..29]);
    encoded[29..33].copy_from_slice(&checksum.to_be_bytes());
    std::fs::write(&path, encoded).unwrap();
    assert!(
        frame::read(&path, 1.0)
            .err()
            .unwrap()
            .to_string()
            .contains("RGBA pixels")
    );
}
