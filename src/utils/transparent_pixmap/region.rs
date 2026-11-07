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

#[cfg(test)]
mod tests {
    use super::*;

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
