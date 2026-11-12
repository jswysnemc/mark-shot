//! 【标注缓存】【透明页】完整清空自有像素时归还匿名页，保留原始画布坐标与栅格范围

use tiny_skia::Pixmap;

#[cfg(target_os = "linux")]
mod page_residency;
mod region;
pub(crate) use region::reclaim_region;
mod copy;
pub(crate) use copy::copy_changed;

/// 【前景缓存】【稀疏页策略】按pixmap限制自有内部页的透明大页分配，无返回值，不改像素；避免少量扫描行写入占有整块大页
pub(crate) fn prepare_sparse(pixmap: &mut Pixmap) {
    #[cfg(target_os = "linux")]
    {
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
        if let Ok(page) = usize::try_from(page)
            && page.is_power_of_two()
        {
            let bytes = pixmap.data_mut();
            let address = bytes.as_mut_ptr() as usize;
            let leading = (page - address % page) % page;
            if leading < bytes.len() {
                let length = (bytes.len() - leading) / page * page;
                if length != 0 {
                    // 1. 【前景缓存】【自有映射】只改变完整自有页的映射提示，不修改系统策略或相邻分配
                    unsafe {
                        nix::libc::madvise(
                            bytes.as_mut_ptr().add(leading).cast(),
                            length,
                            nix::libc::MADV_NOHUGEPAGE,
                        )
                    };
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = pixmap;
}

/// 【标注缓存】【完整清空】将pixmap的全部自有像素恢复透明，无返回值；Linux归还内部完整匿名页，其他平台及系统调用失败时写入零值
pub(crate) fn clear(pixmap: &mut Pixmap) {
    clear_bytes(pixmap.data_mut());
}

/// 【前景缓存】【区间消费】将pixmap中range指定的已交付字节恢复零值，无返回值，边界整页全部为零时也归还，未交付非零字节始终保留
#[cfg(target_os = "linux")]
pub(crate) fn clear_range(pixmap: &mut Pixmap, range: std::ops::Range<usize>) {
    clear_bytes(&mut pixmap.data_mut()[range.clone()]);
    reclaim_zero_boundaries(pixmap, range);
}

/// 【前景缓存】【边界回收】按pixmap和range只检查触及的两个边界整页，返回值为空，完整页内存在任一非零字节时不归还
#[cfg(target_os = "linux")]
fn reclaim_zero_boundaries(pixmap: &mut Pixmap, range: std::ops::Range<usize>) {
    #[cfg(target_os = "linux")]
    {
        if range.is_empty() {
            return;
        }
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
        let Ok(page) = usize::try_from(page) else {
            return;
        };
        if !page.is_power_of_two() || page < 64 {
            return;
        }
        let bytes = pixmap.data_mut();
        let address = bytes.as_mut_ptr() as usize;
        let owned_first = address.div_ceil(page) * page;
        let owned_last = (address + bytes.len()) / page * page;
        let first = (address + range.start) / page * page;
        let last = (address + range.end - 1) / page * page;
        // 1. 【前景缓存】【自有边界】只处理Pixmap内部完整匿名页，不触及分配器头部；两个边界重合时只检查一次
        for (index, start) in [first, last].into_iter().enumerate() {
            if start < owned_first || start + page > owned_last || index == 1 && first == last {
                continue;
            }
            let offset = start - address;
            if bytes[offset..offset + page]
                .as_chunks::<64>()
                .0
                .iter()
                .all(|block| *block == [0; 64])
            {
                // 2. 【前景缓存】【零值保持】未交付字节若仍非零则保留，已经全零的边界页归还后逻辑内容仍为零
                unsafe {
                    nix::libc::madvise(
                        bytes.as_mut_ptr().add(offset).cast(),
                        page,
                        nix::libc::MADV_DONTNEED,
                    )
                };
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (pixmap, range);
}

/// 【前景缓存】【自有清空】按bytes清空唯一Pixmap提供的自有字节区间，无返回值，不处理区间外像素或分配器元数据
fn clear_bytes(bytes: &mut [u8]) {
    #[cfg(target_os = "linux")]
    {
        // 1. 【标注缓存】【页边界】Pixmap唯一持有普通Vec分配，仅处理数据内部完整页，不覆盖分配器头部或相邻对象
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
        if let Ok(page) = usize::try_from(page)
            && page.is_power_of_two()
        {
            let address = bytes.as_mut_ptr() as usize;
            let leading = (page - address % page) % page;
            if leading < bytes.len() {
                let whole = (bytes.len() - leading) / page * page;
                if whole > 0 {
                    // 2. 【标注缓存】【匿名清空】独占Vec数据内完整匿名页归还后读取为零，地址和容量保持有效；调用失败不假定像素已经清空
                    let result = unsafe {
                        nix::libc::madvise(
                            bytes.as_mut_ptr().add(leading).cast(),
                            whole,
                            nix::libc::MADV_DONTNEED,
                        )
                    };
                    if result == 0 {
                        bytes[..leading].fill(0);
                        bytes[leading + whole..].fill(0);
                        return;
                    }
                }
            }
        }
    }
    // 3. 【标注缓存】【像素兼容】不支持匿名页归还时保持完整透明字节，功能不依赖系统调用成功
    bytes.fill(0);
}

/// 【前景缓存】【透明回收】按pixmap归还内容完全透明的内部匿名页，无返回值；保留标注、面板及其他非零像素
pub(crate) fn reclaim_blank(pixmap: &mut Pixmap) {
    let length = pixmap.data().len();
    reclaim_blank_range(pixmap.data_mut(), 0..length);
}

/// 【前景缓存】【区间透明回收】按bytes和range检查相交的自有完整页，返回值为空；整页非零时保留全部字节，不处理其他页
fn reclaim_blank_range(bytes: &mut [u8], range: std::ops::Range<usize>) {
    #[cfg(target_os = "linux")]
    {
        if range.is_empty() {
            return;
        }
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
        let Ok(page) = usize::try_from(page) else {
            return;
        };
        if !page.is_power_of_two() || page < 64 {
            return;
        }
        let address = bytes.as_mut_ptr() as usize;
        let owned_first = address.div_ceil(page) * page;
        let owned_last = (address + bytes.len()) / page * page;
        let first = ((address + range.start) / page * page).max(owned_first);
        let last = (address + range.end)
            .div_ceil(page)
            .saturating_mul(page)
            .min(owned_last);
        if first >= last {
            return;
        }
        let first = first - address;
        let last = last - address;
        let mut run = first;
        // 1. 【前景缓存】【完整页检查】只扫描自有Vec内部完整页，零值与透明预乘像素相同，不处理边界或相邻对象
        for offset in (first..last).step_by(page) {
            if bytes[offset..offset + page]
                .as_chunks::<64>()
                .0
                .iter()
                .all(|block| *block == [0; 64])
            {
                continue;
            }
            if run < offset {
                // 2. 【前景缓存】【原值归还】已经验证全部为零的匿名页回收后仍为零，非零前景不参与回收
                unsafe {
                    nix::libc::madvise(
                        bytes.as_mut_ptr().add(run).cast(),
                        offset - run,
                        nix::libc::MADV_DONTNEED,
                    )
                };
            }
            run = offset + page;
        }
        if run < last {
            unsafe {
                nix::libc::madvise(
                    bytes.as_mut_ptr().add(run).cast(),
                    last - run,
                    nix::libc::MADV_DONTNEED,
                )
            };
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (bytes, range);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【标注缓存测试】【清空与重用】无参数，验证跨页和小画布全部字节透明、地址不变且能够再次绘制，无返回值
    #[test]
    fn clear_preserves_owned_storage_and_all_transparent_pixels() {
        for size in [(7, 9), (513, 257), (1024, 1024)] {
            let mut pixmap = Pixmap::new(size.0, size.1).unwrap();
            pixmap.data_mut().fill(137);
            let address = pixmap.data().as_ptr();
            clear(&mut pixmap);
            assert_eq!(pixmap.data().as_ptr(), address);
            assert!(pixmap.data().iter().all(|byte| *byte == 0));
            pixmap.fill(tiny_skia::Color::from_rgba8(90, 120, 200, 255));
            assert_eq!(pixmap.pixel(size.0 - 1, size.1 - 1).unwrap().blue(), 200);
            clear(&mut pixmap);
            assert!(pixmap.data().iter().all(|byte| *byte == 0));
        }
    }

    /// 【前景缓存测试】【非零保留】无参数，归还透明页后所有稀疏前景字节和边界仍保持原值，无返回值
    #[test]
    fn reclaiming_blank_pages_keeps_every_nonzero_foreground_pixel() {
        let mut pixmap = Pixmap::new(513, 257).unwrap();
        for offset in [0, 21_431, 239_811, pixmap.data().len() - 1] {
            pixmap.data_mut()[offset] = 91;
        }
        let expected = pixmap.data().to_vec();
        reclaim_blank(&mut pixmap);
        assert_eq!(pixmap.data(), expected);
    }
}
