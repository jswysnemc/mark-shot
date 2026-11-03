//! 【截图历史测试】【入口】存储安全与无损 PNG 的独立回归

mod metadata;
mod preview;
mod storage;

/// 【截图历史测试】【图像构造】返回 width、height 的半透明测试 PNG，不依赖磁盘或图形会话
pub(super) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[10, 80, 170, 91].repeat((width * height) as usize))
        .unwrap();
    bytes
}
