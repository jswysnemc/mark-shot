//! 【画布导航测试】【原图采样】独立检查像素插值、透明混合及可见范围

use super::*;
use tiny_skia::{Color, PixmapPaint};

/// 【画布导航测试】【像素参考】按尺寸、透明度与变换对照通用双线性光栅器；整数边界精确一致，插值误差最多两个色阶
#[test]
fn bilinear_sampling_matches_reference_and_preserves_background() {
    for alpha in [0, 128, 255] {
        let mut source = Pixmap::new(37, 29).unwrap();
        for (index, pixel) in source.pixels_mut().iter_mut().enumerate() {
            *pixel = tiny_skia::ColorU8::from_rgba(
                (index * 17) as u8,
                (index * 31) as u8,
                (index * 7) as u8,
                alpha,
            )
            .premultiply();
        }
        for scale in [0.5, 1.0, 1.25, 2.0, 12.0] {
            let transform = Transform::from_scale(scale, scale).post_translate(-3.0, 8.0);
            let mut actual = Pixmap::new(600, 500).unwrap();
            actual.fill(Color::from_rgba8(51, 55, 62, 255));
            let mut expected = actual.clone();
            expected.draw_pixmap(
                0,
                0,
                source.as_ref(),
                &PixmapPaint {
                    quality: tiny_skia::FilterQuality::Bilinear,
                    ..Default::default()
                },
                transform,
                None,
            );
            assert!(draw(&mut actual, source.as_ref(), transform));
            // 1. 【画布导航测试】【边缘隔离】通用光栅器的边缘覆盖实现独立，内部验证插值并检查外部原样保留
            for row in 0..500 {
                for column in 0..600 {
                    let inside = column as f32 + 1.0 < 37.0 * scale - 3.0
                        && row > 9
                        && (row as f32 + 1.0) < 29.0 * scale + 8.0;
                    let outside = column as f32 > 37.0 * scale - 3.0 + 1.0
                        || row < 7
                        || row as f32 > 29.0 * scale + 9.0;
                    if inside || outside {
                        let offset = (row * 600 + column) * 4;
                        for channel in 0..4 {
                            assert!(
                                actual.data()[offset + channel]
                                    .abs_diff(expected.data()[offset + channel])
                                    <= 2,
                                "scale={scale} alpha={alpha} x={column} y={row} channel={channel}"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// 【画布导航测试】【透明边缘】分数平移只覆盖相交面积，透明像素不能染黑背景；旋转交回通用路径
#[test]
fn fractional_coverage_and_unsupported_transform_are_explicit() {
    let mut source = Pixmap::new(1, 1).unwrap();
    source.fill(Color::WHITE);
    let mut actual = Pixmap::new(3, 3).unwrap();
    assert!(draw(
        &mut actual,
        source.as_ref(),
        Transform::from_translate(0.5, 0.5)
    ));
    assert_eq!(actual.pixel(0, 0).unwrap().alpha(), 64);
    assert_eq!(actual.pixel(1, 1).unwrap().alpha(), 64);
    assert_eq!(actual.pixel(2, 2).unwrap().alpha(), 0);
    assert!(!draw(
        &mut actual,
        source.as_ref(),
        Transform::from_rotate(30.0)
    ));
}
