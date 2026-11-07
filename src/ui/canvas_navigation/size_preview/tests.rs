//! 【滚轮提示测试】【局部像素】完整气泡、颜色与尺寸轮廓在分数设备比例下保持相同栅格

use super::*;
use tiny_skia::Pixmap;

/// 【滚轮提示测试】【完整绘制】按canvas、sample、scale和dirty绘制同一提示，无返回值
fn render(canvas: &mut Pixmap, sample: Sample, scale: f32, dirty: Option<Rect>) {
    let mut fonts = crate::fonts::selection_ui::new();
    let mut cache = cosmic_text::SwashCache::new();
    let rect = Rect::from_xywh(171.5, 110.25, 90.0, 28.0).unwrap();
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut fonts, &mut cache);
    painter.clipped(dirty, |painter| {
        crate::ui::annotation_selection::control_raster::draw(
            painter,
            Rect::from_xywh(72.0, 40.0, 192.0, 105.0).unwrap(),
            |painter| draw(painter, rect, (120.25, 90.5), &sample.text(), sample),
        );
    });
}

/// 【滚轮提示测试】【部分恢复】三种尺寸示意在移位损坏范围内重绘，不能改变未恢复区域或加深半透明像素
#[test]
fn partial_value_badges_and_footprints_match_the_complete_frame() {
    for scale in [1.0, 1.25, 1.5] {
        for kind in [Kind::Stroke, Kind::Mosaic, Kind::Text] {
            let sample = Sample {
                kind,
                value: 40.0,
                image_scale: 1.0,
                color: tiny_skia::Color::from_rgba8(220, 50, 70, 255),
            };
            let mut base = Pixmap::new((400.0 * scale) as u32, (240.0 * scale) as u32).unwrap();
            base.fill(tiny_skia::Color::from_rgba8(55, 65, 75, 255));
            let mut full = base.clone();
            render(&mut full, sample, scale, None);
            let damage = Rect::from_xywh(95.25, 85.75, 105.0, 32.25).unwrap();
            let pixels = crate::renderer::scale_rect(&damage, scale)
                .unwrap()
                .round_out()
                .unwrap();
            let blank = base.clone_rect(pixels).unwrap();
            let mut partial = full.clone();
            crate::renderer::paste(&mut partial, &blank, pixels.x(), pixels.y());
            render(&mut partial, sample, scale, Some(damage));
            assert!(partial.data() == full.data(), "scale={scale} kind={kind:?}");
        }
    }
}
