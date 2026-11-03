//! 【滤镜测试】【缓存容量】验证存量上限、原图复用和删除释放

use mark_shot::editor::filter_cache::{FilterCache, MAX_CACHE_BYTES};
use mark_shot::features::filter::{FilterMode, RegionFilter};
use mark_shot::renderer::FrozenImage;
use tiny_skia::{Color, Pixmap, Rect};

/// 【滤镜测试】【存量上限】五个四 MiB 区域最多保留四个，最早使用缓存先释放，字节统计保持准确
#[test]
fn cache_evicts_oldest_entries_before_exceeding_16_mib() {
    let mut source = Pixmap::new(1024, 1024).unwrap();
    source.fill(Color::from_rgba8(10, 20, 30, 128));
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 1024.0, 1024.0).unwrap());
    let filter =
        RegionFilter::new((0.0, 0.0), (1024.0, 1024.0), 14.0, FilterMode::Grayscale).unwrap();
    let mut cache = FilterCache::default();
    for id in 0..5 {
        assert!(cache.prepare(id, filter, image));
        assert!(cache.retained_bytes() <= MAX_CACHE_BYTES);
    }
    assert_eq!(cache.images.len(), 4);
    assert!(!cache.images.contains_key(&0));
    assert_eq!(cache.retained_bytes(), MAX_CACHE_BYTES);
    cache.remove(1);
    assert_eq!(cache.retained_bytes(), MAX_CACHE_BYTES - 4 * 1024 * 1024);
    cache.retain_live(&[], None);
    assert_eq!(cache.retained_bytes(), 0);
    assert!(cache.images.is_empty());
}

/// 【滤镜测试】【旋转复用】相同区域旋转复用原像素，修改模式只更新当前缓存，重复删除不改变统计
#[test]
fn rotation_reuses_pixels_and_property_changes_replace_only_one_entry() {
    let mut source = Pixmap::new(100, 80).unwrap();
    source.fill(Color::from_rgba8(10, 20, 30, 128));
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 100.0, 80.0).unwrap());
    let filter = RegionFilter::new((10.0, 10.0), (70.0, 60.0), 14.0, FilterMode::Blur).unwrap();
    let mut cache = FilterCache::default();
    assert!(cache.prepare(1, filter, image));
    let original = cache.images[&1].pixels.data().as_ptr();
    assert!(!cache.prepare(
        1,
        RegionFilter {
            rotation: 0.7,
            ..filter
        },
        image
    ));
    assert_eq!(cache.images[&1].pixels.data().as_ptr(), original);
    assert!(cache.prepare(
        1,
        RegionFilter {
            mode: FilterMode::Brighten,
            ..filter
        },
        image
    ));
    assert_eq!(cache.images[&1].key.mode, FilterMode::Brighten);
    assert_eq!(cache.retained_bytes(), 60 * 50 * 4);
    cache.remove(1);
    cache.remove(1);
    assert_eq!(cache.retained_bytes(), 0);
}
