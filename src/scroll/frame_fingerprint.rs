//! 【滚动截图】【像素指纹】完整扫描选区复核重复帧，不分配或保存另一份捕获图像

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use crate::types::MonitorFrame;
use shot_image::Image;

/// 【滚动截图】【指纹初始化】按 width、height 创建带尺寸信息的哈希器，返回固定种子的计算状态
fn start(width: u32, height: u32) -> DefaultHasher {
    let mut hasher = DefaultHasher::new();
    hasher.write_u32(width);
    hasher.write_u32(height);
    hasher
}

/// 【滚动截图】【原帧指纹】按 frame 和已校验的 rect 返回选区像素指纹，不包含填充字节或选区外像素
pub(super) fn capture(frame: &MonitorFrame, rect: (u32, u32, u32, u32)) -> u64 {
    let (left, top, width, height) = rect;
    let mut hasher = start(width, height);
    for vertical in top..top + height {
        let offset = vertical as usize * frame.pw_stride as usize + left as usize * 4;
        hasher.write(&frame.pixels[offset..offset + width as usize * 4]);
    }
    hasher.finish()
}

/// 【滚动截图】【裁切指纹】按 image 返回与原帧选区完全一致的像素指纹，无额外像素分配
pub(super) fn image(image: &Image) -> u64 {
    let mut hasher = start(image.width() as u32, image.height() as u32);
    let mut buffer = [0_u8; 1024];
    for vertical in 0..image.height() {
        for chunk in image.row(vertical).chunks(256) {
            for (&pixel, bytes) in chunk.iter().zip(buffer.as_chunks_mut::<4>().0) {
                *bytes = [
                    (pixel >> 16) as u8,
                    (pixel >> 8) as u8,
                    pixel as u8,
                    (pixel >> 24) as u8,
                ];
            }
            hasher.write(&buffer[..chunk.len() * 4]);
        }
    }
    hasher.finish()
}
