//! 【线条测试】【控制框范围】验证实际控制框与命中一致，并完整落入动态损坏区域

use std::collections::HashMap;

use mark_shot::features::{
    arrow::ArrowStyle,
    line::{LineSkeleton, LineStyle},
};
use mark_shot::renderer::{Painter, draw_annotation};
use mark_shot::types::{Annotation, AnnotationShape};
use tiny_skia::{Color, Pixmap, Rect};

/// 【控制框测试】【标注构造】按 head、width 创建带外部控制点的 annotation；head 为空时创建直线，返回完整边界
fn annotation(head: Option<ArrowStyle>, width: f32) -> Annotation {
    let mut line = LineSkeleton::new((130.0, 180.0), (420.0, 210.0)).unwrap();
    line.controls.push((260.0, 70.0));
    let shape = match head {
        Some(style) => AnnotationShape::Arrow {
            line,
            style,
            line_style: LineStyle::Solid,
        },
        None => AnnotationShape::Line {
            line,
            style: LineStyle::Solid,
        },
    };
    let mut annotation = Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 1,
        shape,
        color: Color::TRANSPARENT,
        shadow_color: Color::TRANSPARENT,
        stroke_width: width,
        bbox: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【控制框测试】【控制框绘制】把 annotation 的选择界面按 scale 绘入透明画布，返回像素图；不连接桌面
fn chrome(annotation: &Annotation, scale: f32) -> Pixmap {
    let mut image = Pixmap::new((600.0 * scale) as u32, (400.0 * scale) as u32).unwrap();
    draw_annotation(
        &mut Painter::new(&mut image).with_scale(scale),
        annotation,
        true,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
    );
    image
}

/// 【控制框测试】【完整损坏覆盖】所有头部、两种笔宽及分数缩放下，实际外框、阴影和控制点像素都包含在编辑损坏范围
#[test]
fn all_visible_selection_pixels_are_inside_the_declared_editing_damage() {
    for head in std::iter::once(None).chain(ArrowStyle::ALL.into_iter().map(Some)) {
        for width in [3.0, 72.0] {
            let annotation = annotation(head, width);
            let damage = annotation.damage_bbox(true);
            for scale in [1.0, 1.2] {
                let image = chrome(&annotation, scale);
                let escaped =
                    image
                        .data()
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .enumerate()
                        .find(|(index, pixel)| {
                            let x = (*index % image.width() as usize) as f32 / scale;
                            let y = (*index / image.width() as usize) as f32 / scale;
                            pixel[3] > 0
                                && (x < damage.left()
                                    || x > damage.right()
                                    || y < damage.top()
                                    || y > damage.bottom())
                        });
                assert!(
                    escaped.is_none(),
                    "{head:?} width={width} scale={scale}: pixels outside {damage:?}: {escaped:?}"
                );
            }
        }
    }
}

/// 【控制框测试】【命中绘制一致】骨架外框的四个可命中角点附近必须存在真实控制线，不能再绘制二次留白外框
#[test]
fn painted_frame_corners_match_the_skeleton_hit_targets() {
    let annotation = annotation(Some(ArrowStyle::Fletched), 3.0);
    let frame = mark_shot::features::annotation_selection::chrome::bounds(&annotation).unwrap();
    let image = chrome(&annotation, 1.0);
    for (x, y) in [
        (frame.left(), frame.top()),
        (frame.right(), frame.top()),
        (frame.left(), frame.bottom()),
        (frame.right(), frame.bottom()),
    ] {
        let ink = (-6..=6).any(|dy| {
            (-6..=6).any(|dx| {
                image
                    .pixel(
                        (x.round() as i32 + dx) as u32,
                        (y.round() as i32 + dy) as u32,
                    )
                    .is_some_and(|pixel| pixel.alpha() > 64)
            })
        });
        assert!(
            ink,
            "the clickable frame corner ({x}, {y}) has no matching visible handle"
        );
    }
}
