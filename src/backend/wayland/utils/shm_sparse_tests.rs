//! 【共享前景测试】【物理页与像素】用真实共享文件验证透明回收、边界保留和通道转换

use super::*;

/// 【共享前景测试】【原生介质】创建与Wayland槽位相同的匿名共享内存，返回文件，不依赖临时目录文件系统的打洞支持
fn shared_file() -> std::fs::File {
    nix::sys::memfd::memfd_create(
        c"mark-shot-shm-test",
        nix::sys::memfd::MFdFlags::MFD_CLOEXEC,
    )
    .unwrap()
    .into()
}

/// 【共享前景测试】【原生驻留】按bytes读取当前共享映射驻留页，返回页数，不改动映射像素
#[cfg(target_os = "linux")]
fn resident(bytes: &[u8]) -> usize {
    let mut pages = vec![0; bytes.len().div_ceil(4096)];
    let result = unsafe {
        nix::libc::mincore(
            bytes.as_ptr().cast_mut().cast(),
            bytes.len(),
            pages.as_mut_ptr(),
        )
    };
    assert_eq!(result, 0);
    pages.iter().filter(|value| **value & 1 != 0).count()
}

/// 【共享前景测试】【整页与边缘】无参数，真实共享文件写入稀疏前景后保留所有像素及相邻区域，并减少物理驻留，无返回值
#[test]
fn sparse_shared_copy_preserves_pixels_and_releases_only_complete_zero_pages() {
    let file = shared_file();
    let length = 4 * 1024 * 1024;
    file.set_len(length as u64).unwrap();
    let mut map = unsafe {
        memmap2::MmapOptions::new()
            .len(length)
            .map_mut(&file)
            .unwrap()
    };
    map.fill(217);
    let mut source = vec![0; length - 40];
    for offset in [0, 34191, 91833, source.len() - 1] {
        source[offset] = 91;
    }
    copy(&mut map[20..length - 20], &source, false);
    #[cfg(target_os = "linux")]
    assert!(
        resident(&map) < 10,
        "transparent shared pages were materialized"
    );
    assert_eq!(&map[..20], &[217; 20]);
    assert_eq!(&map[length - 20..], &[217; 20]);
    assert_eq!(&map[20..length - 20], source.as_slice());
}

/// 【共享前景测试】【通道与损坏】无参数，跨页损坏只修改请求字节，RGBA到BGRA结果与既有复制逐字节相同，无返回值
#[test]
fn swizzled_sparse_copy_matches_dense_copy_and_preserves_outside_damage() {
    let file = shared_file();
    let length = 256 * 1024;
    file.set_len(length as u64).unwrap();
    let mut map = unsafe {
        memmap2::MmapOptions::new()
            .len(length)
            .map_mut(&file)
            .unwrap()
    };
    map.fill(93);
    let mut source = vec![0; length - 56];
    for offset in [0, 43192, 72340, source.len() - 4] {
        source[offset..offset + 4].copy_from_slice(&[11, 20, 90, 120]);
    }
    let mut expected = vec![93; length];
    crate::utils::copy_swizzled(&mut expected[28..length - 28], &source);
    copy(&mut map[28..length - 28], &source, true);
    assert_eq!(&*map, expected.as_slice());
}
