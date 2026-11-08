//! 【图像原件回归】【读取回收】大图读取结束后释放原件页，重新读取必须保留全部原始像素

use super::ImageSource;
use tiny_skia::{Color, Pixmap};

/// 【图像原件回归】【磁盘目录】无参数，在项目构建目录创建私有测试目录，避免系统临时目录使用内存文件系统，返回所有者
fn directory() -> tempfile::TempDir {
    let root = std::env::current_dir()
        .unwrap()
        .join("target/image-source-tests");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// 【图像原件回归】【透明纹理】按width和height生成包含全部透明度的独立纹理，返回预乘原件，期望图不读取缓存
fn varied(width: u32, height: u32) -> Pixmap {
    let mut image = Pixmap::new(width, height).unwrap();
    for (index, pixel) in image.pixels_mut().iter_mut().enumerate() {
        let hash = (index as u32).wrapping_mul(2_654_435_761).rotate_left(7);
        *pixel = tiny_skia::ColorU8::from_rgba(
            hash as u8,
            (hash >> 8) as u8,
            (hash >> 16) as u8,
            index as u8,
        )
        .premultiply();
    }
    image
}

/// 【图像原件回归】【固定原件】按width和height返回含透明度的固定原件，不使用缓存内容生成期望值
fn pixels(width: u32, height: u32) -> Pixmap {
    let mut image = Pixmap::new(width, height).unwrap();
    image.fill(Color::from_rgba8(29, 73, 131, 137));
    image
}

/// 【图像原件回归】【映射常驻页】按address查找当前进程实际映射，返回该映射RSS的KiB，未找到时失败
#[cfg(target_os = "linux")]
fn resident_kib(address: usize) -> usize {
    let maps = std::fs::read_to_string("/proc/self/smaps").unwrap();
    let mut current = false;
    for line in maps.lines() {
        let columns: Vec<_> = line.split_whitespace().collect();
        if let Some((start, end)) = columns.first().and_then(|column| column.split_once('-')) {
            let start = usize::from_str_radix(start, 16).unwrap();
            let end = usize::from_str_radix(end, 16).unwrap();
            current = start <= address && address < end;
        } else if current && columns.first() == Some(&"Rss:") {
            return columns[1].parse().unwrap();
        }
    }
    panic!("the image source mapping was not found");
}

/// 【图像原件回归】【文件驻留页】按address与length读取内核实际驻留页，返回KiB，不以客户端RSS代替文件缓存占用
#[cfg(target_os = "linux")]
fn file_pages_kib(address: usize, length: usize) -> usize {
    // 1. 【图像原件回归】【内核页查询】只查询仍由来源持有的有效映射，结果数组覆盖每个系统页，不修改像素
    let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;
    let start = address / page * page;
    let bytes = length + address - start;
    let mut flags = vec![0_u8; bytes.div_ceil(page)];
    let result =
        unsafe { nix::libc::mincore(start as *mut nix::libc::c_void, bytes, flags.as_mut_ptr()) };
    assert_eq!(result, 0, "{}", std::io::Error::last_os_error());
    flags.iter().filter(|value| **value & 1 != 0).count() * page / 1024
}

/// 【图像原件回归】【大图释放】无参数，真实读取大原件后常驻页不得超过4MiB，读取页回收不能改变透明度或颜色，无返回值
#[test]
#[cfg(target_os = "linux")]
fn large_source_releases_read_pages_without_losing_pixels() {
    let directory = directory();
    let source = ImageSource::store_in(pixels(3000, 4000), directory.path());
    let expected = tiny_skia::ColorU8::from_rgba(29, 73, 131, 137).premultiply();
    let expected = [
        expected.red(),
        expected.green(),
        expected.blue(),
        expected.alpha(),
    ];
    for pixel in source.data().as_chunks::<4>().0 {
        assert_eq!(*pixel, expected);
    }
    let address = source.data().as_ptr() as usize;
    source.release_pages();
    let remaining = resident_kib(address);
    println!("source_resident_kib={remaining}, limit_kib=4096");
    assert!(
        remaining <= 4096,
        "read-only original still retains {remaining} KiB"
    );
    let backing = file_pages_kib(address, source.data().len());
    println!("source_backing_resident_kib={backing}, limit_kib=4096");
    assert!(
        backing <= 4096,
        "original pixels stayed resident in the file cache: {backing} KiB"
    );
    assert_eq!(&source.data()[..4], &expected);
    assert_eq!(&source.data()[source.data().len() - 4..], &expected);
}

/// 【图像原件回归】【重读与裁切】无参数，回收后的完整像素与跨边界裁切必须和原件一致，缓存所有者结束后清理私有目录，无返回值
#[test]
fn cached_source_preserves_all_alpha_values_and_releases_its_files() {
    let directory = directory();
    let original = varied(1100, 1100);
    let expected = original.clone();
    let source = ImageSource::store_in(original, directory.path());
    assert_eq!(source.cached_bytes(), expected.data().len());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    source.release_pages();
    assert_eq!(source.data(), expected.data());
    for rect in [
        tiny_skia::IntRect::from_xywh(-7, -13, 60, 70).unwrap(),
        tiny_skia::IntRect::from_xywh(923, 980, 360, 140).unwrap(),
        tiny_skia::IntRect::from_xywh(0, 0, 1100, 1100).unwrap(),
    ] {
        let actual = source.clone_rect(rect).unwrap();
        let expected = expected.clone_rect(rect).unwrap();
        assert_eq!(actual.data(), expected.data());
        assert_eq!(
            (actual.width(), actual.height()),
            (expected.width(), expected.height())
        );
    }
    drop(source);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

/// 【图像原件回归】【缓存失败】无参数，不可用目录必须保留原始颜色、透明度与裁切能力，无返回值
#[test]
fn cache_failure_keeps_the_original_pixels_available() {
    let directory = directory();
    let invalid = directory.path().join("not-a-directory");
    std::fs::write(&invalid, "occupied").unwrap();
    let original = varied(1100, 1100);
    let expected = original.clone();
    let source = ImageSource::store_in(original, &invalid);
    assert_eq!(source.cached_bytes(), 0);
    source.release_pages();
    assert_eq!(source.data(), expected.data());
}

/// 【图像原件回归】【区域新版本】无参数，写回与恢复必须只修改相交像素，新版本替换后不保留旧缓存目录，无返回值
#[test]
fn cached_edits_preserve_unrelated_pixels_and_restore_exactly() {
    let directory = directory();
    let original = varied(1100, 1100);
    let baseline = original.clone();
    let mut expected = original.clone();
    let mut source = ImageSource::store_in(original, directory.path());
    let patch = pixels(20, 30);
    source.replace_pixels(&patch, (1093, 1095));
    crate::renderer::paste(&mut expected, &patch, 1093, 1095);
    source.release_pages();
    assert_eq!(source.data(), expected.data());
    assert_eq!(source.cached_bytes(), baseline.data().len());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    let restore = baseline
        .clone_rect(tiny_skia::IntRect::from_xywh(1093, 1095, 7, 5).unwrap())
        .unwrap();
    source.replace_pixels(&restore, (1093, 1095));
    source.release_pages();
    assert_eq!(source.data(), baseline.data());
    drop(source);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

/// 【图像原件回归】【修改失败回退】无参数，缓存目录失效后仍完成区域写入，旧原件与区域外像素继续完整保留，无返回值
#[test]
fn failed_cached_edit_keeps_a_complete_editable_original() {
    let directory = directory();
    let root = directory.path().join("cache");
    let original = varied(1100, 1100);
    let mut expected = original.clone();
    let mut source = ImageSource::store_in(original, &root);
    assert!(source.cached_bytes() > 0);
    std::fs::remove_dir_all(&root).unwrap();
    let patch = pixels(20, 30);
    source.replace_pixels(&patch, (100, 100));
    crate::renderer::paste(&mut expected, &patch, 100, 100);
    assert_eq!(source.cached_bytes(), 0);
    assert_eq!(source.data(), expected.data());
}

/// 【图像原件回归】【内存介质拒绝】无参数，当前系统临时目录为内存介质时拒绝把原件转入该缓存，并保留完整像素，无返回值
#[test]
#[cfg(target_os = "linux")]
fn memory_backed_cache_cannot_report_a_disk_original() {
    use std::os::fd::AsRawFd;
    let directory = tempfile::tempdir().unwrap();
    let file = tempfile::tempfile_in(directory.path()).unwrap();
    let mut status = std::mem::MaybeUninit::<nix::libc::statfs>::uninit();
    let result = unsafe { nix::libc::fstatfs(file.as_raw_fd(), status.as_mut_ptr()) };
    assert_eq!(result, 0);
    let status = unsafe { status.assume_init() };
    if !super::cache_file::MEMORY_FILESYSTEMS.contains(&(status.f_type as u32)) {
        return;
    }
    let source = ImageSource::store_in(pixels(1100, 1100), directory.path());
    assert_eq!(source.cached_bytes(), 0);
    assert_eq!(source.data(), pixels(1100, 1100).data());
}

/// 【图像原件回归】【预览栅格】无参数，分段预览必须与完整原件双线性绘制逐字节一致，覆盖透明度、分数缩放及非整数平移，无返回值
#[test]
fn bounded_preview_preserves_full_raster_pixels() {
    let directory = directory();
    let original = varied(1100, 2200);
    let source = ImageSource::store_in(original.clone(), directory.path());
    let paint = tiny_skia::PixmapPaint {
        quality: tiny_skia::FilterQuality::Bilinear,
        ..tiny_skia::PixmapPaint::default()
    };
    for (scale, x, y) in [
        (1.0, 0.0, 0.0),
        (0.49, 653.5, 127.25),
        (0.21752347, 388.1137, -0.8123),
        (1.2, -17.5, 23.75),
        (1.5, 80.0, -23.0),
        (2.3, 11.2345, 30.5678),
    ] {
        let transform = tiny_skia::Transform::from_scale(scale, scale).post_translate(x, y);
        let mut expected = Pixmap::new(2345, 1719).unwrap();
        expected.fill(Color::from_rgba8(28, 31, 34, 255));
        expected.draw_pixmap(0, 0, original.as_ref(), &paint, transform, None);
        let mut actual = Pixmap::new(2345, 1719).unwrap();
        actual.fill(Color::from_rgba8(28, 31, 34, 255));
        source.draw_preview(&mut actual, &paint, transform);
        let differences = actual
            .pixels()
            .iter()
            .zip(expected.pixels())
            .filter(|(first, second)| first != second)
            .count();
        assert_eq!(
            differences, 0,
            "preview raster differs for {scale}, {x}, {y}"
        );
    }
}
/// 【冻结截图测试】【热路径所有权】无参数，四千像素背景复用原分配且不进入文件缓存，读取与归还后像素不变，无返回值
#[test]
fn frozen_screen_reuses_memory_without_disk_roundtrips() {
    let mut pixels = tiny_skia::Pixmap::new(3840, 2160).unwrap();
    pixels.fill(tiny_skia::Color::from_rgba8(91, 127, 23, 255));
    let address = pixels.data().as_ptr();
    let source = super::ImageSource::store_snapshot(pixels);
    assert_eq!(source.cached_bytes(), 0);
    assert_eq!(source.data().as_ptr(), address);
    source.release_pages();
    assert_eq!(source.pixel(3839, 2159).unwrap().red(), 91);
}
