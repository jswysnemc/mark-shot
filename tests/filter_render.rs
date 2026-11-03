//! 【滤镜测试】【屏幕合成】验证透明接缝、冻结来源、缩放旋转与精确局部重绘

#[path = "fixtures/filter_pixels.rs"]
mod pixels;

use mark_shot::features::filter::{
    FilterMode, RegionFilter, draw,
    raster::{FilterKey, Raster},
};
use mark_shot::renderer::{FrozenImage, Painter, draw_annotation};
use mark_shot::types::{Annotation, AnnotationShape};
use std::collections::HashMap;
use tiny_skia::{Color, IntRect, Pixmap, Rect};

/// 【滤镜测试】【绘制辅助】将 source 对应 filter 绘入透明层；scale、opacity、cached 控制显示，返回完整像素图
fn render(source: &Pixmap, filter: RegionFilter, scale: f32, opacity: f32, cached: bool) -> Pixmap {
    let image = FrozenImage::image(
        source,
        Rect::from_xywh(0.0, 0.0, source.width() as f32, source.height() as f32).unwrap(),
    );
    let raster =
        cached.then(|| Raster::new(image, FilterKey::new(image, filter).unwrap()).unwrap());
    let mut output = Pixmap::new(
        (source.width() as f32 * scale).round() as u32,
        (source.height() as f32 * scale).round() as u32,
    )
    .unwrap();
    draw::draw(
        &mut Painter::new(&mut output)
            .with_scale(scale)
            .with_frozen_image(image),
        filter,
        opacity,
        raster.as_ref(),
    );
    output
}

/// 【滤镜测试】【缓存与分块】五模式在旋转和分数缩放下，缓存与按需分块结果逐像素一致
#[test]
fn cached_and_uncached_draws_match_at_rotated_fractional_scales() {
    let source = pixels::image(700, 640);
    for mode in FilterMode::ALL {
        for rotation in [0.0, 0.37] {
            let mut filter = RegionFilter::new((17.0, 13.0), (680.0, 620.0), 14.0, mode).unwrap();
            filter.rotation = rotation;
            for scale in [0.73, 1.2] {
                pixels::same(
                    &render(&source, filter, scale, 0.51, true),
                    &render(&source, filter, scale, 0.51, false),
                );
            }
        }
    }
}

/// 【滤镜测试】【局部重绘】独立损坏缓冲跨越横纵原图块边界，全部模式与完整绘制的裁切像素一致
#[test]
fn partial_redraws_keep_global_sampling_and_coverage_phase() {
    let source = pixels::image(700, 640);
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 700.0, 640.0).unwrap());
    for mode in FilterMode::ALL {
        let mut filter = RegionFilter::new((17.0, 13.0), (680.0, 620.0), 14.0, mode).unwrap();
        filter.rotation = 0.37;
        for scale in [0.73, 1.2] {
            let whole = render(&source, filter, scale, 0.51, false);
            let at = IntRect::from_xywh(
                whole.width() as i32 / 2 - 20,
                whole.height() as i32 / 2 - 20,
                220,
                180,
            )
            .unwrap();
            let expected = whole.clone_rect(at).unwrap();
            let mut actual = Pixmap::new(at.width(), at.height()).unwrap();
            draw::draw(
                &mut Painter::new(&mut actual)
                    .with_scale(scale)
                    .with_origin((at.x() as f32 / scale, at.y() as f32 / scale))
                    .with_frozen_image(image),
                filter,
                0.51,
                None,
            );
            pixels::same(&actual, &expected);
        }
    }
}

/// 【滤镜测试】【透明接缝】半透明原图和半透明滤镜跨越块边界时只覆盖一次，原图平均值不重复预乘
#[test]
fn translucent_tiles_never_double_cover_shared_edges() {
    let mut source = Pixmap::new(700, 640).unwrap();
    source.fill(Color::from_rgba8(100, 50, 25, 128));
    let filter = RegionFilter::new((0.0, 0.0), (700.0, 640.0), 14.0, FilterMode::Pixelate).unwrap();
    for cached in [false, true] {
        let output = render(&source, filter, 1.0, 0.5, cached);
        let expected = pixels::pixel(&output, 300, 300);
        assert_eq!(expected[3], 64);
        for x in [1, 14, 511, 512, 513, 698] {
            for y in [1, 14, 511, 512, 513, 638] {
                assert_eq!(
                    pixels::pixel(&output, x, y),
                    expected,
                    "{x}, {y}, cached={cached}"
                );
            }
        }
    }
}

/// 【滤镜测试】【原图依赖】没有冻结原图时不从当前画布反色，零透明度不会修改任何像素
#[test]
fn missing_frozen_source_and_zero_opacity_leave_the_canvas_unchanged() {
    let source = pixels::image(80, 60);
    let filter = RegionFilter::new((5.0, 5.0), (75.0, 55.0), 14.0, FilterMode::Invert).unwrap();
    let mut output = source.clone();
    draw::draw(&mut Painter::new(&mut output), filter, 1.0, None);
    pixels::same(&output, &source);
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 80.0, 60.0).unwrap());
    draw::draw(
        &mut Painter::new(&mut output).with_frozen_image(image),
        filter,
        0.0,
        None,
    );
    pixels::same(&output, &source);
}

