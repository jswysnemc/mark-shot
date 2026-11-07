//! 【尺子悬停】【局部缓冲】十字线与提示保留透明合成顺序，每个临时缓冲限定为固定像素块

use super::{Painter, rect_bounds, scale_rect};
use crate::features::{
    annotation_style::window::WindowUnits,
    ruler::{labels::InfoLabel, state::RulerHover},
};
use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{IntRect, Pixmap, Rect};

const SIDE: u32 = 512;
const PAD: u32 = 8;

/// 【尺子悬停】【可见内容】按area、hover和labels返回当前块是否含十字线或说明，padding保留抗锯齿边缘，不改变绘制顺序
fn contributes(area: Rect, hover: &RulerHover, labels: &[InfoLabel], padding: f32) -> bool {
    let line = hover.bounds.intersect(&area).is_some()
        && ((area.left() - padding <= hover.point.0 && hover.point.0 <= area.right() + padding)
            || (area.top() - padding <= hover.point.1 && hover.point.1 <= area.bottom() + padding));
    line || labels
        .iter()
        .chain(std::iter::once(&hover.label))
        .any(|label| {
            label
                .rect
                .outset(padding, padding)
                .and_then(|rect| rect.intersect(&area))
                .is_some()
        })
}

/// 【尺子悬停】【区域合成】按canvas、labels和hover绘制透明叠层，origin与scale映射窗口，dirty限制交付，fonts和cache提供字体，window提供逻辑单位，无返回值
pub(super) fn draw(
    canvas: &mut Pixmap,
    labels: &[InfoLabel],
    hover: &RulerHover,
    origin: (f32, f32),
    scale: f32,
    dirty: Option<&Rect>,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
    window: WindowUnits,
) {
    let Some((x, y, width, height)) = dirty
        .copied()
        .and_then(|area| scale_rect(&area, scale))
        .or_else(|| Rect::from_xywh(0.0, 0.0, canvas.width() as f32, canvas.height() as f32))
        .and_then(|area| rect_bounds(&area, canvas.width(), canvas.height()))
    else {
        return;
    };
    let damage = IntRect::from_xywh(x as i32, y as i32, width, height).unwrap();
    let viewport = IntRect::from_xywh(0, 0, canvas.width(), canvas.height()).unwrap();
    let mut parent = Painter::new(canvas)
        .with_scale(scale)
        .with_origin(origin)
        .with_window_units(window)
        .with_text(fonts, cache);
    // 1. 【尺子悬停】【固定块】像素原点取自完整窗口，局部损坏不改变十字线和提示的采样位置
    for top in (y / SIDE..=(y + height - 1) / SIDE).map(|row| row * SIDE) {
        for left in (x / SIDE..=(x + width - 1) / SIDE).map(|column| column * SIDE) {
            let bounds = IntRect::from_xywh(left as i32, top as i32, SIDE, SIDE).unwrap();
            let clipped = bounds.intersect(&damage).unwrap();
            let raster = IntRect::from_xywh(
                left as i32 - PAD as i32,
                top as i32 - PAD as i32,
                SIDE + PAD * 2,
                SIDE + PAD * 2,
            )
            .unwrap()
            .intersect(&viewport)
            .unwrap();
            let mut tile = Pixmap::new(raster.width(), raster.height()).unwrap();
            let local_origin = (
                origin.0 + raster.x() as f32 / scale,
                origin.1 + raster.y() as f32 / scale,
            );
            let global = Rect::from_xywh(
                local_origin.0,
                local_origin.1,
                raster.width() as f32 / scale,
                raster.height() as f32 / scale,
            )
            .unwrap();
            if !contributes(global, hover, labels, 2.0 / scale) {
                continue;
            }
            {
                let mut local = parent.sub(&mut tile).with_origin(local_origin);
                super::ruler_labels::draw(&mut local, labels);
                super::ruler_overlay::draw_hover(&mut local, hover);
            }
            // 2. 【尺子悬停】【单次交付】只合成已经恢复的损坏交集，外部像素不重复叠加半透明十字线或提示
            if clipped == raster {
                parent.draw_pixmap(raster.x(), raster.y(), &tile);
            } else if let Some(crop) = tile.clone_rect(
                IntRect::from_xywh(
                    clipped.x() - raster.x(),
                    clipped.y() - raster.y(),
                    clipped.width(),
                    clipped.height(),
                )
                .unwrap(),
            ) {
                parent.draw_pixmap(clipped.x(), clipped.y(), &crop);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【尺子悬停测试】【完整参考】按canvas、hover、origin和scale使用原有完整透明层绘制参考像素，无返回值
    fn reference(canvas: &mut Pixmap, hover: &RulerHover, origin: (f32, f32), scale: f32) {
        let mut fonts = FontSystem::new();
        let mut cache = SwashCache::new();
        let mut full = Pixmap::new(canvas.width(), canvas.height()).unwrap();
        super::super::ruler_overlay::draw_hover(
            &mut Painter::new(&mut full)
                .with_scale(scale)
                .with_origin(origin)
                .with_text(&mut fonts, &mut cache),
            hover,
        );
        Painter::new(canvas).draw_pixmap(0, 0, &full);
    }

    /// 【尺子悬停测试】【原有像素】无参数，对照四种比例下跨块十字线与提示的完整透明合成，无返回值
    #[test]
    fn tiled_hover_keeps_full_overlay_pixels() {
        for scale in [1.0, 1.2, 1.5, 2.0] {
            let origin = (0.0, 0.0);
            let hover = RulerHover {
                point: (511.375, 391.625),
                bounds: Rect::from_xywh(0.0, 0.0, 1280.0, 960.0).unwrap(),
                label: InfoLabel {
                    rect: Rect::from_xywh(521.375, 401.625, 149.25, 49.5).unwrap(),
                    unit: 1.0,
                    lines: vec!["X: 511  Y: 392".into(), "RGB: 90, 120, 200".into()],
                    font_size: 13.0,
                },
            };
            let mut expected =
                Pixmap::new((1280.0 * scale) as u32, (960.0 * scale) as u32).unwrap();
            expected.fill(tiny_skia::Color::from_rgba8(90, 120, 200, 255));
            let mut actual = expected.clone();
            reference(&mut expected, &hover, origin, scale);
            draw(
                &mut actual,
                &[],
                &hover,
                origin,
                scale,
                None,
                &mut FontSystem::new(),
                &mut SwashCache::new(),
                Default::default(),
            );
            crate::app::region_rendering_test_support::equal(
                &actual,
                &expected,
                &format!("hover scale={scale}"),
            );
            // 1. 【尺子悬停测试】【重复损坏】恢复窄范围后重新绘制，未恢复像素不能重复叠加十字线或说明
            for dirty in [
                Rect::from_xywh(499.375, 0.0, 22.5, 960.0).unwrap(),
                Rect::from_xywh(0.0, 379.625, 1280.0, 71.5).unwrap(),
            ] {
                let (x, y, width, height) = scale_rect(&dirty, scale)
                    .and_then(|area| rect_bounds(&area, actual.width(), actual.height()))
                    .unwrap();
                for _ in 0..3 {
                    for row in y..y + height {
                        let start = (row * actual.width() + x) as usize * 4;
                        for pixel in actual.data_mut()[start..start + width as usize * 4]
                            .as_chunks_mut::<4>()
                            .0
                        {
                            pixel.copy_from_slice(&[90, 120, 200, 255]);
                        }
                    }
                    draw(
                        &mut actual,
                        &[],
                        &hover,
                        origin,
                        scale,
                        Some(&dirty),
                        &mut FontSystem::new(),
                        &mut SwashCache::new(),
                        Default::default(),
                    );
                    crate::app::region_rendering_test_support::equal(
                        &actual,
                        &expected,
                        &format!("hover partial scale={scale}"),
                    );
                }
            }
        }
    }
}
