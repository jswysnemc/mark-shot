//! 【工具区域测试】【内存与像素】通过实际帧入口验证工具正文不创建整屏缓存且逐像素保持原有结果

use crate::{
    app::region_rendering_test_support::{annotation, editor, equal, render},
    features::ruler::{Measurement, RulerGeometry},
    types::AnnotationShape,
};
use tiny_skia::Rect;

/// 【工具区域测试】【完整与局部】按shape和cached_pixels对照四种设备比例、整帧和反复局部重绘，返回值为空，同时检查真实缓存需求
fn compare(shape: AnnotationShape, cached_pixels: bool) {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for state in [&mut cached, &mut region] {
            state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
            state.ruler.label_bounds = state.selection.zone;
            state.annotations = vec![
                annotation(
                    1,
                    13.0,
                    AnnotationShape::Rectangle {
                        radius: 0.0,
                        fixed_ratio: false,
                        start: (90.375, 60.625),
                        end: (561.125, 401.875),
                        filled: true,
                    },
                ),
                annotation(2, 0.0, shape.clone()),
            ];
        }
        let label = format!("tool {:?} scale={scale}", shape);
        equal(
            &render(&mut region, None, false),
            &render(&mut cached, None, false),
            &label,
        );
        assert_eq!(
            region.region_rendering.cached_annotations, cached_pixels,
            "{label}"
        );
        if !cached_pixels {
            assert_eq!(region.annotations_layer[0].data().len(), 4, "{label}");
        }
        let whole = region.canvas[0].clone();
        for x in [111.375, 255.875, 339.75, 509.625] {
            for _ in 0..3 {
                render(
                    &mut region,
                    Rect::from_xywh(x, 50.375, 17.625, 398.125),
                    false,
                );
                equal(&region.canvas[0], &whole, &label);
            }
        }
        equal(
            &render(&mut region, None, true),
            &render(&mut cached, None, true),
            &label,
        );
        equal(&render(&mut region, None, false), &whole, &label);
    }
}

/// 【工具区域测试】【画笔正文】无参数，验证跨块曲线和半透明背景保持完整视口栅格，不放宽裁切差异，无返回值
#[test]
fn pen_body_preserves_the_original_full_viewport_rasterization() {
    compare(
        AnnotationShape::Pen {
            points: vec![
                (90.375, 75.625),
                (261.125, 131.875),
                (467.75, 391.25),
                (569.375, 401.25),
            ],
        },
        true,
    );
}

/// 【工具区域测试】【长度与角度】无参数，验证两类尺子正文、刻度和半透明混合不需要整屏缓存，无返回值
#[test]
fn ruler_bodies_do_not_allocate_an_output_sized_cache() {
    for geometry in [
        RulerGeometry::Length {
            start: (91.375, 90.625),
            end: (551.125, 381.875),
            rotation: 13.25,
        },
        RulerGeometry::Angle {
            points: [(311.375, 230.625), (571.125, 351.875), (111.75, 91.25)],
            count: 3,
        },
    ] {
        compare(
            AnnotationShape::Ruler {
                measurement: Measurement {
                    geometry,
                    pixel_scale: 1.0,
                    style_scale: 1.0,
                },
            },
            false,
        );
    }
}
