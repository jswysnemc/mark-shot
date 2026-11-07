//! 【原生前景】【字节交付】区分只读借用与明确消费，消费模式保留尚未交付的字节

use crate::{AppResult, types::DamageRect};
use tiny_skia::Pixmap;

pub(crate) enum FramePixels<'a> {
    Borrowed(&'a [u8]),
    Consumed(&'a mut Pixmap),
}

impl FramePixels<'_> {
    /// 【原生前景】【字节长度】无参数，返回完整源网格字节数，不读取像素或改变所有权
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Borrowed(pixels) => pixels.len(),
            Self::Consumed(pixmap) => pixmap.data().len(),
        }
    }

    /// 【原生前景】【像素复制】按size和damage把前景转换到native，返回校验结果；完整帧覆盖全部字节，局部帧保留范围外原生内容
    pub(crate) fn write_to(
        &mut self,
        native: &mut [u8],
        size: (u32, u32),
        damage: Option<DamageRect>,
    ) -> AppResult {
        let length = size.0 as usize * size.1 as usize * 4;
        if self.len() != length || native.len() != length {
            return Err("foreground dimensions do not match native storage".into());
        }
        if let Self::Consumed(pixmap) = self {
            crate::utils::transparent_pixmap::prepare_sparse(pixmap);
        }
        // 1. 【原生前景】【完整批次】完整帧按至多1MiB分批交付，消费后立即归还CPU页，再处理后续批次与冻结背景
        if damage.is_none_or(|area| area == (0, 0, size.0, size.1)) {
            let batch = 1024 * 1024;
            for first in (0..length).step_by(batch) {
                self.write_range(native, first..(first + batch).min(length));
            }
            self.finish();
            return Ok(());
        }
        let (x, y, width, height) = damage.unwrap_or((0, 0, size.0, size.1));
        let width = width.min(size.0.saturating_sub(x));
        let stride = size.0 as usize * 4;
        let bytes = width as usize * 4;
        for row in y..y.saturating_add(height).min(size.1) {
            let first = row as usize * stride + x as usize * 4;
            if first + bytes <= length {
                self.write_range(native, first..first + bytes);
            }
        }
        self.finish();
        Ok(())
    }

    /// 【原生前景】【消费结束】无参数，回收可能来自大页或短扫描行的全零自有页，返回值为空，未交付非零像素继续保留
    fn finish(&mut self) {
        if let Self::Consumed(pixmap) = self {
            crate::utils::transparent_pixmap::reclaim_blank(pixmap);
        }
    }

    /// 【原生前景】【字节批次】按range写入native，返回值为空；借用路径不改源，消费路径在复制结束后才清空相同源区间
    fn write_range(&mut self, native: &mut [u8], range: std::ops::Range<usize>) {
        match self {
            Self::Borrowed(source) => crate::backend::wayland::utils::shm_sparse::copy(
                &mut native[range.clone()],
                &source[range],
                true,
            ),
            Self::Consumed(pixmap) => {
                crate::backend::wayland::utils::shm_sparse::copy(
                    &mut native[range.clone()],
                    &pixmap.data()[range.clone()],
                    true,
                );
                // 2. 【原生前景】【所有权归还】来源只读借用已经结束，仅清空明确交付的自有字节，活动原生缓冲始终不参与修改
                crate::utils::transparent_pixmap::clear_range(pixmap, range);
            }
        }
    }
}

#[cfg(test)]
#[path = "frame_pixels_tests.rs"]
mod tests;
