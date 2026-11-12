//! 【前景缓存】【局部透明页】仅检查损坏矩形相交的自有页，保留邻近非零像素和完整栅格

use tiny_skia::{IntRect, Pixmap};

/// 【前景缓存】【矩形透明回收】按pixmap和region归还全零页，返回值为空；相交整页存在任何非零字节时保留，范围外页面不扫描
pub(crate) fn reclaim_region(pixmap: &mut Pixmap, region: IntRect) {
    let bounds = IntRect::from_xywh(0, 0, pixmap.width(), pixmap.height()).unwrap();
    let Some(region) = region.intersect(&bounds) else {
        return;
    };
    let stride = pixmap.width() as usize * 4;
    let width = region.width() as usize * 4;
    let bytes = pixmap.data_mut();
    #[cfg(target_os = "linux")]
    {
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
        if let Ok(page) = usize::try_from(page)
            && page.is_power_of_two()
            && page >= 64
        {
            let address = bytes.as_ptr() as usize;
            let first = region.top() as usize * stride + region.left() as usize * 4;
            let last = (region.bottom() as usize - 1) * stride + region.right() as usize * 4;
            if let Some(snapshot) =
                super::page_residency::Snapshot::read(address + first..address + last, page)
            {
                reclaim_private(bytes, region, stride, width, page, &snapshot);
                return;
            }
        }
    }
    // 1. 【前景缓存】【完整行范围】覆盖整行时合并连续区间，避免对同一页重复检查
    if width == stride {
        super::reclaim_blank_range(
            bytes,
            region.top() as usize * stride..region.bottom() as usize * stride,
        );
        return;
    }
    // 2. 【前景缓存】【相交页验证】窄片段仅检查每行触及的完整页，页内其他笔迹仍参与非零校验
    for row in region.top()..region.bottom() {
        let start = row as usize * stride + region.left() as usize * 4;
        super::reclaim_blank_range(bytes, start..start + width);
    }
}

/// 【前景缓存】【独占透明页】按 bytes、region、stride、width、page 与 snapshot 回收实际写入过的全零相交页，无返回值
#[cfg(target_os = "linux")]
fn reclaim_private(
    bytes: &mut [u8],
    region: IntRect,
    stride: usize,
    width: usize,
    page: usize,
    snapshot: &super::page_residency::Snapshot,
) {
    let address = bytes.as_ptr() as usize;
    let owned_first = address.div_ceil(page) * page;
    let owned_last = (address + bytes.len()) / page * page;
    let mut checked_until = owned_first;
    let mut run: Option<std::ops::Range<usize>> = None;
    // 1. 【前景缓存】【相交页集合】按地址顺序检查实际相交页，跳过行边界重复页和已经共享的零页
    for row in region.top()..region.bottom() {
        let start = address + row as usize * stride + region.left() as usize * 4;
        let first = (start / page * page).max(owned_first);
        let last = (start + width)
            .div_ceil(page)
            .saturating_mul(page)
            .min(owned_last);
        for position in (first..last).step_by(page) {
            if position < checked_until {
                continue;
            }
            checked_until = position + page;
            let offset = position - address;
            let blank = snapshot.exclusive(position)
                && bytes[offset..offset + page]
                    .as_chunks::<64>()
                    .0
                    .iter()
                    .all(|block| *block == [0; 64]);
            if blank {
                if let Some(run) = &mut run
                    && run.end == offset
                {
                    run.end += page;
                } else {
                    release(bytes, run.take());
                    run = Some(offset..offset + page);
                }
            } else {
                release(bytes, run.take());
            }
        }
    }
    release(bytes, run);
}

