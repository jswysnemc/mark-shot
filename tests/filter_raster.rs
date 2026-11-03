//! 【滤镜测试】【原图区域】验证分块结果与完整参考算法一致，网格和模糊留量不改变接缝

#[path = "fixtures/filter_pixels.rs"]
mod pixels;

use mark_shot::features::filter::{
    FilterMode, RegionFilter,
    raster::{FilterKey, MAX_CACHED_BYTES, Raster},
};
use mark_shot::renderer::FrozenImage;
use shot_image::{ImageView, blur, filters, pixelate};
use tiny_skia::{Color, IntRect, Pixmap, Rect};

/// 【滤镜测试】【整数格式】把 image 转为 packed ARGB；straight 为真时先反预乘，返回紧凑像素数组
fn argb(image: &Pixmap, straight: bool) -> Vec<u32> {
    image
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| {
            let alpha = u32::from(pixel[3]);
            let channel = |value: u8| {
                if straight && alpha != 0 {
                    ((u32::from(value) * 255 + alpha / 2) / alpha).min(255)
                } else {
                    u32::from(value)
                }
            };
            (alpha << 24) | (channel(pixel[0]) << 16) | (channel(pixel[1]) << 8) | channel(pixel[2])
        })
        .collect()
}

/// 【滤镜测试】【像素格式】把 image 的预乘 ARGB 转回 RGBA；返回测试像素图
fn rgba(image: &shot_image::Image) -> Pixmap {
    let mut output = Pixmap::new(image.width() as u32, image.height() as u32).unwrap();
    for (pixel, value) in output
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(image.pixels())
    {
        *pixel = [
            ((value >> 16) & 255) as u8,
            ((value >> 8) & 255) as u8,
            (value & 255) as u8,
            (value >> 24) as u8,
        ];
    }
    output
}

/// 【滤镜测试】【完整参考】按 source、key 直接处理完整区域；返回与原版同参数的参考结果，不使用区域分块实现
fn reference(source: &Pixmap, key: FilterKey) -> Pixmap {
    let input_rect = if key.mode == FilterMode::Blur {
        let radius = filters::blur_radius(f64::from(key.strength));
        key.region
            .make_outset(radius * 2, radius * 2)
            .unwrap()
            .intersect(&IntRect::from_xywh(0, 0, source.width(), source.height()).unwrap())
            .unwrap()
    } else {
        key.region
    };
    let input = source.clone_rect(input_rect).unwrap();
    let values = argb(
        &input,
        !matches!(key.mode, FilterMode::Blur | FilterMode::Pixelate),
    );
    let view = ImageView::new(
        &values,
        input.width() as i32,
        input.height() as i32,
        input.width() as usize,
    )
    .unwrap();
    let filtered = match key.mode {
        FilterMode::Pixelate => pixelate::apply(view, key.strength.round() as i32).unwrap(),
        FilterMode::Blur => {
            blur::gaussian(view, filters::blur_radius(f64::from(key.strength))).unwrap()
        }
        FilterMode::Grayscale => filters::apply(view, filters::Filter::Grayscale).unwrap(),
        FilterMode::Invert => filters::apply(view, filters::Filter::Invert).unwrap(),
        FilterMode::Brighten => filters::apply(
            view,
            filters::Filter::Brighten(filters::brightness_amount(f64::from(key.strength))),
        )
        .unwrap(),
    };
    let filtered = rgba(&filtered);
    filtered
        .clone_rect(
            key.region
                .translate(-input_rect.x(), -input_rect.y())
                .unwrap(),
        )
        .unwrap()
}

/// 【滤镜测试】【全部模式分块】五模式乘三种强度跨越横纵块边界，结果逐像素等于完整处理
#[test]
fn every_mode_and_strength_matches_whole_region_reference() {
    let source = pixels::image(690, 620);
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 690.0, 620.0).unwrap());
    for mode in FilterMode::ALL {
        for strength in [4.0, 14.0, 48.0] {
            let filter = RegionFilter::new((31.0, 27.0), (640.0, 598.0), strength, mode).unwrap();
            let key = FilterKey::new(image, filter).unwrap();
            let actual = Raster::new(image, key).unwrap();
            pixels::same(&actual.pixels, &reference(&source, key));
        }
    }
}

