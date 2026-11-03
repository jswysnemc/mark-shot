//! 【Wayland缓冲】【有界复用】贴图和原生菜单共用空闲缓冲选择，最多保留三个当前尺寸缓冲

use crate::AppResult;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::protocol::wl_shm;

/// 【Wayland缓冲】【缓冲选择】为 size 从 buffers、pool 返回空闲缓冲索引；拒绝过大尺寸，三个缓冲仍占用时返回空值
pub fn available(
    pool: &mut SlotPool,
    buffers: &mut Vec<Buffer>,
    size: (u32, u32),
) -> AppResult<Option<usize>> {
    let (width, height) = (size.0 as i32, size.1 as i32);
    if width <= 0 || height <= 0 || u64::from(size.0) * u64::from(size.1) > 64 * 1024 * 1024 {
        return Err("Wayland buffer exceeds the pixel limit".into());
    }
    buffers.retain(|buffer| buffer.height() == height && buffer.stride() == width * 4);
    if let Some(index) = buffers
        .iter()
        .position(|buffer| buffer.canvas(pool).is_some())
    {
        return Ok(Some(index));
    }
    if buffers.len() >= 3 {
        return Ok(None);
    }
    let (buffer, _) = pool.create_buffer(width, height, width * 4, wl_shm::Format::Argb8888)?;
    buffers.push(buffer);
    Ok(Some(buffers.len() - 1))
}
