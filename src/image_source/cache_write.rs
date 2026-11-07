//! 【图像原件】【分批写入】同步并归还已经完成的文件页，避免完整原件集中驻留在写入缓存

use std::{fs::File, io::Write};

/// 【图像原件】【写入批次】按file写入pixels，每批最多4MiB，返回原生文件结果，不保留额外完整缓冲
pub(super) fn write(file: &mut File, pixels: &[u8]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    let mut offset = 0;
    for bytes in pixels.chunks(super::MEMORY_BYTES) {
        file.write_all(bytes)?;
        // 1. 【图像原件】【脏页同步】数据同步成功后才能归还文件缓存，禁止把尚未写完的原件当作可重读来源
        file.sync_data()?;
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            // 2. 【图像原件】【批次回收】仅归还当前已完成范围，最后不足一页的字节由映射完成后的完整回收处理
            let result = unsafe {
                nix::libc::posix_fadvise(
                    file.as_raw_fd(),
                    offset as nix::libc::off_t,
                    bytes.len() as nix::libc::off_t,
                    nix::libc::POSIX_FADV_DONTNEED,
                )
            };
            if result != 0 {
                return Err(std::io::Error::from_raw_os_error(result));
            }
            offset += bytes.len();
        }
    }
    Ok(())
}
