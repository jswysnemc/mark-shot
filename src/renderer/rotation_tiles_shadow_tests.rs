//! 【旋转阴影测试】【窗口范围】旋转像素块覆盖放大的窗口阴影，尾部不在块边界截断

use super::*;
use crate::{
    features::{
        annotation_rotation::geometry,
        annotation_style::window::WindowUnits,
        arrow::{ArrowStyle, path},
        line::{LineSkeleton, LineStyle},
    },
    renderer::{annotation_rotation, blurred_shadow},
    types::AnnotationShape,
};
use tiny_skia::{Color, Rect};

/// 【旋转阴影测试】【箭头构造】按 shift 平移旋转箭头，使阴影经过不同物理块边界；返回标注及正文路径
fn annotation(shift: f32) -> (Annotation, tiny_skia::Path) {
    let line = LineSkeleton::new((235.25 + shift, 200.5), (500.25 + shift, 240.5)).unwrap();
    let outline = path::outline(&line, ArrowStyle::Direct, LineStyle::Solid, 8.0).unwrap();
    let mut annotation = Annotation {
        style_scale: 1.0,
        rotation_degrees: 31.0,
        frozen_bounds: None,
        id: 9,
        shape: AnnotationShape::Arrow {
            line,
            style: ArrowStyle::Direct,
            line_style: LineStyle::Solid,
        },
        color: Color::BLACK,
        shadow_color: Color::from_rgba8(15, 30, 45, 150),
        stroke_width: 8.0,
        bbox: Rect::from_xywh(235.25 + shift, 200.5, 265.0, 40.0).unwrap(),
    };
    annotation.update_bbox();
    (annotation, outline)
}

/// 【旋转阴影测试】【完整对照】无参数，四倍窗口单位下固定旋转块与直接完整场景绘制的阴影通道误差不超过一级，无返回值
#[test]
fn enlarged_window_shadows_keep_the_full_rotated_tail() {
    for scale in [0.25, 0.5, 1.0] {
        for shift in [0.0, 37.0, 91.0] {
            let (annotation, path) = annotation(shift);
            let mut tiled = Pixmap::new(960, 600).unwrap();
            annotation_rotation::draw(
                &mut Painter::new(&mut tiled)
                    .with_scale(scale)
                    .with_window_units(WindowUnits::new(4.0)),
                &annotation,
                None,
                false,
                |painter| blurred_shadow::draw(painter, &path, None, annotation.shadow_color),
            );
            let mut full = Pixmap::new(960, 600).unwrap();
            Painter::new(&mut full)
                .with_scale(scale)
                .with_window_units(WindowUnits::new(4.0))
                .transformed(
                    geometry::transform(annotation.rotation_center(), annotation.rotation_degrees),
                    |painter| blurred_shadow::draw(painter, &path, None, annotation.shadow_color),
                );
            assert!(full.pixels().iter().any(|pixel| pixel.alpha() > 0));
            let worst = full
                .data()
                .iter()
                .zip(tiled.data())
                .map(|(expected, actual)| expected.abs_diff(*actual))
                .max()
                .unwrap_or(0);
            assert!(
                worst <= 1,
                "scale={scale}, shift={shift}, clipped shadow error={worst}"
            );
        }
    }
}