/// 【滤镜测试】【原图像素相位】原图导出带非整数逻辑原点时，逐像素变换不能混入相邻像素或改变透明度
#[test]
fn native_export_with_a_fractional_global_origin_keeps_exact_filter_pixels() {
    let source = pixels::image(300, 220);
    let viewport = Rect::from_xywh(17.0, 33.0, 137.0, 220.0 * 137.0 / 300.0).unwrap();
    let scale = source.width() as f32 / viewport.width();
    let image = FrozenImage::image(&source, viewport).mapped(scale, (0.0, 0.0));
    let filter = RegionFilter::new(
        (viewport.left(), viewport.top()),
        (viewport.right(), viewport.bottom()),
        14.0,
        FilterMode::Invert,
    )
    .unwrap()
    .scaled(scale);
    let key = FilterKey::new(image, filter).unwrap();
    let expected = Raster::new(image, key).unwrap();
    let mut output = Pixmap::new(300, 220).unwrap();
    draw::draw(
        &mut Painter::new(&mut output)
            .with_origin((viewport.left() * scale, viewport.top() * scale))
            .with_frozen_image(image),
        filter,
        1.0,
        None,
    );
    pixels::same(&output, &expected.pixels);
}

/// 【滤镜测试】【子缓冲相位】带分数原点的局部缓冲继承父画布相位，旋转遮罩和颜色逐像素保持一致
#[test]
fn child_buffers_inherit_the_fractional_parent_pixel_grid() {
    let source = pixels::image(300, 220);
    let viewport = Rect::from_xywh(17.3, 33.1, 300.0, 220.0).unwrap();
    let image = FrozenImage::image(&source, viewport);
    let mut filter =
        RegionFilter::new((65.0, 80.0), (260.0, 210.0), 14.0, FilterMode::Invert).unwrap();
    filter.rotation = 0.37;
    let scale = 1.2;
    let mut whole = Pixmap::new(360, 264).unwrap();
    let at = IntRect::from_xywh(40, 35, 210, 180).unwrap();
    let mut partial = Pixmap::new(at.width(), at.height()).unwrap();
    let mut painter = Painter::new(&mut whole)
        .with_scale(scale)
        .with_origin((viewport.left(), viewport.top()))
        .with_frozen_image(image);
    draw::draw(&mut painter, filter, 0.51, None);
    let phase = painter.pixel_phase();
    let origin = painter.origin();
    let mut child = painter.sub(&mut partial).with_origin((
        origin.0 + at.x() as f32 / scale,
        origin.1 + at.y() as f32 / scale,
    ));
    assert_eq!(child.pixel_phase(), phase);
    draw::draw(&mut child, filter, 0.51, None);
    pixels::same(&partial, &whole.clone_rect(at).unwrap());
}

/// 【滤镜测试】【标注层序】滤镜始终处理冻结蓝色图像而不是已有红色标注，较新的标注仍可覆盖滤镜
#[test]
fn filters_sample_frozen_pixels_and_keep_annotation_order() {
    let mut source = Pixmap::new(160, 100).unwrap();
    source.fill(Color::from_rgba8(10, 30, 50, 255));
    let bounds = Rect::from_xywh(20.0, 20.0, 120.0, 60.0).unwrap();
    let mut old = Annotation {
        id: 1,
        shape: AnnotationShape::Rectangle {
            start: (20.0, 20.0),
            end: (140.0, 80.0),
            filled: true,
        },
        color: Color::from_rgba8(255, 0, 0, 255),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 1.0,
        bbox: bounds,
    };
    old.update_bbox();
    let filter = Annotation {
        id: 2,
        shape: AnnotationShape::Redact {
            filter: RegionFilter::new((20.0, 20.0), (140.0, 80.0), 14.0, FilterMode::Invert)
                .unwrap(),
        },
        color: Color::WHITE,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 0.0,
        bbox: bounds,
    };
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 160.0, 100.0).unwrap());
    let mut output = source.clone();
    let mut painter = Painter::new(&mut output).with_frozen_image(image);
    let mut editors = HashMap::new();
    let cache = HashMap::new();
    draw_annotation(&mut painter, &old, false, &mut editors, &cache, None);
    draw_annotation(&mut painter, &filter, false, &mut editors, &cache, None);
    assert_eq!(
        pixels::pixel(painter.canvas(), 80, 50),
        [245, 225, 205, 255]
    );
    draw_annotation(&mut painter, &old, false, &mut editors, &cache, None);
    assert_eq!(pixels::pixel(painter.canvas(), 80, 50), [255, 0, 0, 255]);
}
