//! 【原生前景测试】【消费边界】使用完整预乘像素与跨页局部更新验证字节交付契约

use super::*;

/// 【原生前景测试】【像素构造】按width和height返回跨通道、跨页且非透明的真实预乘源图
fn foreground(width: u32, height: u32) -> Pixmap {
    let mut pixmap = Pixmap::new(width, height).unwrap();
    for (index, pixel) in pixmap.pixels_mut().iter_mut().enumerate() {
        *pixel =
            tiny_skia::ColorU8::from_rgba(index as u8, (index >> 2) as u8, (index >> 5) as u8, 137)
                .premultiply();
    }
    pixmap
}

/// 【原生前景测试】【借用保留】无参数，只读交付保持CPU全部内容且通道转换准确，无返回值
#[test]
fn borrowed_foreground_keeps_all_source_pixels() {
    let source = foreground(513, 257);
    let expected = source.clone();
    let mut native = vec![0; source.data().len()];
    FramePixels::Borrowed(source.data())
        .write_to(&mut native, (513, 257), None)
        .unwrap();
    assert_eq!(source.data(), expected.data());
    let mut swizzled = vec![0; native.len()];
    crate::utils::copy_swizzled(&mut swizzled, expected.data());
    assert_eq!(native, swizzled);
}

/// 【原生前景测试】【完整消费】无参数，交付后原生字节完整准确、CPU恢复零值且地址可重用，无返回值
#[test]
fn consumed_whole_frame_releases_source_only_after_copying_each_batch() {
    let mut source = foreground(1100, 1100);
    let address = source.data().as_ptr();
    let mut expected = vec![0; source.data().len()];
    crate::utils::copy_swizzled(&mut expected, source.data());
    let mut native = vec![0; expected.len()];
    FramePixels::Consumed(&mut source)
        .write_to(&mut native, (1100, 1100), None)
        .unwrap();
    assert_eq!(native, expected);
    assert_eq!(source.data().as_ptr(), address);
    assert!(
        source.data().iter().all(|byte| *byte == 0),
        "CPU foreground was retained after explicit consumption"
    );
}

/// 【原生前景测试】【局部消费】无参数，局部交付只消费已发送字节，原生和CPU范围外内容都保持不变，无返回值
#[test]
fn consumed_damage_preserves_all_unsubmitted_and_native_outside_pixels() {
    let mut source = foreground(513, 257);
    let original = source.clone();
    let damage = (19, 11, 383, 211);
    let mut native = vec![71; source.data().len()];
    FramePixels::Consumed(&mut source)
        .write_to(&mut native, (513, 257), Some(damage))
        .unwrap();
    for (index, (cpu, output)) in source
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(native.as_chunks::<4>().0)
        .enumerate()
    {
        let (x, y) = (index as u32 % 513, index as u32 / 513);
        let before = &original.data()[index * 4..index * 4 + 4];
        if (19..402).contains(&x) && (11..222).contains(&y) {
            assert_eq!(*cpu, [0; 4], "submitted source pixel retained");
            assert_eq!(*output, [before[2], before[1], before[0], before[3]]);
        } else {
            assert_eq!(cpu.as_slice(), before);
            assert_eq!(*output, [71; 4]);
        }
    }
}

/// 【原生前景测试】【自有驻留】按pixmap读取内部完整页的present/exclusive标志，返回实际自有驻留页数，不读取或输出物理地址
#[cfg(target_os = "linux")]
fn owned_pages(pixmap: &Pixmap) -> usize {
    use std::io::{Read, Seek, SeekFrom};
    let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;
    let start = pixmap.data().as_ptr() as usize;
    let first = start.div_ceil(page);
    let last = (start + pixmap.data().len()) / page;
    let mut file = std::fs::File::open("/proc/self/pagemap").unwrap();
    file.seek(SeekFrom::Start(first as u64 * 8)).unwrap();
    let mut count = 0;
    for _ in first..last {
        let mut entry = [0; 8];
        file.read_exact(&mut entry).unwrap();
        let flags = u64::from_ne_bytes(entry);
        if flags & (1 << 63 | 1 << 56) == (1 << 63 | 1 << 56) {
            count += 1;
        }
    }
    count
}

/// 【原生前景测试】【真实匿名归还】无参数，消费完整真实前景后其自有匿名驻留页应归还，源容量和原生所有像素仍保持有效，无返回值
#[cfg(target_os = "linux")]
#[test]
fn consumed_source_releases_owned_anonymous_pages_without_changing_native_pixels() {
    let mut source = foreground(1600, 1200);
    let before = owned_pages(&source);
    let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;
    assert!(before > source.data().len() / page * 9 / 10);
    let mut native = vec![0; source.data().len()];
    FramePixels::Consumed(&mut source)
        .write_to(&mut native, (1600, 1200), None)
        .unwrap();
    assert!(
        owned_pages(&source) < 8,
        "consumed anonymous source still owns complete physical pages"
    );
    for (index, pixel) in native.as_chunks::<4>().0.iter().enumerate() {
        let expected =
            tiny_skia::ColorU8::from_rgba(index as u8, (index >> 2) as u8, (index >> 5) as u8, 137)
                .premultiply();
        assert_eq!(
            *pixel,
            [
                expected.blue(),
                expected.green(),
                expected.red(),
                expected.alpha()
            ]
        );
    }
}

/// 【原生前景测试】【稀疏局部归还】无参数，真实窄扫描行交付后，全零边界页必须归还且原生范围外像素不变，无返回值
#[cfg(target_os = "linux")]
#[test]
fn consumed_sparse_damage_releases_zero_boundary_pages_after_small_row_copies() {
    let mut source = Pixmap::new(2000, 480).unwrap();
    crate::utils::transparent_pixmap::clear(&mut source);
    let damage = (57, 20, 7, 400);
    for row in 20..420 {
        let first = (row * 2000 + 57) * 4;
        source.data_mut()[first..first + 28].fill(7);
    }
    assert!(owned_pages(&source) > 100);
    let mut native = vec![71; source.data().len()];
    FramePixels::Consumed(&mut source)
        .write_to(&mut native, (2000, 480), Some(damage))
        .unwrap();
    let remaining = owned_pages(&source);
    assert!(
        remaining < 4,
        "zero boundary pages remain resident after sparse consumption: {remaining}, source nonzero bytes={}",
        source.data().iter().filter(|byte| **byte != 0).count()
    );
    for (index, pixel) in native.as_chunks::<4>().0.iter().enumerate() {
        let (x, y) = (index % 2000, index / 2000);
        assert_eq!(
            *pixel,
            if (57..64).contains(&x) && (20..420).contains(&y) {
                [7; 4]
            } else {
                [71; 4]
            }
        );
    }
}
