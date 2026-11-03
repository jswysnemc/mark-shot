//! 【截图历史测试】【安全存储】原版保留规则、链接拒绝、权限与并发

use super::super::Store;
use std::{
    fs,
    os::unix::fs::{MetadataExt, symlink},
    sync::{Arc, Barrier},
};

/// 【截图历史测试】【访问器构造】创建临时存储与测试图像；返回临时目录、访问器和 PNG
fn setup(limit: u64) -> (tempfile::TempDir, Store, Vec<u8>) {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join("history"), limit);
    (directory, store, super::png(16, 9))
}

/// 【截图历史测试】【延迟创建】查询、清空和删除不存在记录不创建目录，保存文件使用私有权限
#[test]
fn no_directory_until_successful_append_and_private_permissions() {
    let (_directory, store, png) = setup(1024 * 1024);
    assert!(store.entries().unwrap().is_empty());
    store.clear().unwrap();
    store.prune(1).unwrap();
    store
        .remove("capture-20261103123456123-0123456789abcdef0123456789abcdef.png")
        .unwrap();
    assert!(!store.directory().exists());
    let entry = store.append(&png, 1.2, "transparent", 50).unwrap();
    assert_eq!(
        fs::metadata(store.directory()).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(store.directory().join(&entry.name))
            .unwrap()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        store.entries().unwrap().as_slice(),
        std::slice::from_ref(&entry)
    );
    let (recorded, scale) = store.read(&entry.name).unwrap();
    assert_eq!(scale, 1.2);
    assert_eq!(
        image::load_from_memory(&recorded).unwrap().into_rgba8(),
        image::load_from_memory(&png).unwrap().into_rgba8()
    );
}

/// 【截图历史测试】【数量淘汰】只保留最新 limit 项，文件时间严格递增；零数量与过大新图不损失旧项
#[test]
fn limits_preserve_newest_and_oversized_append_is_non_destructive() {
    let (_directory, store, png) = setup(1024 * 1024);
    let first = store.append(&png, 1.0, "first", 2).unwrap();
    let second = store.append(&png, 1.0, "second", 2).unwrap();
    let third = store.append(&png, 1.0, "third", 2).unwrap();
    assert!(first.created < second.created && second.created < third.created);
    assert_eq!(
        store
            .entries()
            .unwrap()
            .iter()
            .map(|entry| &entry.name)
            .collect::<Vec<_>>(),
        [&third.name, &second.name]
    );
    let small = Store::new(store.directory(), 1);
    assert!(small.append(&png, 1.0, "", 2).is_err());
    assert!(store.append(&png, 1.0, "", 0).is_err());
    assert_eq!(store.entries().unwrap().len(), 2);
    assert!(!store.directory().join(first.name).exists());
}

/// 【截图历史测试】【字节淘汰】总字节限制与数量同时生效，旧项按时间而不是文件大小删除
#[test]
fn byte_limit_evicts_only_oldest_items() {
    let (_directory, store, png) = setup(1024 * 1024);
    let first = store.append(&png, 1.0, "", 50).unwrap();
    let constrained = Store::new(store.directory(), first.bytes * 2);
    let second = constrained.append(&png, 1.0, "", 50).unwrap();
    let third = constrained.append(&png, 1.0, "", 50).unwrap();
    assert_eq!(
        constrained
            .entries()
            .unwrap()
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<Vec<_>>(),
        [third.name, second.name]
    );
}

/// 【截图历史测试】【即时数量】调整保留数量可在没有新截图时淘汰旧项，继续保留最新记录
#[test]
fn explicit_pruning_applies_limit_without_appending() {
    let (_directory, store, png) = setup(1024 * 1024);
    store.append(&png, 1.0, "", 50).unwrap();
    let newest = store.append(&png, 1.0, "", 50).unwrap();
    store.prune(1).unwrap();
    assert_eq!(
        store.entries().unwrap().as_slice(),
        std::slice::from_ref(&newest)
    );
}

