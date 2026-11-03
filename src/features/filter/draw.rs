//! 【滤镜】【分块合成】按原始画布像素网格映射原图，旋转与局部重绘不会改变插值相位

use tiny_skia::{FillRule, IntRect, Mask, Path, Pixmap, PixmapPaint, Rect, Transform};

use super::{
    FilterMode, RegionFilter,
    geometry::{bounds_of, corners},
    raster::{FilterKey, PixelView, Raster, render_tile},
};
use crate::renderer::Painter;

const OUTPUT_TILE_PIXELS: i32 = 512;

/// 【滤镜】【正文绘制】将 filter 原图效果绘入 painter；opacity 控制透明度，cached 提供可选区域缓存，无返回值
pub fn draw(
    painter: &mut Painter<'_>,
    filter: RegionFilter,
    opacity: f32,
    cached: Option<&Raster>,
) {
    if !filter.valid() || !opacity.is_finite() || opacity <= 0.0 {
        return;
    }
    let scale = painter.scale();
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let Some(image) = painter.frozen_image() else {
        return;
    };
    let Some(key) = FilterKey::new(image, filter) else {
        return;
    };
    let Some(region) = key.grid.rect(key.region) else {
        return;
    };
    let Some(path) = filter.path(region) else {
        return;
    };
    let cached = cached.filter(|cached| cached.key == key);
    let origin = painter.origin();
    let phase = painter.pixel_phase();
    let size = painter.size();
    let Some(viewport) = Rect::from_xywh(origin.0, origin.1, size.0, size.1) else {
        return;
    };

    // 1. 【滤镜】【正文绘制】原图核心块互不重叠，平滑插值只读取相邻的一像素留量
    for core in key.tiles() {
        let Some(body) = key
            .grid
            .rect(core)
            .and_then(|rect| bounds_of(corners(rect).map(|point| filter.to_visual(point))))
            .and_then(|rect| rect.outset(1.0 / scale, 1.0 / scale))
        else {
            continue;
        };
        if body.intersect(&viewport).is_none() {
            continue;
        }
        let tile;
        let pixels = if let Some(cached) = cached {
            cached.view()
        } else {
            let Some(rendered) = render_tile(image, key, core) else {
                continue;
            };
            tile = rendered;
            tile.view()
        };
        draw_core(
            painter,
            filter,
            key,
            core,
            pixels,
            &path,
            body,
            opacity.min(1.0),
            phase,
            viewport,
        );
    }
}

/// 【滤镜】【核心块合成】把 filter 的 core 绘入 painter；key、pixels、path 提供原图与遮罩，body 固定分块边界，phase 固定输出相位，viewport 只限制可见块，opacity 控制透明度，无返回值
fn draw_core(
    painter: &mut Painter<'_>,
    filter: RegionFilter,
    key: FilterKey,
    core: IntRect,
    pixels: PixelView<'_>,
    path: &Path,
    body: Rect,
    opacity: f32,
    phase: (f64, f64),
    viewport: Rect,
) {
    let scale = painter.scale();
    let (Some(area), Some(visible)) = (
        pixel_bounds(body, scale, phase),
        pixel_bounds(viewport, scale, phase),
    ) else {
        return;
    };
    // 1. 【滤镜】【核心块合成】遮罩块始终锚定完整正文，局部裁切不改变扫描起点与抗锯齿覆盖率
    for y in (area.y()..area.bottom()).step_by(OUTPUT_TILE_PIXELS as usize) {
        for x in (area.x()..area.right()).step_by(OUTPUT_TILE_PIXELS as usize) {
            let width = (area.right() - x).min(OUTPUT_TILE_PIXELS) as u32;
            let height = (area.bottom() - y).min(OUTPUT_TILE_PIXELS) as u32;
            if IntRect::from_xywh(x, y, width, height)
                .and_then(|rect| rect.intersect(&visible))
                .is_none()
            {
                continue;
            }
            let Some(mut mask) = Mask::new(width, height) else {
                continue;
            };
            let logical_origin = (
                ((f64::from(x) + phase.0) / f64::from(scale)) as f32,
                ((f64::from(y) + phase.1) / f64::from(scale)) as f32,
            );
            let transform = Transform::from_translate(-logical_origin.0, -logical_origin.1)
                .post_scale(scale, scale);
            mask.fill_path(path, FillRule::Winding, true, transform);
            let Some(mut temporary) = Pixmap::new(width, height) else {
                continue;
            };
            fill_chunk(
                &mut temporary,
                mask.data(),
                filter,
                key,
                core,
                pixels,
                (x, y),
                scale,
                phase,
            );
            let local = painter.to_px(logical_origin.0, logical_origin.1);
            // 2. 【滤镜】【核心块合成】每个输出像素只归属一个原图块，半透明接缝不重复叠加
            painter.canvas().draw_pixmap(
                local.0.round() as i32,
                local.1.round() as i32,
                temporary.as_ref(),
                &PixmapPaint {
                    opacity,
                    ..PixmapPaint::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
}

/// 【滤镜】【输出像素边界】把 rect 按 scale、phase 映射到固定输出网格；返回向外对齐范围，空区域返回空值
fn pixel_bounds(rect: Rect, scale: f32, phase: (f64, f64)) -> Option<IntRect> {
    Rect::from_ltrb(
        (f64::from(rect.left()) * f64::from(scale) - phase.0) as f32,
        (f64::from(rect.top()) * f64::from(scale) - phase.1) as f32,
        (f64::from(rect.right()) * f64::from(scale) - phase.0) as f32,
        (f64::from(rect.bottom()) * f64::from(scale) - phase.1) as f32,
    )?
    .round_out()
}

/// 【滤镜】【输出像素计算】向 output 写入 coverage 遮罩内的 filter 像素；key、core、pixels、origin、scale、phase 固定原始画布坐标，无返回值
fn fill_chunk(
    output: &mut Pixmap,
    coverage: &[u8],
    filter: RegionFilter,
    key: FilterKey,
    core: IntRect,
    pixels: PixelView<'_>,
    origin: (i32, i32),
    scale: f32,
    phase: (f64, f64),
) {
    let width = output.width();
    let height = output.height();
    let data = output.data_mut();
    for y in 0..height {
        for x in 0..width {
            let index = y as usize * width as usize + x as usize;
            let mask = coverage[index];
            if mask == 0 {
                continue;
            }
            let point = (
                (f64::from(origin.0) + f64::from(x) + phase.0 + 0.5) / f64::from(scale),
                (f64::from(origin.1) + f64::from(y) + phase.1 + 0.5) / f64::from(scale),
            );
            let native = key.grid.to_pixel(filter.local_f64(point));
            let owner = (
                native.0.floor().clamp(
                    f64::from(key.region.left()),
                    f64::from(key.region.right() - 1),
                ) as i32,
                native.1.floor().clamp(
                    f64::from(key.region.top()),
                    f64::from(key.region.bottom() - 1),
                ) as i32,
            );
            if owner.0 < core.left()
                || owner.0 >= core.right()
                || owner.1 < core.top()
                || owner.1 >= core.bottom()
            {
                continue;
            }
            let pixel = pixels.sample(native, key.mode == FilterMode::Pixelate);
            for channel in 0..4 {
                data[index * 4 + channel] = multiply(pixel[channel], mask);
            }
        }
    }
}

/// 【滤镜】【覆盖率相乘】返回预乘通道 value 与 factor 的八位四舍五入结果
fn multiply(value: u8, factor: u8) -> u8 {
    ((u16::from(value) * u16::from(factor) + 127) / 255) as u8
}
