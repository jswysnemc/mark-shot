//! 【共享前景】【透明页写入】透明整页保持共享零页，非零像素仍按原始网格完整交付

/// 【共享前景】【像素写入】把source写入同尺寸destination，swizzled选择RGBA到BGRA转换，无返回值；只回收目的区间内部完整透明页
pub(crate) fn copy(destination: &mut [u8], source: &[u8], swizzled: bool) {
    assert_eq!(destination.len(), source.len());
    #[cfg(target_os = "linux")]
    {
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
        if let Ok(page) = usize::try_from(page)
            && page.is_power_of_two()
            && page >= 64
        {
            let address = destination.as_ptr() as usize;
            let first = ((page - address % page) % page).min(destination.len());
            let last = first + (destination.len() - first) / page * page;
            write(&mut destination[..first], &source[..first], swizzled);
            let mut zero_start = first;
            // 1. 【共享前景】【透明区间】以目的地址页边界检查来源，整页透明时不写入物理页，连续透明页只执行一次归还
            for offset in (first..last).step_by(page) {
                if source[offset..offset + page]
                    .as_chunks::<64>()
                    .0
                    .iter()
                    .all(|block| *block == [0; 64])
                {
                    continue;
                }
                remove(&mut destination[zero_start..offset]);
                write(
                    &mut destination[offset..offset + page],
                    &source[offset..offset + page],
                    swizzled,
                );
                zero_start = offset + page;
            }
            remove(&mut destination[zero_start..last]);
            // 2. 【共享前景】【边界保留】不足一页的头尾沿用逐字节写入，不移除相邻槽位或本次损坏范围外内容
            write(&mut destination[last..], &source[last..], swizzled);
            return;
        }
    }
    write(destination, source, swizzled);
}

/// 【共享前景】【通道处理】按swizzled复制source到destination，返回值为空，来源和目的始终具有相同长度
fn write(destination: &mut [u8], source: &[u8], swizzled: bool) {
    if swizzled {
        crate::utils::copy_swizzled(destination, source);
    } else {
        destination.copy_from_slice(source);
    }
}

/// 【共享前景】【真实归还】将bytes对应的已验证透明共享整页恢复零值，无返回值，系统不支持归还时写入零值
#[cfg(target_os = "linux")]
fn remove(bytes: &mut [u8]) {
    if bytes.is_empty() {
        return;
    }
    let result = unsafe {
        nix::libc::madvise(
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            nix::libc::MADV_REMOVE,
        )
    };
    if result != 0 {
        bytes.fill(0);
    }
}

#[cfg(test)]
#[path = "shm_sparse_tests.rs"]
mod tests;