/// 【截图历史测试】【链接边界】拒绝路径、链接与硬链接，清空仍删除损坏合法文件并保留无关内容
#[test]
fn links_paths_and_unrelated_files_are_never_owned() {
    let (directory, store, png) = setup(1024 * 1024);
    let entry = store.append(&png, 1.0, "", 50).unwrap();
    let target = directory.path().join("private.png");
    fs::write(&target, &png).unwrap();
    let linked = "capture-20261103123456123-11111111111111111111111111111111.png";
    let hard = "capture-20261103123456124-22222222222222222222222222222222.png";
    let broken = "capture-20261103123456125-33333333333333333333333333333333.png";
    symlink(&target, store.directory().join(linked)).unwrap();
    fs::hard_link(&target, store.directory().join(hard)).unwrap();
    fs::write(store.directory().join(broken), b"broken").unwrap();
    fs::write(store.directory().join("notes.txt"), b"keep").unwrap();
    assert_eq!(store.entries().unwrap().len(), 1);
    for name in [linked, hard, "../private.png", "/etc/passwd", "notes.txt"] {
        assert!(store.read(name).is_err());
        assert!(store.remove(name).is_err());
    }
    store.clear().unwrap();
    assert!(store.entries().unwrap().is_empty());
    assert!(!store.directory().join(entry.name).exists());
    assert!(!store.directory().join(broken).exists());
    assert!(
        fs::symlink_metadata(store.directory().join(linked))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(store.directory().join(hard).exists());
    assert_eq!(fs::read(target).unwrap(), png);
    assert_eq!(
        fs::read(store.directory().join("notes.txt")).unwrap(),
        b"keep"
    );
}

/// 【截图历史测试】【并发记录】并行保存仍严格排序且没有半张文件，最终数量受同一个锁约束
#[test]
fn concurrent_appends_are_serialized_without_collisions() {
    let (_directory, store, png) = setup(1024 * 1024);
    let barrier = Arc::new(Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let (store, png, barrier) = (store.clone(), png.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                store.append(&png, 1.0, "", 5).unwrap()
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let entries = store.entries().unwrap();
    assert_eq!(entries.len(), 5);
    assert!(
        entries
            .windows(2)
            .all(|pair| pair[0].created > pair[1].created)
    );
    assert!(fs::read_dir(store.directory()).unwrap().all(|file| {
        let name = file.unwrap().file_name();
        name == ".history.lock" || name.to_string_lossy().starts_with("capture-")
    }));
}

/// 【截图历史测试】【锁和目录链接】目录或锁被链接替换时拒绝保存；外部目标内容不改变
#[test]
fn linked_directory_and_lock_do_not_redirect_writes() {
    let directory = tempfile::tempdir().unwrap();
    let actual = directory.path().join("actual");
    let alias = directory.path().join("alias");
    fs::create_dir(&actual).unwrap();
    symlink(&actual, &alias).unwrap();
    let png = super::png(1, 1);
    assert!(
        Store::new(&alias, 1024 * 1024)
            .append(&png, 1.0, "", 50)
            .is_err()
    );
    assert_eq!(fs::read_dir(&actual).unwrap().count(), 0);
    let target = directory.path().join("private");
    fs::write(&target, b"keep").unwrap();
    symlink(&target, actual.join(".history.lock")).unwrap();
    assert!(
        Store::new(&actual, 1024 * 1024)
            .append(&png, 1.0, "", 50)
            .is_err()
    );
    assert_eq!(fs::read(target).unwrap(), b"keep");
}

/// 【截图历史测试】【锁等待】其他进程持锁时约一秒返回忙碌；失败不删除已有记录
#[test]
fn contended_lock_has_a_one_second_deadline() {
    let (_directory, store, png) = setup(1024 * 1024);
    let entry = store.append(&png, 1.0, "", 50).unwrap();
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(store.directory().join(".history.lock"))
        .unwrap();
    let _held = nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusive).unwrap();
    let before = std::time::Instant::now();
    assert_eq!(
        store.remove(&entry.name).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(
        (std::time::Duration::from_millis(950)..std::time::Duration::from_secs(3))
            .contains(&before.elapsed())
    );
    assert!(store.directory().join(entry.name).exists());
}

/// 【截图历史测试】【坏图像流】头部仍有效但图像数据损坏时，原图读取必须拒绝，不能另存无效内容
#[test]
fn invalid_compressed_pixels_cannot_be_read_for_actions() {
    let (_directory, store, png) = setup(1024 * 1024);
    let entry = store.append(&png, 1.0, "", 50).unwrap();
    let path = store.directory().join(&entry.name);
    let mut bytes = fs::read(&path).unwrap();
    let mut offset = 8;
    while offset < bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        if &bytes[offset + 4..offset + 8] == b"IDAT" {
            bytes[offset + 8] = 0;
            let checksum = crc32fast::hash(&bytes[offset + 4..offset + 8 + length]);
            bytes[offset + 8 + length..offset + 12 + length]
                .copy_from_slice(&checksum.to_be_bytes());
            break;
        }
        offset += length + 12;
    }
    fs::write(path, bytes).unwrap();
    assert!(store.read(&entry.name).is_err());
}
