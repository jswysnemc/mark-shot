//! 【渲染】【柔和阴影】统一窗口单位，以有限像素块模糊轮廓并合成阴影

use tiny_skia::{Color, FillRule, IntRect, Paint, Path, Pixmap, Rect, Stroke, Transform};

use super::Painter;

const TILE_PIXELS: i32 = 128;
const MAX_BLUR_PIXELS: f32 = 64.0;

/// 【渲染】【阴影留量】按窗口逻辑尺寸 blur 返回保守模糊外伸；非正值或非有限值返回零
pub(crate) fn padding(blur: f32) -> f32 {
    if blur.is_finite() && blur > 0.0 {
        blur * 1.5 + 3.0
    } else {
        0.0
    }
}

/// 【渲染】【柔和阴影】将 path 以阴影 color 绘入 painter；stroke 为描边参数，None 表示填充，无返回值
/// 偏移和模糊配置采用窗口逻辑像素，先换算画布单位，再应用设备比例与场景变换
pub fn draw(painter: &mut Painter<'_>, path: &Path, stroke: Option<&Stroke>, color: Color) {
    let shadow = &crate::config::get().annotations.shadow;
    let [dx, dy] = shadow.offset;
    let scale = painter.scale();
    if !dx.is_finite()
        || !dy.is_finite()
        || color.alpha() <= 0.0
        || !scale.is_finite()
        || scale <= 0.0
    {
        return;
    }
    // 1. 【渲染】【阴影轮廓】窗口偏移和模糊独立于设备比例，描边先转换成可填充轮廓
    let units = painter.window_units();
    let transform = painter.transform_shifted(units.canvas(dx), units.canvas(dy));
    let silhouette = match stroke {
        Some(stroke) => path.stroke(stroke, 1.0 / scale),
        None => Some(path.clone()),
    };
    let Some(device) = silhouette.and_then(|path| path.transform(transform)) else {
        return;
    };
    let radius = if shadow.blur.is_finite() && shadow.blur > 0.0 {
        (units.canvas(shadow.blur) * scale)
            .clamp(1.0, MAX_BLUR_PIXELS)
            .round() as i32
    } else {
        0
    };
    if radius == 0 {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        painter.canvas().fill_path(
            &device,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        return;
    }
    // 2. 【渲染】【阴影支持范围】三次盒式滤波需要半径总和，块外像素参与计算但不重复合成
    let support = shot_image::blur::radii((f64::from(radius) / 2.0).max(1.0))
        .iter()
        .sum::<i32>()
        + 2;
    draw_tiles(painter, &device, color, radius, support);
}

/// 【渲染】【阴影分块】按 radius 和 support 将设备轮廓 device、color 合成到 painter；只生成可见块，无返回值
fn draw_tiles(painter: &mut Painter<'_>, device: &Path, color: Color, radius: i32, support: i32) {
    let canvas = painter.canvas();
    let Some(canvas_rect) =
        Rect::from_xywh(0.0, 0.0, canvas.width() as f32, canvas.height() as f32)
    else {
        return;
    };
    let Some(visible) = device
        .bounds()
        .outset(support as f32, support as f32)
        .and_then(|bounds| bounds.intersect(&canvas_rect))
        .and_then(|bounds| bounds.round_out())
    else {
        return;
    };
    // 1. 【渲染】【有限缓冲】每块至多一百二十八像素加滤波支持范围，空心框内部不执行模糊
    for y in (visible.y()..visible.bottom()).step_by(TILE_PIXELS as usize) {
        for x in (visible.x()..visible.right()).step_by(TILE_PIXELS as usize) {
            let width = (visible.right() - x).min(TILE_PIXELS) as u32;
            let height = (visible.bottom() - y).min(TILE_PIXELS) as u32;
            let Some(area) = IntRect::from_xywh(
                x - support,
                y - support,
                width + 2 * support as u32,
                height + 2 * support as u32,
            ) else {
                continue;
            };
            let Some(mask) = blurred_tile(device, color, area, radius) else {
                continue;
            };
            let Some(core) = IntRect::from_xywh(support, support, width, height)
                .and_then(|core| mask.clone_rect(core))
            else {
                continue;
            };
            // 2. 【渲染】【阴影合成】仅合成互不重叠的核心区，块间支持像素不会增加透明度
            painter.draw_pixmap(x, y, &core);
        }
    }
}

/// 【渲染】【阴影像素】按设备轮廓 device、阴影 color、物理 area 和模糊 radius 返回预乘像素块，空块或分配失败返回 None
fn blurred_tile(device: &Path, color: Color, area: IntRect, radius: i32) -> Option<Pixmap> {
    let local = device.clone().transform(Transform::from_translate(
        -(area.x() as f32),
        -(area.y() as f32),
    ))?;
    let mut mask = Pixmap::new(area.width(), area.height())?;
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    mask.fill_path(
        &local,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    if mask.pixels().iter().all(|pixel| pixel.alpha() == 0) {
        return None;
    }
    // 1. 【渲染】【像素格式】共享图像核心接收预乘 ARGB，颜色和透明度同时参与滤波
    let pixels = mask
        .pixels()
        .iter()
        .map(|pixel| u32::from_be_bytes([pixel.alpha(), pixel.red(), pixel.green(), pixel.blue()]))
        .collect();
    let blurred =
        shot_image::blur::gaussian_owned(pixels, area.width() as i32, area.height() as i32, radius)
            .ok()?;
    // 2. 【渲染】【缓冲复用】模糊结果写回原掩码，不再分配同尺寸的第二张 RGBA 图像
    for (output, pixel) in mask
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(blurred.pixels())
    {
        let [alpha, red, green, blue] = pixel.to_be_bytes();
        output.copy_from_slice(&[red, green, blue, alpha]);
    }
    Some(mask)
}
