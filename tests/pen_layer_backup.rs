//! 【画笔缓存测试】【原始像素】验证按块备份的边界、重复输入、多显示器和生命周期

use mark_shot::editor::pen_layer_backup::PenLayerBackup;
use tiny_skia::{Color, IntRect, Pixmap};

/// 【画笔缓存测试】【图层构造】按 width、height 和 color 返回填充好的原始图层
fn layer(width: u32, height: u32, color: Color) -> Pixmap {
    let mut layer = Pixmap::new(width, height).unwrap();
    layer.fill(color);
    layer
}

/// 【画笔缓存测试】【稀疏备份】小区域只备份一个像素块，恢复原始预乘像素，不改变未触及的块
#[test]
fn sparse_snapshots_restore_exact_original_tiles() {
    let red = Color::from_rgba8(255, 0, 0, 128);
    let blue = Color::from_rgba8(0, 0, 255, 255);
    let mut layers = [layer(128, 128, red)];
    let original = layers[0].clone();
    let mut backup = PenLayerBackup::default();
    assert_eq!(
        backup.snapshot(0, &layers[0], IntRect::from_xywh(10, 10, 20, 20).unwrap()),
        64 * 64 * 4
    );
    layers[0].fill(blue);
    let damage = backup.restore(&mut layers);
    assert_eq!(damage, [(0, IntRect::from_xywh(0, 0, 64, 64).unwrap())]);
    assert_eq!(layers[0].pixel(30, 30), original.pixel(30, 30));
    assert_eq!(
        layers[0].pixel(90, 90),
        Some(blue.to_color_u8().premultiply())
    );
    assert!(backup.is_empty());
    assert_eq!(backup.snapshot_bytes(), 0);
}

/// 【画笔缓存测试】【重复区域】重复备份不能覆盖本手势前的原始像素，也不增加存储字节
#[test]
fn repeated_snapshot_requests_do_not_record_already_drawn_pixels() {
    let original = layer(128, 128, Color::from_rgba8(120, 80, 40, 128));
    let mut layers = [original.clone()];
    let mut backup = PenLayerBackup::default();
    backup.snapshot(0, &layers[0], IntRect::from_xywh(10, 10, 20, 20).unwrap());
    let count = backup.snapshot_bytes();
    layers[0].fill(Color::from_rgba8(0, 255, 0, 255));
    assert_eq!(
        backup.snapshot(0, &layers[0], IntRect::from_xywh(20, 20, 35, 35).unwrap()),
        0
    );
    assert_eq!(backup.snapshot_bytes(), count);
    backup.restore(&mut layers);
    assert_eq!(layers[0].pixel(40, 40), original.pixel(40, 40));
}

/// 【画笔缓存测试】【边缘裁切】超出边界的请求只保存真实图层像素，部分像素块不会读取行外数据
#[test]
fn edge_tiles_and_outside_requests_are_clipped_to_the_layer() {
    let original = layer(73, 70, Color::from_rgba8(30, 80, 150, 180));
    let mut layers = [original.clone()];
    let mut backup = PenLayerBackup::default();
    assert_eq!(
        backup.snapshot(0, &layers[0], IntRect::from_xywh(60, 60, 30, 30).unwrap()),
        73 * 70 * 4
    );
    assert_eq!(
        backup.snapshot(0, &layers[0], IntRect::from_xywh(200, 200, 10, 10).unwrap()),
        0
    );
    layers[0].fill(Color::TRANSPARENT);
    let damage = backup.restore(&mut layers);
    assert!(layers[0].data() == original.data());
    assert_eq!(damage, [(0, IntRect::from_xywh(0, 0, 73, 70).unwrap())]);
}

/// 【画笔缓存测试】【多个显示器】同样的像素坐标在不同显示器上保存独立备份，并分别通知损坏
#[test]
fn snapshots_on_multiple_monitors_remain_independent() {
    let originals = [
        layer(128, 128, Color::from_rgba8(255, 0, 0, 128)),
        layer(128, 128, Color::from_rgba8(0, 255, 0, 128)),
    ];
    let mut layers = originals.clone();
    let mut backup = PenLayerBackup::default();
    for (index, layer) in layers.iter().enumerate() {
        backup.snapshot(index, layer, IntRect::from_xywh(60, 60, 20, 20).unwrap());
    }
    for layer in &mut layers {
        layer.fill(Color::TRANSPARENT);
    }
    let damage = backup.restore(&mut layers);
    assert_eq!(damage.len(), 2);
    for (index, layer) in layers.iter().enumerate() {
        assert!(layer.data() == originals[index].data());
    }
}

/// 【画笔缓存测试】【缓存释放】丢弃备份不修改图层；随后恢复为空，不残留像素字节
#[test]
fn discarding_a_snapshot_releases_pixels_without_restoring_them() {
    let mut layers = [layer(100, 100, Color::from_rgba8(0, 0, 255, 255))];
    let mut backup = PenLayerBackup::default();
    backup.snapshot(0, &layers[0], IntRect::from_xywh(1, 1, 20, 20).unwrap());
    layers[0].fill(Color::TRANSPARENT);
    backup.discard();
    assert!(backup.restore(&mut layers).is_empty());
    assert_eq!(backup.snapshot_bytes(), 0);
    assert!(layers[0].pixels().iter().all(|pixel| pixel.alpha() == 0));
}

/// 【画笔缓存测试】【尺寸变化】恢复时跳过已经更换尺寸的图层，不写入旧坐标或发生越界
#[test]
fn changed_or_missing_layers_do_not_receive_old_pixels() {
    let original = layer(128, 128, Color::from_rgba8(255, 0, 0, 255));
    let mut backup = PenLayerBackup::default();
    backup.snapshot(0, &original, IntRect::from_xywh(1, 1, 20, 20).unwrap());
    backup.snapshot(2, &original, IntRect::from_xywh(1, 1, 20, 20).unwrap());
    let mut layers = [Pixmap::new(64, 64).unwrap()];
    assert!(backup.restore(&mut layers).is_empty());
    assert!(layers[0].pixels().iter().all(|pixel| pixel.alpha() == 0));
    assert!(backup.is_empty());
}