/// 【前景缓存】【连续归还】按 bytes 与 range 归还已经验证全零的独占页，空范围或原生失败时保持原字节，无返回值
#[cfg(target_os = "linux")]
fn release(bytes: &mut [u8], range: Option<std::ops::Range<usize>>) {
    if let Some(range) = range {
        // 2. 【前景缓存】【所有权边界】范围只包含 Vec 内部完整相交页，内容已验证为零，没有外部像素借用
        unsafe {
            nix::libc::madvise(
                bytes.as_mut_ptr().add(range.start).cast(),
                range.len(),
                nix::libc::MADV_DONTNEED,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【局部透明页测试】【共享零页保留】只读透明页不占独立像素内存，回收不能取消其映射并导致下一帧重复缺页
    #[cfg(target_os = "linux")]
    #[test]
    fn regional_reclamation_keeps_already_shared_zero_pages_mapped() {
        use std::os::unix::fs::FileExt;
        let mut pixmap = Pixmap::new(2048, 257).unwrap();
        crate::utils::transparent_pixmap::prepare_sparse(&mut pixmap);
        crate::utils::transparent_pixmap::clear(&mut pixmap);
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;
        let address = pixmap.data().as_ptr() as usize;
        let offset = 31 * 2048 * 4 + 512 * 4;
        assert_eq!(std::hint::black_box(pixmap.data()[offset]), 0);
        let entries = std::fs::File::open("/proc/self/pagemap").unwrap();
        let flags = || {
            let mut value = [0; 8];
            entries
                .read_exact_at(&mut value, (((address + offset) / page) * 8) as u64)
                .unwrap();
            u64::from_ne_bytes(value)
        };
        assert_ne!(flags() & (1 << 63), 0);
        assert_eq!(flags() & (1 << 56), 0);
        reclaim_region(&mut pixmap, IntRect::from_xywh(512, 31, 127, 19).unwrap());
        assert_ne!(
            flags() & (1 << 63),
            0,
            "shared zero page was unnecessarily unmapped"
        );
        assert_eq!(flags() & (1 << 56), 0);
    }

    /// 【局部透明页测试】【边界笔迹】无参数，验证负边界、小宽度、跨页片段及矩形外笔迹全部保持原值，无返回值
    #[test]
    fn clipped_regions_keep_nonzero_pixels_on_intersecting_and_unrelated_pages() {
        for size in [(7, 9), (513, 257), (2048, 257)] {
            let mut pixmap = Pixmap::new(size.0, size.1).unwrap();
            for offset in [0, pixmap.data().len() / 2, pixmap.data().len() - 1] {
                pixmap.data_mut()[offset] = 91;
            }
            let expected = pixmap.data().to_vec();
            for region in [
                IntRect::from_xywh(-3, -3, 33, 33).unwrap(),
                IntRect::from_xywh(13, 17, 299, 101).unwrap(),
                IntRect::from_xywh(0, 0, size.0, size.1).unwrap(),
            ] {
                reclaim_region(&mut pixmap, region);
                assert_eq!(pixmap.data(), expected);
            }
        }
    }

    /// 【局部透明页测试】【真实归还】无参数，验证仅归还相交全零自有页，矩形外独占页保留；只读页标志，不读取物理地址，无返回值
    #[cfg(target_os = "linux")]
    #[test]
    fn regional_reclamation_returns_zero_pages_without_scanning_unrelated_rows() {
        use std::io::{Read, Seek, SeekFrom};
        let mut pixmap = Pixmap::new(2048, 257).unwrap();
        crate::utils::transparent_pixmap::prepare_sparse(&mut pixmap);
        pixmap.data_mut().fill(0);
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;
        let address = pixmap.data().as_ptr() as usize;
        let stride = pixmap.width() as usize * 4;
        let selected = (address + 31 * stride + 512 * 4) / page;
        let unrelated = (address + 200 * stride + 512 * 4) / page;
        let mut entries = std::fs::File::open("/proc/self/pagemap").unwrap();
        let mut owned = |index: usize| {
            entries.seek(SeekFrom::Start((index * 8) as u64)).unwrap();
            let mut flags = [0; 8];
            entries.read_exact(&mut flags).unwrap();
            let flags = u64::from_ne_bytes(flags);
            flags & (1 << 63) != 0 && flags & (1 << 56) != 0
        };
        assert!(owned(selected) && owned(unrelated));
        reclaim_region(&mut pixmap, IntRect::from_xywh(512, 31, 127, 19).unwrap());
        assert!(!owned(selected));
        assert!(owned(unrelated));
        assert!(pixmap.data().iter().all(|byte| *byte == 0));
    }
}
