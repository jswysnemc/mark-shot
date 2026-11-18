//! 【荧光笔测试】【背景混合】通过真实标注与即时预览入口验证深色内容保持可读

use super::annotations::draw_highlighter_live;
use super::*;
use std::collections::HashMap;
use tiny_skia::{Color, Pixmap, Rect};

/// 【荧光笔测试】【标注构造】按 angle 返回中心固定的荧光笔，角度用于覆盖旋转分块路径
fn annotation(angle: f32) -> crate::types::Annotation {
    let mut annotation = crate::types::Annotation {
        id: 1,
        style_scale: 1.0,
        rotation_degrees: f64::from(angle),
        frozen_bounds: None,
        shape: crate::types::AnnotationShape::Highlighter {
            points: vec![(50.0, 100.0), (150.0, 100.0)],
            flat_ends: [false; 2],
        },
        color: Color::from_rgba8(230, 70, 30, 255),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 16.0,
        bbox: Rect::from_xywh(50.0, 100.0, 100.0, 1.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【荧光笔测试】【实际绘制】按 annotation、background、origin 与 size 返回生产入口的像素，局部原点沿用完整场景坐标
fn render(
    annotation: &crate::types::Annotation,
    background: Color,
    origin: (f32, f32),
    size: u32,
) -> Pixmap {
    let mut canvas = Pixmap::new(size, size).unwrap();
    canvas.fill(background);
    draw_annotation(
        &mut Painter::new(&mut canvas).with_origin(origin),
        annotation,
        false,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
    );
    canvas
}

/// 【荧光笔测试】【旋转背景】无参数，零度、斜角与直角均保留黑字，彩色背景的中心混合也相同，无返回值
#[test]
fn rotated_highlighter_multiplies_the_actual_background() {
    for background in [
        Color::BLACK,
        Color::from_rgba8(35, 90, 170, 255),
        Color::WHITE,
    ] {
        let expected = render(&annotation(0.0), background, (0.0, 0.0), 200)
            .pixel(100, 100)
            .unwrap();
        for angle in [45.0, 90.0] {
            let full = render(&annotation(angle), background, (0.0, 0.0), 200);
            let actual = full.pixel(100, 100).unwrap();
            let differences = [
                actual.red().abs_diff(expected.red()),
                actual.green().abs_diff(expected.green()),
                actual.blue().abs_diff(expected.blue()),
                actual.alpha().abs_diff(expected.alpha()),
            ];
            assert!(
                differences.into_iter().all(|value| value <= 1),
                "angle={angle}, background={background:?}, actual={actual:?}, expected={expected:?}"
            );
            let local = render(&annotation(angle), background, (80.0, 80.0), 40);
            for y in 0..40 {
                for x in 0..40 {
                    assert_eq!(local.pixel(x, y), full.pixel(x + 80, y + 80));
                }
            }
        }
    }
}

/// 【荧光笔测试】【即时背景】无参数，实际即时预览与已提交笔迹对黑字产生相同乘法结果，无返回值
#[test]
fn live_highlighter_preserves_black_text_like_committed_strokes() {
    let mut canvas = Pixmap::new(200, 200).unwrap();
    canvas.fill(Color::BLACK);
    let annotation = annotation(0.0);
    let crate::types::AnnotationShape::Highlighter { points, flat_ends } = &annotation.shape else {
        unreachable!()
    };
    draw_highlighter_live(
        &mut Painter::new(&mut canvas),
        points,
        *flat_ends,
        annotation.color,
        annotation.stroke_width,
        None,
    );
    assert_eq!(
        canvas.pixel(100, 100).unwrap(),
        render(&annotation, Color::BLACK, (0.0, 0.0), 200)
            .pixel(100, 100)
            .unwrap()
    );
}
