//! 【滚动截图测试】【真实平色】验证不透明面板样本、缩放位置和截断映射，避免背景或部分面板被当作安全位置

use super::Stamp;
use crate::types::{MonitorFrame, StreamInfo};

/// 【滚动截图测试】【面板缓冲】返回 width、height 的白底与绿色保存控件原始像素，不使用图形会话
fn panel(width: u32, height: u32) -> Vec<u8> {
    (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                if y >= height - 58 && y < height - 20 && x > 40 && x < 80 {
                    [16, 185, 129, 255]
                } else {
                    [255, 255, 255, 255]
                }
            })
        })
        .collect()
}

/// 【滚动截图测试】【显示帧】将 panel 以 factor 置于 origin 并为每行加填充，返回包含实际行距的帧
fn frame(panel: &[u8], size: (u32, u32), origin: (u32, u32), factor: u32) -> MonitorFrame {
    let (width, height, stride) = (600_u32, 500_u32, 600 * 4 + 16);
    let mut pixels = vec![40; stride as usize * height as usize];
    for y in 0..size.1 * factor {
        for x in 0..size.0 * factor {
            let src = ((y / factor) * size.0 + x / factor) as usize * 4;
            let dst = ((y + origin.1) * stride + (x + origin.0) * 4) as usize;
            pixels[dst..dst + 4].copy_from_slice(&panel[src..src + 4]);
        }
    }
    MonitorFrame {
        output: 0,
        pixels,
        pw_width: width,
        pw_height: height,
        pw_stride: stride,
        info: StreamInfo {
            node_id: 0,
            size: Some((600, 500)),
            position: Some((0, 0)),
        },
    }
}

/// 【滚动截图测试】【位置与缩放】复核当前与移动后的位置、整数放大和填充行，完整面板以外不得命中，无参数
#[test]
fn tracks_relocation_and_scaled_compositor_pixels_with_padded_stride() {
    let size = (120, 120);
    let rgba = panel(size.0, size.1);
    let stamp = Stamp::new(&rgba, size, 1.0).unwrap();
    let first = frame(&rgba, size, (40, 80), 1);
    assert_eq!(
        stamp.locate(&first, size, size, None),
        Some((40, 80, 120, 120))
    );
    let moved = frame(&rgba, size, (180, 100), 1);
    assert_eq!(
        stamp.locate(&moved, size, size, Some((40, 80, 120, 120))),
        Some((180, 100, 120, 120))
    );
    let scaled = frame(&rgba, size, (120, 120), 2);
    assert_eq!(
        stamp.locate(&scaled, (240, 240), size, None),
        Some((120, 120, 240, 240))
    );
}

/// 【滚动截图测试】【映射拒绝】截断、错误行距、部分遮挡或其他源尺寸不能产生有效面板位置，无参数
#[test]
fn rejects_obscured_panels_invalid_rows_and_foreign_source_sizes() {
    let size = (120, 120);
    let rgba = panel(size.0, size.1);
    let stamp = Stamp::new(&rgba, size, 1.0).unwrap();
    let mut image = frame(&rgba, size, (40, 80), 1);
    let row = (80 + size.1 - 58) as usize * image.pw_stride as usize;
    image.pixels[row..row + image.pw_width as usize * 4].fill(40);
    assert!(stamp.locate(&image, size, size, None).is_none());
    image = frame(&rgba, size, (40, 80), 1);
    assert!(stamp.locate(&image, size, (121, 120), None).is_none());
    image.pixels.truncate(100);
    assert!(stamp.locate(&image, size, size, None).is_none());
    image.pw_stride = 16;
    assert!(stamp.locate(&image, size, size, None).is_none());
    assert!(Stamp::new(&vec![255; 120 * 120 * 4], size, 1.0).is_none());
}
