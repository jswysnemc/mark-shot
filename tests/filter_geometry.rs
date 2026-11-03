//! 【滤镜测试】【几何与参数】覆盖原版模式、原图强度和分数缩放坐标

use mark_shot::features::filter::{
    FilterMode, RegionFilter, STRENGTH, clamped_strength, geometry::PixelGrid,
};
use mark_shot::renderer::FrozenImage;
use tiny_skia::{Pixmap, Rect};

/// 【滤镜测试】【模式兼容】五模式保持原版编号，名称与整数配置都可读取，未知值不误选
#[test]
fn modes_keep_reference_ids_and_stable_names() {
    for (index, mode) in FilterMode::ALL.into_iter().enumerate() {
        assert_eq!(mode as usize, index);
        assert_eq!(FilterMode::parse(mode.key()), Some(mode));
        assert_eq!(FilterMode::parse(&index.to_string()), Some(mode));
        let text = format!("mode = {}", index);
        #[derive(serde::Deserialize)]
        struct Config {
            mode: FilterMode,
        }
        assert_eq!(toml::from_str::<Config>(&text).unwrap().mode, mode);
    }
    assert_eq!(FilterMode::default(), FilterMode::Pixelate);
    assert_eq!(FilterMode::from_pixelate(false), FilterMode::Blur);
    assert_eq!(FilterMode::parse("other"), None);
    assert_eq!(FilterMode::parse("5"), None);
}

/// 【滤镜测试】【强度范围】默认十四，范围四至四十八，非有限值使用默认强度
#[test]
fn strength_is_native_and_has_reference_defaults() {
    assert_eq!(STRENGTH, (4.0, 48.0));
    assert_eq!(clamped_strength(f32::NAN), 14.0);
    assert_eq!(clamped_strength(f32::INFINITY), 14.0);
    assert_eq!(clamped_strength(-10.0), 4.0);
    assert_eq!(clamped_strength(100.0), 48.0);
    let filter = RegionFilter::new((20.0, 30.0), (80.0, 65.0), 14.0, FilterMode::Brighten).unwrap();
    let scaled = filter.scaled(2.75);
    assert_eq!(scaled.strength, 14.0);
    assert_eq!(scaled.mode, filter.mode);
    assert_eq!(scaled.start, (55.0, 82.5));
    assert_eq!(scaled.style_scale, 2.75);
}

/// 【滤镜测试】【矩形规范】反向拖动得到规范矩形，零面积草稿有效但不生成正文，非有限坐标拒绝创建
#[test]
fn reversed_rectangles_and_empty_drafts_are_distinct() {
    let filter = RegionFilter::new((80.0, 65.0), (20.0, 30.0), 14.0, FilterMode::Invert).unwrap();
    assert_eq!(filter.rect(), Rect::from_ltrb(20.0, 30.0, 80.0, 65.0));
    let empty = RegionFilter::new((20.0, 30.0), (20.0, 30.0), 14.0, FilterMode::Invert).unwrap();
    assert!(empty.valid());
    assert!(empty.rect().is_none());
    assert!(RegionFilter::new((f32::NAN, 30.0), (20.0, 30.0), 14.0, FilterMode::Invert).is_none());
}

/// 【滤镜测试】【原图网格】本地图像始终使用原始像素，导出缩放不改变区域对齐和强度
#[test]
fn mapped_source_keeps_native_pixel_regions() {
    let original = Pixmap::new(1600, 900).unwrap();
    let viewport = Rect::from_xywh(-80.0, 20.0, 640.0, 360.0).unwrap();
    let image = FrozenImage::image(&original, viewport);
    let filter = RegionFilter::new((-60.0, 40.0), (75.0, 130.0), 14.0, FilterMode::Blur).unwrap();
    let grid = image.native_grid().unwrap();
    assert_eq!((grid.width, grid.height, grid.scale), (1600, 900, 2.5));
    let region = grid.region(filter).unwrap();
    assert_eq!(
        (region.x(), region.y(), region.width(), region.height()),
        (50, 50, 338, 225)
    );
    for scale in [0.73, 1.2, 2.5] {
        let mapped = image.mapped(scale, (0.0, 0.0)).native_grid().unwrap();
        assert_eq!(mapped.region(filter.scaled(scale)), Some(region), "{scale}");
    }
}

/// 【滤镜测试】【裁切边界】像素范围向外对齐并限制原图，亚像素位置不会按逻辑整点截断
#[test]
fn fractional_pixel_regions_clip_without_logical_rounding() {
    let grid = PixelGrid::new(
        Rect::from_xywh(-20.0, 30.0, 100.0, 80.0).unwrap(),
        1.5,
        150,
        120,
    )
    .unwrap();
    let filter =
        RegionFilter::new((-20.25, 29.5), (-9.75, 40.25), 14.0, FilterMode::Pixelate).unwrap();
    let region = grid.region(filter).unwrap();
    assert_eq!(
        (region.x(), region.y(), region.width(), region.height()),
        (0, 0, 16, 16)
    );
    assert!(PixelGrid::new(grid.bounds, f32::NAN, 150, 120).is_none());
    assert!(PixelGrid::new(grid.bounds, 1.0, 0, 120).is_none());
}

/// 【滤镜测试】【旋转映射】视觉和局部坐标互为逆变换，旋转改变边界但不改变原图采样框
#[test]
fn rotation_preserves_the_unrotated_sampling_rectangle() {
    let mut filter =
        RegionFilter::new((100.0, 140.0), (260.0, 220.0), 14.0, FilterMode::Grayscale).unwrap();
    let rect = filter.rect().unwrap();
    filter.rotation = 0.37;
    let point = (118.0, 202.0);
    let restored = filter.to_local(filter.to_visual(point));
    assert!((restored.0 - point.0).abs() < 0.0001);
    assert!((restored.1 - point.1).abs() < 0.0001);
    assert_eq!(filter.rect(), Some(rect));
    assert!(filter.bounds().unwrap().height() > rect.height());
}
