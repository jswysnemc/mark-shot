//! 【截图另存为测试】【门户与落盘】验证快照共享、门户 URI 解析、原子落盘及符号链接防护

use super::{Snapshot, portal, writer};
use std::{path::PathBuf, sync::Arc};

/// 【截图另存为测试】【图像数据】返回含透明像素的 PNG 原始字节，无外部文件或模型依赖
fn png() -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(17, 9).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(200, 100, 50, 128));
    crate::utils::encode_png_with_alpha(pixmap)
}

/// 【截图另存为测试】【共享原件】克隆快照只共享 PNG 字节，不重编码、缩放或丢失透明度
#[test]
fn snapshot_clones_share_the_exact_original_png() {
    let data = png();
    let snapshot = Snapshot::new(data.clone(), 1.25, "search".into()).unwrap();
    let copy = snapshot.clone();
    assert!(Arc::ptr_eq(&snapshot.png, &copy.png));
    assert_eq!(*copy.png, data);
    assert_eq!(copy.scale, 1.25);
    assert_eq!(copy.search, "search");
    assert!(Snapshot::new(b"broken".to_vec(), 1.0, String::new()).is_err());
    assert!(Snapshot::new(png(), f32::NAN, String::new()).is_err());
}

/// 【截图另存为测试】【门户URI解码】支持空格与中文编码路径并自动补齐缺失的 png 扩展名
#[test]
fn portal_uri_decoding_handles_spaces_unicode_and_default_extension() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("截图 样本");
    let uri = url::Url::from_file_path(&target).unwrap();
    let parsed = portal::parse_selected_uri(uri.as_str()).unwrap();
    assert_eq!(parsed, dir.path().join("截图 样本.png"));

    let explicit = dir.path().join("custom.png");
    let explicit_uri = url::Url::from_file_path(&explicit).unwrap();
    assert_eq!(
        portal::parse_selected_uri(explicit_uri.as_str()).unwrap(),
        explicit
    );

    assert!(portal::parse_selected_uri("https://example.com/a.png").is_err());

    let fuse_doc = PathBuf::from("/run/user/1000/doc/8f9a1b2c/raw_name");
    assert_eq!(portal::normalize_selected_path(fuse_doc.clone()), fuse_doc);
}

/// 【截图另存为测试】【原子保存与覆盖】正常目录可新建或覆盖普通文件，缺失父目录返回错误
#[test]
fn writer_saves_and_overwrites_regular_files_while_reporting_missing_parent() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("image.png");
    let first = png();
    writer::write_png(&first, &path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), first);

    let mut second_pixmap = tiny_skia::Pixmap::new(5, 5).unwrap();
    second_pixmap.fill(tiny_skia::Color::from_rgba8(10, 200, 120, 255));
    let second = crate::utils::encode_png_with_alpha(second_pixmap);
    writer::write_png(&second, &path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), second);

    let missing = directory.path().join("missing/image.png");
    assert!(writer::write_png(&first, &missing).is_err());
    assert!(!missing.exists());
}

/// 【截图另存为测试】【符号链接拦截】保存目标为符号链接时拒绝写入，保护外部原文件不被篡改
#[test]
fn linked_targets_are_rejected_without_modifying_external_file() {
    let directory = tempfile::tempdir().unwrap();
    let external = directory.path().join("external.txt");
    let link = directory.path().join("image.png");
    std::fs::write(&external, b"protected").unwrap();
    crate::utils::test_files::link_file(&external, &link).unwrap();
    assert!(writer::write_png(&png(), &link).is_err());
    assert_eq!(std::fs::read(&external).unwrap(), b"protected");
}
