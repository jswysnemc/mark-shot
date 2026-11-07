//! 【区域标注测试】【逐像素对照】正文层叠、实时草稿和取消提交均通过实际编辑器帧入口检查

use crate::{
    app::region_rendering_test_support::{annotation, editor, equal, render},
    types::AnnotationShape,
};
use tiny_skia::Rect;

/// 【区域标注测试】【正文构造】无参数，返回跨固定块边界的半透明重叠图形和曲线
fn bodies() -> Vec<crate::types::Annotation> {
    vec![
        annotation(
            1,
            0.0,
            AnnotationShape::Rectangle {
                start: (120.375, 97.625),
                end: (521.125, 301.875),
                filled: true,
            },
        ),
        annotation(
            2,
            13.0,
            AnnotationShape::Circle {
                start: (160.375, 177.625),
                end: (551.125, 398.875),
                filled: true,
            },
        ),
        annotation(
            3,
            0.0,
            AnnotationShape::Pen {
                points: vec![
                    (90.375, 75.625),
                    (261.125, 131.875),
                    (467.75, 391.25),
                    (569.375, 401.25),
                ],
            },
        ),
    ]
}

/// 【区域标注测试】【正文损坏】无参数，比较整帧缓存、区域正文和反复窄条带恢复的像素，无返回值
#[test]
fn translucent_bodies_and_rotations_match_cached_pixels() {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for state in [&mut cached, &mut region] {
            state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
            state.annotations = bodies();
        }
        equal(
            &render(&mut region, None, false),
            &render(&mut cached, None, false),
            &format!("bodies scale={scale}"),
        );
        let whole = region.canvas[0].clone();
        for x in [111.375, 255.875, 339.75, 509.625] {
            for _ in 0..3 {
                render(
                    &mut region,
                    Rect::from_xywh(x, 50.375, 17.625, 398.125),
                    false,
                );
                equal(
                    &region.canvas[0],
                    &whole,
                    &format!("partial bodies scale={scale} x={x}"),
                );
            }
        }
    }
}

/// 【区域标注测试】【草稿层次】无参数，比较普通草稿和荧光笔读底图的合成顺序，无返回值
#[test]
fn pending_shapes_and_highlighter_keep_the_existing_blend_order() {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        for highlighter in [false, true] {
            let mut cached = editor(scale, false);
            let mut region = editor(scale, true);
            for state in [&mut cached, &mut region] {
                state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
                state.annotations = bodies();
                state.pending = Some(annotation(
                    4,
                    0.0,
                    if highlighter {
                        AnnotationShape::Highlighter {
                            points: vec![(130.375, 180.625), (490.75, 310.875)],
                            flat_ends: [true, false],
                        }
                    } else {
                        AnnotationShape::Rectangle {
                            start: (130.375, 180.625),
                            end: (490.75, 310.875),
                            filled: true,
                        }
                    },
                ));
                state.annotations.truncate(2);
            }
            equal(
                &render(&mut region, None, false),
                &render(&mut cached, None, false),
                &format!("pending scale={scale} highlighter={highlighter}"),
            );
            let whole = region.canvas[0].clone();
            render(
                &mut region,
                Rect::from_xywh(335.375, 55.625, 130.875, 399.375),
                false,
            );
            render(
                &mut cached,
                Rect::from_xywh(335.375, 55.625, 130.875, 399.375),
                false,
            );
            equal(
                &region.canvas[0],
                if highlighter {
                    &cached.canvas[0]
                } else {
                    &whole
                },
                &format!("pending partial scale={scale} highlighter={highlighter}"),
            );
        }
    }
}

/// 【区域标注测试】【固定图形】无参数，验证不创建完整缓存时矩形、圆形和全部标记的整帧及局部像素，无返回值
#[test]
fn fixed_shapes_use_tiles_and_keep_the_cached_pixels() {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for state in [&mut cached, &mut region] {
            state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
            state.annotations = bodies().into_iter().take(2).collect();
        }
        for (index, kind) in crate::types::MarkerShape::ALL.into_iter().enumerate() {
            let x = 70.375 + (index % 6) as f32 * 78.25;
            let y = 70.625 + (index / 6) as f32 * 80.125;
            let mark = annotation(
                index as u64 + 20,
                index as f64 * 13.25,
                AnnotationShape::Marker {
                    start: (x, y),
                    end: (x + 61.375, y + 63.875),
                    kind,
                    filled: index % 2 == 0,
                },
            );
            cached.annotations.push(mark.clone());
            region.annotations.push(mark);
        }
        equal(
            &render(&mut region, None, false),
            &render(&mut cached, None, false),
            &format!("fixed shapes scale={scale}"),
        );
        assert!(!region.region_rendering.cached_annotations);
        assert_eq!(region.annotations_layer[0].data().len(), 4);
        let whole = region.canvas[0].clone();
        for x in [101.375, 255.875, 339.75, 509.625] {
            for _ in 0..3 {
                render(
                    &mut region,
                    Rect::from_xywh(x, 50.375, 17.625, 398.125),
                    false,
                );
                equal(
                    &region.canvas[0],
                    &whole,
                    &format!("fixed shapes partial scale={scale} x={x}"),
                );
            }
        }
        // 1. 【区域标注测试】【隐藏恢复】隐藏正文后恢复，不能留下旧块或改变已缓存的透明像素
        equal(
            &render(&mut region, None, true),
            &render(&mut cached, None, true),
            &format!("hidden fixed shapes scale={scale}"),
        );
        equal(
            &render(&mut region, None, false),
            &whole,
            &format!("restored fixed shapes scale={scale}"),
        );
    }
}
