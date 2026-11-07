//! 【图像原件】【区域复制】按真实像素行裁切，读取窗口分批回收，不改变透明像素

use super::{ImageSource, MEMORY_BYTES};
use tiny_skia::{IntRect, IntSize, Pixmap, PixmapRef};

/// 【图像原件】【写回范围】按size、patch和position返回目标相交范围，区域外返回空值，不移动有效像素
pub(super) fn patch_bounds(
    size: IntSize,
    patch: PixmapRef<'_>,
    position: (i32, i32),
) -> Option<IntRect> {
    IntRect::from_xywh(position.0, position.1, patch.width(), patch.height())?
        .intersect(&IntRect::from_xywh(0, 0, size.width(), size.height())?)
}

/// 【图像原件】【片段写入】按size、patch和position遍历原值行，write接收目标字节位置与源行，返回写入结果
pub(super) fn patch_rows(
    size: IntSize,
    patch: PixmapRef<'_>,
    position: (i32, i32),
    mut write: impl FnMut(usize, &[u8]) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let Some(bounds) = patch_bounds(size, patch, position) else {
        return Ok(());
    };
    let source_stride = patch.width() as usize * 4;
    let target_stride = size.width() as usize * 4;
    let bytes = bounds.width() as usize * 4;
    for row in bounds.top()..bounds.bottom() {
        let start =
            (row - position.1) as usize * source_stride + (bounds.left() - position.0) as usize * 4;
        let target = row as usize * target_stride + bounds.left() as usize * 4;
        write(target, &patch.data()[start..start + bytes])?;
    }
    Ok(())
}

/// 【图像原件】【内存片段】将patch按position逐字节写入target，不进行混合，返回值为空
pub(super) fn patch_memory(target: &mut Pixmap, patch: PixmapRef<'_>, position: (i32, i32)) {
    let size = IntSize::from_wh(target.width(), target.height()).unwrap();
    patch_rows(size, patch, position, |offset, bytes| {
        target.data_mut()[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    })
    .expect("memory rows cannot report a file error");
}

/// 【图像原件】【行裁切】按source和region复制原件相交区域，返回完整RGBA画布，读取约4MiB后回收原件页
pub(super) fn copy(source: &ImageSource, region: IntRect) -> Option<Pixmap> {
    let full = IntRect::from_xywh(0, 0, source.width(), source.height())?;
    let bounds = full.intersect(&region)?;
    let mut output = Pixmap::new(bounds.width(), bounds.height())?;
    let stride = source.width() as usize * 4;
    let row_bytes = bounds.width() as usize * 4;
    let rows = (MEMORY_BYTES / stride).max(1);
    // 1. 【图像原件】【逐行原值】源视图始终保持完整坐标，逐行复制不触发混合、重采样或透明度归一化
    for row in 0..bounds.height() as usize {
        let start = (bounds.top() as usize + row) * stride + bounds.left() as usize * 4;
        output.data_mut()[row * row_bytes..(row + 1) * row_bytes]
            .copy_from_slice(&source.data()[start..start + row_bytes]);
        if (row + 1) % rows == 0 {
            source.release_pages();
        }
    }
    // 2. 【图像原件】【读取结束】最后不足一批的原件页也回收，输出继续持有已经复制的全部像素
    source.release_pages();
    Some(output)
}
