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

    Ok(ShmBuffer { buffer })
}

impl ShmBuffer {
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
