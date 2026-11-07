//! 【标注多选对照测试】【输入适配】只把Qt输入字段转换为当前标注类型，不用Rust变换函数生成预期

use crate::{
    features::{
        filter::{FilterMode, RegionFilter},
        line::LineSkeleton,
        magnifier::{Magnifier, MagnifierShape},
        number::{NumberMark, NumberStyle},
        ruler::{Measurement, RulerGeometry},
        text::{TextAppearance, TextFrame},
    },
    types::{Annotation, AnnotationShape},
};
use serde_json::Value;
use tiny_skia::{Color, Rect};

/// 【标注多选对照测试】【坐标读取】按value返回Qt二维输入坐标
pub(super) fn point(value: &Value) -> (f32, f32) {
    (
        value[0].as_f64().unwrap() as f32,
        value[1].as_f64().unwrap() as f32,
    )
}
/// 【标注多选对照测试】【矩形读取】按value返回左、上、宽、高形式的Qt输入
pub(super) fn rect(value: &Value) -> Rect {
    Rect::from_xywh(
        value[0].as_f64().unwrap() as f32,
        value[1].as_f64().unwrap() as f32,
        value[2].as_f64().unwrap() as f32,
        value[3].as_f64().unwrap() as f32,
    )
    .unwrap()
}
/// 【标注多选对照测试】【成员读取】按value返回同等存储字段的标注，专用工具角度保存在自身模型，不运行生产变换
pub(super) fn decode(value: &Value) -> Annotation {
    let bounds = rect(&value["rect"]);
    let canvas = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    let start = (bounds.left(), bounds.top());
    let end = (bounds.right(), bounds.bottom());
    let points: Vec<_> = value["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(point)
        .collect();
    let width = value["width"].as_f64().unwrap() as f32;
    let degrees = value["rotation"].as_f64().unwrap();
    let radians = degrees.to_radians() as f32;
    let tool = value["tool"].as_u64().unwrap();
    let shape = match tool {
        3 => AnnotationShape::Rectangle {
            start,
            end,
            filled: false,
        },
        4 => AnnotationShape::Circle {
            start,
            end,
            filled: false,
        },
        5 => AnnotationShape::Marker {
            start,
            end,
            kind: Default::default(),
            filled: false,
        },
        6 => AnnotationShape::Redact {
            filter: RegionFilter {
                start,
                end,
                strength: width,
                mode: FilterMode::Pixelate,
                rotation: radians,
                style_scale: 1.0,
            },
        },
        7 => {
            let source = points[0];
            AnnotationShape::Magnifier {
                magnifier: Magnifier {
                    source,
                    center: crate::features::annotation_selection::chrome::center(bounds),
                    size: (bounds.width(), bounds.height()),
                    shape: if value["magnifier_shape"] == 1 {
                        MagnifierShape::Circle
                    } else {
                        MagnifierShape::Rectangle
                    },
                    zoom: 2.75,
                    rotation: radians,
                    pivot: point(&value["pivot"]),
                    image_bounds: canvas,
                    pixel_scale: 1.0,
                    style_scale: 1.0,
                },
            }
        }
        8 => AnnotationShape::Ruler {
            measurement: Measurement {
                geometry: if value["ruler_mode"] == 1 {
                    RulerGeometry::Angle {
                        points: points.try_into().unwrap(),
                        count: 3,
                    }
                } else {
                    RulerGeometry::Length {
                        start,
                        end,
                        rotation: degrees as f32,
                    }
                },
                pixel_scale: 1.0,
                style_scale: 1.0,
            },
        },
        9 => AnnotationShape::Text {
            start,
            content: value["text"].as_str().unwrap_or("Rotation").into(),
            font_size: width + 19.0,
            bold: true,
            italic: false,
            outline: None,
            appearance: TextAppearance {
                family: "Noto Sans".into(),
                ..Default::default()
            },
            frame: TextFrame {
                width: Some(bounds.width()),
                height: Some(bounds.height()),
                anchor: Some(points[0]),
                painted: None,
            },
        },
        10 => AnnotationShape::Pen { points },
        11 => AnnotationShape::Highlighter {
            points,
            flat_ends: [false; 2],
        },
        12 | 13 => {
            let mut line = LineSkeleton::new(points[0], points[1]).unwrap();
            line.controls = points[2..].to_vec();
            if tool == 12 {
                AnnotationShape::Line {
                    line,
                    style: Default::default(),
                }
            } else {
                AnnotationShape::Arrow {
                    line,
                    style: Default::default(),
                    line_style: Default::default(),
                }
            }
        }
        14 => AnnotationShape::Number {
            mark: NumberMark {
                tip: points[0],
                bubble: points[1],
                rotation: radians,
                number: 1,
                style: NumberStyle::Arabic,
                style_scale: 1.0,
            },
        },
        _ => unreachable!(),
    };
    let rotation_degrees = if matches!(tool, 6 | 7 | 8 | 14) {
        0.0
    } else {
        degrees
    };
    let mut annotation = Annotation {
        style_scale: 1.0,
        id: value["id"].as_u64().unwrap(),
        shape,
        rotation_degrees,
        frozen_bounds: Some(canvas),
        color: value["color"]
            .as_array()
            .map_or(Color::from_rgba8(30, 40, 50, 255), |color| {
                Color::from_rgba8(
                    color[0].as_u64().unwrap() as u8,
                    color[1].as_u64().unwrap() as u8,
                    color[2].as_u64().unwrap() as u8,
                    color[3].as_u64().unwrap() as u8,
                )
            }),
        shadow_color: Color::TRANSPARENT,
        stroke_width: width,
        bbox: bounds,
    };
    if let Some(pivot) =
        crate::features::annotation_collection::geometry::rotation_center(&annotation, canvas)
        && let AnnotationShape::Magnifier { magnifier } = &mut annotation.shape
    {
        magnifier.pivot = pivot;
    }
    annotation
}