/// 【滤镜测试】【区域外模糊】模糊会混入区域外白色，像素化不会把区域外颜色带入
#[test]
fn only_blur_reads_pixels_outside_the_annotation_rectangle() {
    let mut source = Pixmap::new(160, 100).unwrap();
    source.fill(Color::WHITE);
    for y in 20..80 {
        for x in 40..120 {
            let offset = (y * 160 + x) * 4;
            source.data_mut()[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
    }
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 160.0, 100.0).unwrap());
    for mode in [FilterMode::Blur, FilterMode::Pixelate] {
        let filter = RegionFilter::new((40.0, 20.0), (120.0, 80.0), 14.0, mode).unwrap();
        let actual = Raster::new(image, FilterKey::new(image, filter).unwrap()).unwrap();
        let edge = pixels::pixel(&actual.pixels, 0, 30)[0];
        assert_eq!(edge > 0, mode == FilterMode::Blur);
    }
}

/// 【滤镜测试】【导出参数】原图缩放和坐标偏移不改变任何模式的像素，提亮强度不随导出比例增长
#[test]
fn native_pixels_and_strength_survive_preview_and_export_mapping() {
    let source = pixels::image(700, 640);
    let viewport = Rect::from_xywh(-30.0, 20.0, 280.0, 256.0).unwrap();
    let image = FrozenImage::image(&source, viewport);
    for mode in FilterMode::ALL {
        let filter = RegionFilter::new((10.0, 35.0), (170.0, 240.0), 14.0, mode).unwrap();
        let first = Raster::new(image, FilterKey::new(image, filter).unwrap()).unwrap();
        for scale in [0.73, 1.2, 2.5] {
            let offset = (-23.0, 15.0);
            let mapped = image.mapped(scale, offset);
            let filter = filter.scaled(scale).translated(offset);
            let other = Raster::new(mapped, FilterKey::new(mapped, filter).unwrap()).unwrap();
            assert_eq!(first.key.region, other.key.region);
            assert_eq!(other.key.strength, 14.0);
            pixels::same(&first.pixels, &other.pixels);
        }
    }
}

/// 【滤镜测试】【旋转缓存键】旋转只改变绘制位置，不重算原图；模式或强度变化产生新缓存键
#[test]
fn rotation_does_not_invalidate_frozen_pixels_but_properties_do() {
    let source = pixels::image(100, 80);
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 100.0, 80.0).unwrap());
    let filter = RegionFilter::new((10.0, 10.0), (70.0, 60.0), 14.0, FilterMode::Blur).unwrap();
    let key = FilterKey::new(image, filter).unwrap();
    assert_eq!(
        FilterKey::new(
            image,
            RegionFilter {
                rotation: 0.7,
                ..filter
            }
        ),
        Some(key)
    );
    assert_ne!(
        FilterKey::new(
            image,
            RegionFilter {
                strength: 20.0,
                ..filter
            }
        ),
        Some(key)
    );
    assert_ne!(
        FilterKey::new(
            image,
            RegionFilter {
                mode: FilterMode::Brighten,
                ..filter
            }
        ),
        Some(key)
    );
}

/// 【滤镜测试】【单缓存上限】超过四 MiB 的区域不分配完整缓存，仍可由五百一十二像素块处理
#[test]
fn large_regions_skip_whole_image_cache() {
    let source = pixels::image(1600, 900);
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 1600.0, 900.0).unwrap());
    let filter =
        RegionFilter::new((0.0, 0.0), (1600.0, 900.0), 14.0, FilterMode::Grayscale).unwrap();
    let key = FilterKey::new(image, filter).unwrap();
    assert!(key.bytes().unwrap() > MAX_CACHED_BYTES);
    assert!(Raster::new(image, key).is_none());
    let tile =
        mark_shot::features::filter::raster::render_tile(image, key, key.tiles().next().unwrap())
            .unwrap();
    assert!(tile.pixels.width() <= 514 && tile.pixels.height() <= 514);
}
