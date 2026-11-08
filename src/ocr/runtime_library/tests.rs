use super::{install, manifest::Distribution};
use crate::ocr::models::Asset;
use std::{fs, io::Write, sync::atomic::AtomicBool};

const CONTENT: &[u8] = b"abc";
const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

/// 【识别运行库】【测试清单】以 archive 指定格式，返回两个固定摘要的小文件清单
fn distribution(archive: &'static str) -> Distribution {
    Distribution {
        archive: Asset {
            file: archive,
            size: 0,
            sha256: HASH,
        },
        library: Asset {
            file: "runtime.dll",
            size: 3,
            sha256: HASH,
        },
        shared: Asset {
            file: "shared.dll",
            size: 3,
            sha256: HASH,
        },
        platform: "test",
    }
}

/// 【识别运行库】【测试归档】把 entries 写到 path，返回 ZIP 文件，无网络访问
fn archive(path: &std::path::Path, entries: &[(&str, &[u8])]) {
    let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, data) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
}

#[test]
fn extracts_only_verified_libraries_and_notices() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let path = root.path().join("runtime.zip");
    archive(
        &path,
        &[
            ("package/lib/runtime.dll", CONTENT),
            ("package/lib/shared.dll", CONTENT),
            ("package/LICENSE", CONTENT),
            ("package/unwanted.exe", CONTENT),
        ],
    );
    install::extract(
        &path,
        dest.path(),
        distribution("runtime.zip"),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(fs::read_dir(dest.path()).unwrap().count(), 3);
    assert!(!dest.path().join("unwanted.exe").exists());
    fs::write(dest.path().join("runtime.dll"), b"xyz").unwrap();
    assert!(install::verify(dest.path(), distribution("runtime.zip")).is_err());
}

#[test]
fn rejects_traversal_duplicates_missing_and_wrong_content() {
    for entries in [
        vec![("../runtime.dll", CONTENT)],
        vec![("a/runtime.dll", CONTENT), ("b/runtime.dll", CONTENT)],
        vec![("runtime.dll", CONTENT)],
        vec![("runtime.dll", b"xyz".as_slice()), ("shared.dll", CONTENT)],
        vec![
            ("runtime.dll", b"oversized".as_slice()),
            ("shared.dll", CONTENT),
        ],
    ] {
        let root = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let path = root.path().join("runtime.zip");
        archive(&path, &entries);
        assert!(
            install::extract(
                &path,
                dest.path(),
                distribution("runtime.zip"),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}

#[test]
fn cancelled_extraction_does_not_create_library() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let path = root.path().join("runtime.zip");
    archive(&path, &[("runtime.dll", CONTENT), ("shared.dll", CONTENT)]);
    assert!(
        install::extract(
            &path,
            dest.path(),
            distribution("runtime.zip"),
            &AtomicBool::new(true)
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(dest.path()).unwrap().count(), 0);
}

#[test]
fn extracts_regular_tar_files_without_following_symlinks() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let path = root.path().join("runtime.tgz");
    let mut writer = tar::Builder::new(flate2::write::GzEncoder::new(
        fs::File::create(&path).unwrap(),
        flate2::Compression::default(),
    ));
    for name in ["package/lib/runtime.dll", "package/lib/shared.dll"] {
        let mut header = tar::Header::new_gnu();
        header.set_size(CONTENT.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        writer.append_data(&mut header, name, CONTENT).unwrap();
    }
    let mut link = tar::Header::new_gnu();
    link.set_entry_type(tar::EntryType::Symlink);
    link.set_size(0);
    writer
        .append_link(&mut link, "package/lib/escape", "/tmp/escape")
        .unwrap();
    writer.into_inner().unwrap().finish().unwrap();
    install::extract(
        &path,
        dest.path(),
        distribution("runtime.tgz"),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(fs::read_dir(dest.path()).unwrap().count(), 2);
}
