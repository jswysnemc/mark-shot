//! 【前景缓存】【页面驻留标志】读取当前进程的映射状态，区分独占匿名页与共享零页，不使用物理页编号

use std::{fs::File, ops::Range, os::unix::fs::FileExt, sync::OnceLock};

pub(super) struct Snapshot {
    first_page: usize,
    page: usize,
    flags: Vec<u8>,
}

impl Snapshot {
    /// 【前景缓存】【状态快照】按 address 和 page 读取相交页标志，返回快照；文件不可用或读取失败时返回空值
    pub(super) fn read(address: Range<usize>, page: usize) -> Option<Self> {
        if address.is_empty() || !page.is_power_of_two() {
            return None;
        }
        static PAGEMAP: OnceLock<Option<File>> = OnceLock::new();
        let file = PAGEMAP
            .get_or_init(|| File::open("/proc/self/pagemap").ok())
            .as_ref()?;
        let first_page = address.start / page;
        let count = (address.end - 1) / page - first_page + 1;
        let mut flags = vec![0; count.checked_mul(8)?];
        let offset = u64::try_from(first_page).ok()?.checked_mul(8)?;
        file.read_exact_at(&mut flags, offset).ok()?;
        Some(Self {
            first_page,
            page,
            flags,
        })
    }

    /// 【前景缓存】【独占判断】按 address 返回相交页是否同时驻留且独占，未覆盖位置或共享零页返回否
    pub(super) fn exclusive(&self, address: usize) -> bool {
        let Some(index) = (address / self.page).checked_sub(self.first_page) else {
            return false;
        };
        let Some(bytes) = self.flags.get(index * 8..index * 8 + 8) else {
            return false;
        };
        let flags = u64::from_ne_bytes(bytes.try_into().unwrap());
        // 1. 【前景缓存】【标志限定】只使用 present 与 exclusive 两个标志，零页不需要归还独立物理内存
        flags & ((1 << 63) | (1 << 56)) == ((1 << 63) | (1 << 56))
    }
}
