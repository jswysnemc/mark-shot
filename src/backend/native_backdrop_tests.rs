//! 【原生背景测试】【原有栅格】全部透明度、相交损坏及磁盘来源与既有CPU背景合成逐像素一致

use super::*;
use tiny_skia::{ColorU8, Pixmap};

/// 【原生背景测试】【纹理构造】按size和salt返回包含完整颜色及透明度的预乘纹理
fn texture(size: (u32, u32), salt: u32) -> Pixmap {
    let mut pixels = Pixmap::new(size.0, size.1).unwrap();
    for (index, pixel) in pixels.pixels_mut().iter_mut().enumerate() {
        let hash = (index as u32).wrapping_mul(2_654_435_761).rotate_left(7) ^ salt;
        *pixel = ColorU8::from_rgba(
            hash as u8,
            (hash >> 8) as u8,
            (hash >> 16) as u8,
            index as u8,
        )
        .premultiply();
    }
    pixels
}

/// 【原生背景测试】【通道转换】按pixels返回BGRA字节，使用与现有Wayland上传相同的转换函数
fn bgra(pixels: &Pixmap) -> Vec<u8> {
    let mut bytes = vec![0; pixels.data().len()];
    crate::utils::copy_swizzled(&mut bytes, pixels.data());
    bytes
}

/// 【原生背景测试】【全透明度】无参数，RGBA与BGRA的全部透明度和通道必须匹配既有DestinationOver，无返回值
#[test]
fn native_pixels_match_the_existing_cpu_backdrop_for_all_alpha_values() {
    for size in [(257, 193), (1100, 1100)] {
        let original = texture(size, 93);
        let source = ImageSource::memory(original);
        let mut expected = texture(size, 17);
        let mut actual = bgra(&expected);
        crate::renderer::opaque_backdrop::compose(&source, &mut expected, None);
        compose_bgra(&source, &mut actual, size, None).unwrap();
        assert_eq!(actual, bgra(&expected));
    }
}

/// 【原生背景测试】【局部交付】无参数，反复损坏与越界范围只改变真实相交像素，无返回值
#[test]
fn damaged_native_pixels_match_cpu_composition_without_touching_other_pixels() {
    let size = (257, 193);
    let source = ImageSource::memory(texture(size, 93));
    for damage in [(17, 31, 39, 80), (250, 190, 30, 30), (0, 0, 257, 3)] {
        let mut expected = texture(size, 17);
        let mut actual = bgra(&expected);
        crate::renderer::opaque_backdrop::compose(&source, &mut expected, Some(damage));
        compose_bgra(&source, &mut actual, size, Some(damage)).unwrap();
        assert_eq!(actual, bgra(&expected));
    }
}

/// 【原生背景测试】【网格校验】无参数，拒绝不一致像素网格且保留调用方全部字节，无返回值
#[test]
fn different_pixel_grids_are_rejected_before_modifying_the_native_buffer() {
    let source = ImageSource::memory(texture((33, 29), 19));
    let original = bgra(&texture((32, 29), 31));
    let mut actual = original.clone();
    assert!(compose_bgra(&source, &mut actual, (32, 29), None).is_err());
    assert_eq!(actual, original);
}
