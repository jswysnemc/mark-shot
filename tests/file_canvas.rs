use mark_shot::editor::file_canvas::FileCanvas;
use tiny_skia::{Color, Pixmap, Rect};

/// 【图片编辑测试】【原图像素】验证显示缩放不改变导出分辨率或透明像素
#[test]
fn fit_preview_preserves_native_export() {
    let mut original = Pixmap::new(1000, 500).unwrap();
    original.fill(Color::from_rgba8(200, 100, 50, 128));
    let expected = original.data().to_vec();
    let file = FileCanvas::new(original, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
    assert!(file.viewport.width() < 1000.0);
    let cropped = file.crop(file.viewport).unwrap();
    assert_eq!(
        (cropped.pixels.width(), cropped.pixels.height()),
        (1000, 500)
    );
    assert_eq!(cropped.pixels.data(), expected);
}

/// 【图片编辑测试】【逻辑裁切】验证全局偏移和选区比例精确映射到原始像素
#[test]
fn cropped_region_maps_to_source_pixels() {
    let file = FileCanvas::new(
        Pixmap::new(1000, 500).unwrap(),
        Rect::from_xywh(-640.0, 120.0, 640.0, 480.0).unwrap(),
    )
    .unwrap();
    let region = Rect::from_xywh(
        file.viewport.left(),
        file.viewport.top(),
        file.viewport.width() / 2.0,
        file.viewport.height(),
    )
    .unwrap();
    let cropped = file.crop(region).unwrap();
    assert_eq!(
        (cropped.pixels.width(), cropped.pixels.height()),
        (500, 500)
    );
    assert!(cropped.global_origin.0 < 0.0);
    assert!(
        file.crop(Rect::from_xywh(9999.0, 0.0, 10.0, 10.0).unwrap())
            .is_none()
    );
}

/// 【图片编辑测试】【长图读取】高度超过旧限制的有效长图仍可重新编辑，像素总量受内存预算约束
#[test]
fn tall_history_images_can_be_reopened_without_resizing_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("long.png");
    image::RgbaImage::from_pixel(32, 20_000, image::Rgba([128, 64, 32, 128]))
        .save(&path)
        .unwrap();
    let file = FileCanvas::load(&path, Rect::from_xywh(0.0, 0.0, 1000.0, 700.0).unwrap()).unwrap();
    assert_eq!(
        (file.original.width(), file.original.height()),
        (32, 20_000)
    );
    assert!(file.viewport.height() <= 604.0);
}
