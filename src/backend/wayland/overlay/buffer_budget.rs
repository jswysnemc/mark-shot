//! 【截图覆盖层】【缓冲预算】按实际角色预先触碰共享页，不为滚动截图保留已关闭的静态背景

use crate::types::Output;
use tiny_skia::Rect;

const AUXILIARY_BYTES: usize = 256 * 1024;

/// 【截图覆盖层】【角色预算】按 outputs、hidpi、background 和 panel 返回共享字节预算；包含占位表面余量，避免小槽位触发整池翻倍
pub(super) fn size<'a>(
    outputs: impl Iterator<Item = &'a Output>,
    hidpi: bool,
    background: bool,
    panel: Option<Rect>,
) -> usize {
    outputs
        .map(|output| {
            let scale = if hidpi { output.scale() } else { 1.0 };
            let logical = panel.map_or_else(
                || {
                    let (width, height) = output.info.logical_size.unwrap_or((0, 0));
                    (width.max(0) as u32, height.max(0) as u32)
                },
                |rect| (rect.width().ceil() as u32, rect.height().ceil() as u32),
            );
            let mut bytes = slot_bytes(crate::types::px_size(logical, scale));
            // 1. 【截图覆盖层】【静态背景】普通截图保留原生背景，滚动角色只需要当前控制表面
            if background {
                let (width, height) = output
                    .info
                    .modes
                    .iter()
                    .find(|mode| mode.current)
                    .map_or((0, 0), |mode| mode.dimensions);
                bytes += slot_bytes((width.max(0) as u32, height.max(0) as u32));
            }
            bytes
        })
        .sum::<usize>()
        .saturating_add(AUXILIARY_BYTES)
}

/// 【截图覆盖层】【像素字节】按 size 返回四通道缓冲的六十四字节对齐长度，避免槽位尾部碎片
fn slot_bytes(size: (u32, u32)) -> usize {
    (size.0 as usize * size.1 as usize * 4).next_multiple_of(64)
}
