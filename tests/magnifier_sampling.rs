//! 【放大镜测试】【冻结采样】验证原图、透明度、多输出与导出坐标，不读取编辑图层

use mark_shot::renderer::FrozenImage;
use mark_shot::types::{Capture, Placement};
use tiny_skia::{Color, Pixmap, Rect};

/// 【采样测试】【双色图】返回两个预乘像素，左侧不透明红色，右侧完全透明
fn image() -> Pixmap {
    let mut pixels = Pixmap::new(2, 1).unwrap();
    pixels.data_mut()[..4].copy_from_slice(&[255, 0, 0, 255]);
    pixels
}

/// 【采样测试】【像素中心】恒等采样返回原始像素，双线性边界保持预乘透明度
#[test]
fn pixel_centers_and_interpolation_preserve_premultiplied_alpha() {
    let pixels = image();
    let source = FrozenImage::image(&pixels, Rect::from_xywh(-10.0, 20.0, 2.0, 1.0).unwrap());
    assert_eq!(source.sample((-9.5, 20.5)), Some([255, 0, 0, 255]));
    assert_eq!(source.sample((-8.5, 20.5)), Some([0, 0, 0, 0]));
    assert_eq!(source.sample((-9.0, 20.5)), Some([128, 0, 0, 128]));
    assert_eq!(source.sample((-8.0, 20.5)), None);
    assert_eq!(source.sample((f32::NAN, 20.5)), None);
}

/// 【采样测试】【原图分辨率】缩小预览仍读取原始像素，坐标映射不使用屏幕缓存或标注图层
#[test]
fn a_downscaled_viewport_still_samples_the_original_pixels() {
    let pixels = image();
    let source = FrozenImage::image(&pixels, Rect::from_xywh(100.0, 40.0, 1.0, 0.5).unwrap());
    assert_eq!(source.sample((100.25, 40.25)), Some([255, 0, 0, 255]));
    assert_eq!(source.sample((100.75, 40.25)), Some([0, 0, 0, 0]));
}

/// 【采样测试】【导出映射】改变绘图比例和原点仍读取相同原图像素，无效映射保持原有视图
#[test]
fn export_scale_and_crop_offset_map_back_to_the_same_frozen_pixels() {
    let pixels = image();
    let source = FrozenImage::image(&pixels, Rect::from_xywh(-10.0, 20.0, 2.0, 1.0).unwrap());
    let mapped = source.mapped(1.6, (40.0, -20.0));
    let point = (-9.5 * 1.6 + 40.0, 20.5 * 1.6 - 20.0);
    assert_eq!(mapped.sample(point), source.sample((-9.5, 20.5)));
    assert_eq!(
        source
            .mapped(f32::INFINITY, (0.0, 0.0))
            .sample((-9.5, 20.5)),
        source.sample((-9.5, 20.5))
    );
}

/// 【采样测试】【输出边界】负坐标与分数缩放输出使用各自冻结图像；桌面间隙不读取错误显示器
#[test]
fn desktop_sampling_respects_offsets_scales_and_uncovered_gaps() {
    let mut red = Pixmap::new(4, 4).unwrap();
    red.fill(Color::from_rgba8(255, 0, 0, 255));
    let mut blue = Pixmap::new(3, 3).unwrap();
    blue.fill(Color::from_rgba8(0, 0, 255, 255));
    let captures = [
        Capture {
            pixmap: red.into(),
            scale: 2.0,
        },
        Capture {
            pixmap: blue.into(),
            scale: 1.5,
        },
    ];
    let placements = [
        Placement {
            position: (-4, 10),
            size: (2, 2),
            scale: 2.0,
        },
        Placement {
            position: (0, 10),
            size: (2, 2),
            scale: 1.5,
        },
    ];
    let source = FrozenImage::desktop(&captures, &placements);
    assert_eq!(source.sample((-3.5, 10.5)), Some([255, 0, 0, 255]));
    assert_eq!(source.sample((0.5, 10.5)), Some([0, 0, 255, 255]));
    assert_eq!(source.sample((-1.0, 10.5)), None);
    assert_eq!(source.sample((2.0, 10.5)), None);
}
