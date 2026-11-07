//! 【标注样式测试】【像素度量】比较真实Qt图像测得的边界和颜色，不使用复制的尺寸公式

use mark_shot::{
    features::{line::LineSkeleton, number::NumberMark},
    renderer::{Painter, draw_annotation},
    types::{Annotation, AnnotationShape},
};
use serde::Deserialize;
use std::collections::HashMap;
use tiny_skia::{Color, Pixmap, Rect};

#[derive(Deserialize)]
struct Reference {
    samples: Vec<Sample>,
}

#[derive(Debug, Deserialize)]
struct Sample {
    tool: String,
    scale: f32,
    width: f32,
    device: f32,
    bounds: [u32; 4],
    colors: [[u8; 4]; 3],
}

/// 【标注样式测试】【投影模型】按sample返回完成原图投影后的标注，全部定点已经位于窗口坐标，不加载图形后端
fn annotation(sample: &Sample) -> Annotation {
    let (start, end) = ((240.0, 240.0), (400.0, 240.0));
    let mut line = LineSkeleton::new(start, end).unwrap();
    line.style_scale = sample.scale;
    let shape = match sample.tool.as_str() {
        "pen" => AnnotationShape::Pen {
            points: vec![start, end],
        },
        "highlighter" => AnnotationShape::Highlighter {
            points: vec![start, end],
            flat_ends: [false; 2],
        },
        "line" => AnnotationShape::Line {
            line,
            style: Default::default(),
        },
        "arrow" => AnnotationShape::Arrow {
            line,
            style: Default::default(),
            line_style: Default::default(),
        },
        "ellipse" => AnnotationShape::Circle {
            start: (260.0, 200.0),
            end: (380.0, 280.0),
            filled: false,
        },
        "number" => {
            let mut mark = NumberMark::new((320.0, 240.0), 1, Default::default()).unwrap();
            mark.style_scale = sample.scale;
            AnnotationShape::Number { mark }
        }
        _ => panic!("unknown reference tool"),
    };
    let mut annotation = Annotation {
        id: 1,
        style_scale: sample.scale,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        shape,
        color: Color::from_rgba8(239, 68, 68, 180),
        shadow_color: Color::TRANSPARENT,
        stroke_width: sample.width * sample.scale,
        bbox: Rect::from_xywh(240.0, 200.0, 160.0, 80.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【标注样式测试】【实际边界】按pixels返回所有非白色像素的最小外框，无实际正文时返回空值
fn bounds(pixels: &Pixmap) -> Option<[u32; 4]> {
    let mut bounds = [pixels.width(), pixels.height(), 0, 0];
    for (index, pixel) in pixels.data().as_chunks::<4>().0.iter().enumerate() {
        if *pixel == [255; 4] {
            continue;
        }
        let x = index as u32 % pixels.width();
        let y = index as u32 / pixels.width();
        bounds[0] = bounds[0].min(x);
        bounds[1] = bounds[1].min(y);
        bounds[2] = bounds[2].max(x + 1);
        bounds[3] = bounds[3].max(y + 1);
    }
    (bounds[2] > bounds[0] && bounds[3] > bounds[1]).then_some(bounds)
}

/// 【标注样式测试】【窗口像素】按真实Qt图像检查窗口外框和实色描边颜色；箭头只比较头部外移范围，渐细尾部及椭圆边缘的光栅颜色另行核对
#[test]
fn rendered_style_bounds_and_colors_match_complete_production_qt_samples() {
    let reference: Reference =
        serde_json::from_str(include_str!("../fixtures/qt-annotation-style/metrics.json")).unwrap();
    assert_eq!(reference.samples.len(), 144);
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    let mut editors = HashMap::new();
    let filters = HashMap::new();
    for sample in reference.samples {
        let mut pixels = Pixmap::new(
            (640.0 * sample.device) as u32,
            (480.0 * sample.device) as u32,
        )
        .unwrap();
        pixels.fill(Color::WHITE);
        if sample.tool == "highlighter" {
            Painter::new(&mut pixels)
                .with_scale(sample.device)
                .fill_rounded(
                    Rect::from_xywh(300.0, 180.0, 40.0, 120.0).unwrap(),
                    0.0,
                    Color::BLACK,
                );
        }
        draw_annotation(
            &mut Painter::new(&mut pixels)
                .with_scale(sample.device)
                .with_text(&mut fonts, &mut cache),
            &annotation(&sample),
            false,
            &mut editors,
            &filters,
            None,
        );
        let actual = bounds(&pixels).unwrap();
        let tolerance = sample.device.ceil() as u32;
        for (index, (actual, expected)) in actual.into_iter().zip(sample.bounds).enumerate() {
            if sample.tool == "arrow" && index == 0 {
                continue;
            }
            assert!(
                actual.abs_diff(expected) <= tolerance,
                "{sample:?} actual bounds={actual} expected={expected}"
            );
        }
        if matches!(sample.tool.as_str(), "number" | "ellipse" | "arrow") {
            continue;
        }
        for (index, x) in [260.0, 320.0, 370.0].into_iter().enumerate() {
            let pixel = pixels
                .pixel((x * sample.device) as u32, (240.0 * sample.device) as u32)
                .unwrap();
            for (actual, expected) in [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
                .into_iter()
                .zip(sample.colors[index])
            {
                assert!(
                    actual.abs_diff(expected) <= 2,
                    "{sample:?} x={x} actual channel={actual} expected={expected}"
                );
            }
        }
    }
}
