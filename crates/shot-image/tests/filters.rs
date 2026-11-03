//! 【滤镜测试】【图像核心】验证原版参数换算、预乘像素和区域内网格

use shot_image::{ImageView, blur, filters, pixelate};

/// 【滤镜测试】【图像借用】以 pixels、width、height 创建紧凑测试视图；返回已验证视图
fn view(pixels: &[u32], width: i32, height: i32) -> ImageView<'_> {
    ImageView::new(pixels, width, height, width as usize).unwrap()
}

/// 【滤镜测试】【强度换算】模糊半径和提亮比例沿用原版规则，非有限值不能引发异常
#[test]
fn strength_conversion_matches_reference() {
    assert_eq!(filters::blur_radius(14.0), 14);
    assert_eq!(filters::blur_radius(14.5), 15);
    assert_eq!(filters::blur_radius(-9.0), 1);
    assert_eq!(filters::blur_radius(90.0), 48);
    assert_eq!(filters::blur_radius(f64::NAN), 1);
    assert_eq!(filters::blur_radius(f64::INFINITY), 1);
    assert_eq!(filters::brightness_amount(30.0), 0.5);
    assert_eq!(filters::brightness_amount(48.0), 0.8);
    assert_eq!(filters::brightness_amount(-9.0), 0.05);
    assert_eq!(filters::brightness_amount(90.0), 0.9);
    assert_eq!(filters::brightness_amount(f64::NAN), 0.05);
}

/// 【滤镜测试】【颜色换算】灰度使用原版权重，反色和提亮保持直通通道计算与透明度
#[test]
fn color_filters_match_reference_pixels_and_preserve_alpha() {
    let pixels = [0xffc8_6432, 0x80c8_6432, 0x0014_c83c];
    let source = view(&pixels, 3, 1);
    let gray = filters::apply(source, filters::Filter::Grayscale).unwrap();
    let invert = filters::apply(source, filters::Filter::Invert).unwrap();
    let bright = filters::apply(source, filters::Filter::Brighten(0.5)).unwrap();
    assert_eq!(gray.pixels(), &[0xff7e_7e7e, 0x803f_3f3f, 0]);
    assert_eq!(invert.pixels(), &[0xff37_9bcd, 0x801c_4e67, 0]);
    assert_eq!(bright.pixels(), &[0xffe4_b299, 0x8072_594d, 0]);
    assert!(filters::apply(source, filters::Filter::Brighten(f64::NAN)).is_err());
}

/// 【滤镜测试】【块透明度】像素化直接平均预乘通道，不把平均颜色再乘一次透明度
#[test]
fn pixelation_averages_premultiplied_channels_once() {
    let pixels = [0x8064_3214, 0xffc8_6428, 0x8064_3214, 0xffc8_6428];
    let image = pixelate::apply(view(&pixels, 2, 2), 2).unwrap();
    assert_eq!(image.pixels(), &[0xc096_4b1e; 4]);
}

/// 【滤镜测试】【残余块】网格从区域左上角开始，最右列和最下行只平均有效像素
#[test]
fn pixelation_handles_partial_blocks_and_strided_input() {
    let pixels = [
        0xff00_0000,
        0xff00_0000,
        0xffff_ffff,
        0x1234_5678,
        0xff00_0000,
        0xff00_0000,
        0xffff_ffff,
        0x1234_5678,
        0xff14_283c,
        0xff28_3c50,
        0xff64_788c,
    ];
    let source = ImageView::new(&pixels, 3, 3, 4).unwrap();
    let image = pixelate::apply(source, 2).unwrap();
    assert_eq!(
        image.pixels(),
        &[
            0xff00_0000,
            0xff00_0000,
            0xffff_ffff,
            0xff00_0000,
            0xff00_0000,
            0xffff_ffff,
            0xff1e_3246,
            0xff1e_3246,
            0xff64_788c,
        ]
    );
}

/// 【滤镜测试】【模糊边界】三次盒式模糊柔化黑白边界，远端颜色和零半径原样保留
#[test]
fn gaussian_softens_edges_and_zero_radius_keeps_the_input() {
    let pixels: Vec<u32> = (0..400)
        .map(|index| {
            if index % 40 < 20 {
                0xff00_0000
            } else {
                0xffff_ffff
            }
        })
        .collect();
    let source = view(&pixels, 40, 10);
    let image = blur::gaussian(source, 6).unwrap();
    let channel = |x: usize| image.pixels()[5 * 40 + x] & 255;
    assert!((41..215).contains(&channel(20)));
    assert!(channel(0) < 20);
    assert!(channel(39) > 235);
    assert_eq!(blur::gaussian(source, 0).unwrap().pixels(), pixels);
    assert!(blur::gaussian(source, 4097).is_err());
}

/// 【滤镜测试】【窄图与透明度】模糊保留恒定预乘像素，一行或一列图像按原版规则保持不变
#[test]
fn gaussian_keeps_constant_alpha_and_one_dimensional_images() {
    let pixels = vec![0x8064_3214; 20 * 20];
    assert_eq!(
        blur::gaussian(view(&pixels, 20, 20), 48).unwrap().pixels(),
        pixels
    );
    assert_eq!(
        blur::gaussian(view(&pixels[..20], 20, 1), 48)
            .unwrap()
            .pixels(),
        &pixels[..20]
    );
    assert_eq!(
        blur::gaussian(view(&pixels[..20], 1, 20), 48)
            .unwrap()
            .pixels(),
        &pixels[..20]
    );
}
