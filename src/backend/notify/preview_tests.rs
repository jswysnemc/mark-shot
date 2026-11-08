//! 【桌面通知测试】【缩略图】验证长图尺寸、透明边缘和损坏输入

use super::*;

/// 【桌面通知测试】【预览像素】无参数，检查等比缩略与方形透明填充，避免预乘数据直接写入通知；无返回值
#[test]
fn thumbnail_preserves_alpha_and_pads_only_when_requested() {
    let pixels = [80, 160, 240, 128].repeat(16 * 4096);
    let png = crate::utils::encode_rgba(&pixels, 16, 4096);
    let preview = thumbnail(&png, false).unwrap();
    assert_eq!(preview.dimensions(), (1, 256));
    for pixel in preview.pixels() {
        for (actual, expected) in pixel.0.into_iter().zip([80u8, 160, 240, 128]) {
            assert!(actual.abs_diff(expected) <= 1);
        }
    }
    let square = thumbnail(&png, true).unwrap();
    assert_eq!(square.dimensions(), (256, 256));
    assert_eq!(square.get_pixel(127, 0), preview.get_pixel(0, 0));
    assert_eq!(square.get_pixel(126, 0).0, [0; 4]);
    assert_eq!(square.get_pixel(128, 0).0, [0; 4]);
    assert!(thumbnail(&png[..png.len() / 2], false).is_none());
}
