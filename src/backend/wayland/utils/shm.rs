// ── Shared Memory (shm) buffer Utilities ──────────────────────────────────────
// manage low-level shared memory allocations, stride math, and raw pixel  copying for surface drawing.
//
// in 'utils' to separate raw memory manipulation and buffer rendering
// from the high-level Wayland protocol handlers ('state/compositor_shm_layer.rs')
// reminder: each output has its own surface

use crate::utils::copy_swizzled;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::protocol::{wl_buffer, wl_shm};
pub struct ShmBuffer {
    pub buffer: Buffer,
    reclaimed: bool,
    reclaim_error_reported: bool,
}

pub fn create_shm_buffer(
    pool: &mut SlotPool,
    width: u32,
    height: u32,
) -> Result<ShmBuffer, Box<dyn std::error::Error>> {
    let stride = width as i32 * 4;
    let (buffer, _canvas) = pool.create_buffer(
        width as i32,
        height as i32,
        stride,
        wl_shm::Format::Argb8888,
    )?;

    Ok(ShmBuffer {
        buffer,
        reclaimed: false,
        reclaim_error_reported: false,
    })
}

impl ShmBuffer {
    /// 【共享缓冲】【服务端占用】无参数，返回当前槽位是否仍有服务端正在读取的缓冲
    pub(crate) fn active(&self) -> bool {
        self.buffer.slot().has_active_buffers()
    }

    /// 【共享缓冲】【原生帧写入】按pool和write借用已经释放的原生字节，返回写入结果；活动缓冲不允许修改
    pub(crate) fn write(
        &mut self,
        pool: &mut SlotPool,
        write: impl FnOnce(&mut [u8]) -> crate::AppResult,
    ) -> crate::AppResult {
        let pixels = pool
            .canvas(&self.buffer)
            .ok_or("the native frame buffer is still active")?;
        self.reclaimed = false;
        write(pixels)
    }

    /// 【共享缓冲】【已有帧复制】按pool和source复制最新完整帧，返回结果；来源只读、目标空闲，两个槽位始终独立
    pub(crate) fn copy_from(
        &mut self,
        pool: &mut SlotPool,
        source: &ShmBuffer,
    ) -> crate::AppResult {
        if self.buffer.height() != source.buffer.height()
            || self.buffer.stride() != source.buffer.stride()
        {
            return Err("native frame buffer dimensions changed".into());
        }
        let length = source.buffer.height() as usize * source.buffer.stride() as usize;
        self.write(pool, |destination| {
            if destination.len() != length {
                return Err("native frame buffer length changed".into());
            }
            Ok(())
        })?;
        let source_slot = source.buffer.slot();
        let chunk = 64 * 1024;
        let mut row = vec![0; chunk.min(length)];
        // 1. 【共享缓冲】【只读来源】每次只借用至多64KiB来源并复制到交换区，不写入活动来源，也不跨两次共享池借用保存指针
        for first in (0..length).step_by(chunk) {
            let count = (length - first).min(chunk);
            row[..count].copy_from_slice(&pool.raw_data_mut(&source_slot)[first..first + count]);
            // 2. 【共享缓冲】【独立交付】来源借用结束后才借用已释放目标，透明整页保持零页，不创建第二份完整CPU帧
            let destination = pool
                .canvas(&self.buffer)
                .ok_or("the destination native buffer became active")?;
            super::shm_sparse::copy(&mut destination[first..first + count], &row[..count], false);
        }
        Ok(())
    }

    /// 【共享缓冲】【空闲页回收】按pool归还已释放备用缓冲的内部完整共享页，无返回值；下一次使用必须先重建完整帧
    pub(crate) fn reclaim(&mut self, pool: &mut SlotPool) {
        if self.reclaimed || self.active() {
            return;
        }
        #[cfg(target_os = "linux")]
        if let Some(pixels) = pool.canvas(&self.buffer) {
            let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
            if let Ok(page) = usize::try_from(page)
                && page.is_power_of_two()
            {
                let start = pixels.as_mut_ptr() as usize;
                let leading = (page - start % page) % page;
                if leading < pixels.len() {
                    let length = (pixels.len() - leading) / page * page;
                    if length != 0 {
                        // 1. 【共享缓冲】【真实归还】服务端已释放该独立槽位，仅移除数据内部完整memfd页，不触及相邻缓冲；备用内容随后从最新帧重新复制
                        let result = unsafe {
                            nix::libc::madvise(
                                pixels.as_mut_ptr().add(leading).cast(),
                                length,
                                nix::libc::MADV_REMOVE,
                            )
                        };
                        if result != 0 {
                            if !self.reclaim_error_reported {
                                log::error!(
                                    "【共享缓冲】【空闲页回收】{}",
                                    std::io::Error::last_os_error()
                                );
                                self.reclaim_error_reported = true;
                            }
                            return;
                        }
                    }
                }
                self.reclaimed = true;
            }
        }
    }
    /// 【共享缓冲】【透明初始化】按pool将可写缓冲清零，返回值为空，不创建同尺寸中间数组
    pub(crate) fn clear(&mut self, pool: &mut SlotPool) {
        if let Some(canvas) = pool.canvas(&self.buffer) {
            canvas.fill(0);
        }
    }
    pub fn wl_buffer(&self) -> &wl_buffer::WlBuffer {
        self.buffer.wl_buffer()
    }

    pub fn write_pixels(&mut self, pool: &mut SlotPool, pixels: &[u8]) {
        if let Some(canvas) = pool.canvas(&self.buffer) {
            let len = pixels.len().min(canvas.len());

            copy_swizzled(&mut canvas[..len], &pixels[..len]);
        }
    }

    pub fn write_pixels_rect(
        &mut self,
        pool: &mut SlotPool,
        pixels: &[u8],
        width: u32,
        rect: (u32, u32, u32, u32),
    ) {
        let (x, y, w, h) = rect;
        if w == 0 || h == 0 {
            return;
        }

        let w = w.min(width.saturating_sub(x));
        let stride = (width * 4) as usize;
        let row_bytes = (w * 4) as usize;

        if let Some(dst) = pool.canvas(&self.buffer) {
            let len = dst.len().min(pixels.len());
            for row in 0..h {
                let sy = (y + row) as usize;
                let sx = x as usize;
                let off = sy * stride + sx * 4;
                if off + row_bytes > len {
                    break;
                }

                copy_swizzled(
                    &mut dst[off..off + row_bytes],
                    &pixels[off..off + row_bytes],
                );
            }
        }
    }
}
