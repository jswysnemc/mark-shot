//! 【历史原图回归】【成对存储】验证重选区原件、容量及清理兼容

use crate::features::capture_history::Store;

/// 【历史原图回归】【无损原件】无参数，完成图与完整原图独立读取，成对计费和删除
#[test]
fn original_snapshot_keeps_uncropped_pixels_and_is_deleted_with_result() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path(), 1024 * 1024);
    let crop = super::png(16, 9);
    let original = super::png(240, 180);
    let entry = store
        .append_with_original(&crop, 1.25, "", 20, Some(&original))
        .unwrap();
    assert!(entry.has_original);
    assert_eq!(store.entries().unwrap(), vec![entry.clone()]);
    assert_eq!(
        image::load_from_memory(&store.read(&entry.name).unwrap().0)
            .unwrap()
            .to_rgba8(),
        image::load_from_memory(&crop).unwrap().to_rgba8()
    );
    assert_eq!(
        image::load_from_memory(&store.read_original(&entry.name).unwrap().0)
            .unwrap()
            .to_rgba8(),
        image::load_from_memory(&original).unwrap().to_rgba8()
    );
    let total: u64 = std::fs::read_dir(directory.path())
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "png"))
        .map(|entry| entry.metadata().unwrap().len())
        .sum();
    assert_eq!(entry.bytes, total);
    store.remove(&entry.name).unwrap();
    assert!(store.read_original(&entry.name).is_err());
    assert!(store.entries().unwrap().is_empty());
}

/// 【历史原图回归】【保留策略】无参数，数量淘汰同时清理原图，旧记录仍可编辑，过大事务不影响旧记录
#[test]
fn paired_retention_and_legacy_records_remain_consistent() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path(), 1024 * 1024);
    let png = super::png(16, 9);
    let old = store
        .append_with_original(&png, 1.0, "", 2, Some(&png))
        .unwrap();
    let legacy = store.append(&png, 1.0, "", 1).unwrap();
    assert!(!legacy.has_original);
    assert!(store.read_original(&old.name).is_err());
    assert_eq!(store.entries().unwrap(), vec![legacy.clone()]);
    let tiny = Store::new(directory.path(), 1);
    assert!(
        tiny.append_with_original(&png, 1.0, "", 1, Some(&png))
            .is_err()
    );
    assert!(store.read(&legacy.name).is_ok());
    store.clear().unwrap();
    assert!(store.entries().unwrap().is_empty());
}
