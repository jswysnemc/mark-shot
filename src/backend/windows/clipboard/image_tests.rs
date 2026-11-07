//! 【Windows剪贴板测试】【格式像素】验证透明DIBV5的通道和方向，不访问系统剪贴板

use super::image;

/// 【Windows剪贴板测试】【透明样本】返回覆盖全部透明度的非对称RGBA与PNG，颜色不预乘，完全透明像素保留隐藏通道
pub(super) fn fixture() -> (Vec<u8>, Vec<u8>) {
    let rgba = (0_u16..256)
        .flat_map(|alpha| [alpha as u8 ^ 0x5a, 197, 31, alpha as u8])
        .collect::<Vec<_>>();
    let png = crate::utils::encode_rgba(&rgba, 16, 16);
    (rgba, png)
}

/// 【Windows剪贴板测试】【顶向格式】无参数，全部透明度和隐藏颜色经DIBV5往返保持逐字节一致，无返回值
#[test]
fn dibv5_preserves_every_alpha_and_unassociated_channel() {
    let (rgba, png) = fixture();
    let dib = image::dib(&png).unwrap();
    assert_eq!(i32::from_le_bytes(dib[8..12].try_into().unwrap()), -16);
    assert_eq!(&dib[124..128], &[31, 197, 0x5a, 0]);
    let decoded = ::image::load_from_memory(&image::png(&dib).unwrap())
        .unwrap()
        .into_rgba8();
    assert_eq!(decoded.dimensions(), (16, 16));
    assert_eq!(decoded.as_raw(), &rgba);
}

/// 【Windows剪贴板测试】【底向格式】无参数，反向存储行的正高度DIB仍还原相同图像，无返回值
#[test]
fn bottom_up_dibv5_restores_original_row_order() {
    let (rgba, png) = fixture();
    let mut dib = image::dib(&png).unwrap();
    dib[8..12].copy_from_slice(&16_i32.to_le_bytes());
    let rows = dib[124..].to_vec();
    for (index, row) in rows.as_chunks::<64>().0.iter().rev().enumerate() {
        dib[124 + index * 64..124 + (index + 1) * 64].copy_from_slice(row);
    }
    let decoded = ::image::load_from_memory(&image::png(&dib).unwrap())
        .unwrap()
        .into_rgba8();
    assert_eq!(decoded.as_raw(), &rgba);
}

/// 【Windows剪贴板测试】【非法输入】无参数，不完整头部、溢出调色板和截断像素必须返回错误，无返回值
#[test]
fn invalid_headers_and_truncated_pixels_are_rejected() {
    let (_, png) = fixture();
    let dib = image::dib(&png).unwrap();
    for length in [0, 3, 39, 123, 124, 128] {
        assert!(image::png(&dib[..length]).is_err(), "length={length}");
    }
    let mut overflow = dib;
    overflow[32..36].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(image::png(&overflow).is_err());
}

/// 【Windows剪贴板测试】【旧格式行填充】无参数，二十四位DIB的四字节行对齐和底向顺序正确，缺少透明通道时保持不透明，无返回值
#[test]
fn legacy_rgb_dib_handles_padding_and_bottom_up_rows() {
    let mut dib = vec![0_u8; 40];
    dib[..4].copy_from_slice(&40_u32.to_le_bytes());
    dib[4..8].copy_from_slice(&3_i32.to_le_bytes());
    dib[8..12].copy_from_slice(&2_i32.to_le_bytes());
    dib[12..14].copy_from_slice(&1_u16.to_le_bytes());
    dib[14..16].copy_from_slice(&24_u16.to_le_bytes());
    let pixels = [
        [9, 31, 73],
        [11, 45, 91],
        [21, 60, 121],
        [44, 81, 131],
        [61, 92, 171],
        [85, 111, 201],
    ];
    for row in pixels.as_chunks::<3>().0.iter().rev() {
        for pixel in row {
            dib.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
        dib.extend_from_slice(&[211; 3]);
    }
    let actual = ::image::load_from_memory(&image::png(&dib).unwrap())
        .unwrap()
        .into_rgba8();
    let expected = pixels
        .iter()
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
        .collect::<Vec<_>>();
    assert_eq!(actual.dimensions(), (3, 2));
    assert_eq!(actual.as_raw(), &expected);
}
