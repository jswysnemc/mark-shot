//! 【标注命中测试】【生产矩阵】完整Qt入口的实际输出与编辑器首次选择共用同一组原图样本

use super::*;
use crate::{features::ruler::RulerGeometry, types::AnnotationShape};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    scenes: Vec<Scene>,
    samples: Vec<Sample>,
}

#[derive(Debug, Deserialize)]
struct Scene {
    index: usize,
    scale: f64,
    rect: [f32; 4],
    points: Vec<(f32, f32)>,
    width: f32,
    rotation: f32,
    pivot: (f32, f32),
}

#[derive(Debug, Deserialize)]
struct Sample {
    scene: usize,
    point: (f64, f64),
    first: bool,
    selected: String,
    ruler_first: String,
}

/// 【标注命中测试】【参考模型】按scene构造原图坐标标注，不把窗口倍率写入笔宽或定点，返回持久对象
fn annotation(scene: &Scene) -> crate::types::Annotation {
    let [x, y, width, height] = scene.rect;
    let start = (x, y);
    let end = (x + width, y + height);
    let shape = match scene.index {
        0 => AnnotationShape::Rectangle {
            start,
            end,
            filled: false,
        },
        1 => AnnotationShape::Circle {
            start,
            end,
            filled: false,
        },
        2 => AnnotationShape::Marker {
            start,
            end,
            kind: Default::default(),
            filled: false,
        },
        3 => {
            let mut filter = crate::features::filter::RegionFilter::new(
                start,
                end,
                scene.width,
                Default::default(),
            )
            .unwrap();
            filter.rotation = scene.rotation.to_radians();
            AnnotationShape::Redact { filter }
        }
        4 => AnnotationShape::Pen {
            points: scene.points.clone(),
        },
        5 => AnnotationShape::Highlighter {
            points: scene.points.clone(),
            flat_ends: [false; 2],
        },
        6 | 7 => {
            let line =
                crate::features::line::LineSkeleton::new(scene.points[0], scene.points[1]).unwrap();
            if scene.index == 6 {
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
        8 => {
            let mut mark =
                crate::features::number::NumberMark::new(scene.points[1], 1, Default::default())
                    .unwrap();
            mark.tip = scene.points[0];
            mark.rotation = scene.rotation.to_radians();
            AnnotationShape::Number { mark }
        }
        9 | 10 => {
            let mut magnifier = crate::features::magnifier::Magnifier::new(
                if scene.index == 9 {
                    crate::features::magnifier::MagnifierShape::Rectangle
                } else {
                    crate::features::magnifier::MagnifierShape::Circle
                },
                scene.points[0],
                2.75,
                Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
                1.0,
            )
            .unwrap();
            magnifier.center = (x + width / 2.0, y + height / 2.0);
            magnifier.size = (width, height);
            magnifier.pivot = scene.pivot;
            magnifier.rotation = scene.rotation.to_radians();
            AnnotationShape::Magnifier { magnifier }
        }
        11 | 12 => AnnotationShape::Ruler {
            measurement: crate::features::ruler::Measurement {
                geometry: if scene.index == 11 {
                    RulerGeometry::Length {
                        start,
                        end,
                        rotation: scene.rotation,
                    }
                } else {
                    RulerGeometry::Angle {
                        points: scene.points.clone().try_into().unwrap(),
                        count: 3,
                    }
                },
                pixel_scale: 1.0,
                style_scale: 1.0,
            },
        },
        _ => panic!("unknown reference scene"),
    };
    let mut annotation = crate::types::Annotation {
        id: 7,
        style_scale: 1.0,
        rotation_degrees: f64::from(scene.rotation),
        frozen_bounds: Rect::from_xywh(0.0, 0.0, 640.0, 480.0),
        shape,
        color: Color::BLACK,
        shadow_color: Color::TRANSPARENT,
        stroke_width: scene.width,
        bbox: Rect::from_xywh(x, y, width, height).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【标注命中测试】【参考读取】按环境中的证据文件读取实际Qt输出，缺少采集文件时明确失败，返回完整参考矩阵
fn reference() -> Reference {
    let path = std::env::var("MARK_SHOT_HIT_REFERENCE")
        .expect("set MARK_SHOT_HIT_REFERENCE to the collected Qt reference.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// 【标注命中测试】【首次参考】逐项比较完整Qt首次命中，实际入口按当前视图换算窗口容差，无返回值
#[test]
#[ignore = "requires complete production Qt reference capture"]
fn initial_hits_match_complete_qt_production_matrix() {
    let reference = reference();
    let annotations: Vec<_> = reference.scenes.iter().map(annotation).collect();
    for sample in &reference.samples {
        let scene = &reference.scenes[sample.scene];
        let actual = crate::features::annotation_selection::body_hit::initial(
            &annotations[sample.scene],
            (sample.point.0 as f32, sample.point.1 as f32),
            crate::features::annotation_style::window::WindowUnits::new(1.0 / scene.scale as f32),
        );
        assert_eq!(
            actual, sample.first,
            "sample={sample:?} scene={:?}",
            reference.scenes[sample.scene]
        );
    }
}

/// 【标注命中测试】【尺子正文参考】逐项比较尺子工具首次操作，内部保留新建职责，固定窗口容差不读取样式比例
#[test]
#[ignore = "requires complete production Qt reference capture"]
fn ruler_tool_body_hits_match_complete_qt_production_matrix() {
    let reference = reference();
    let annotations: Vec<_> = reference.scenes.iter().map(annotation).collect();
    for sample in &reference.samples {
        let crate::types::AnnotationShape::Ruler { measurement } = annotations[sample.scene].shape
        else {
            continue;
        };
        let scene = &reference.scenes[sample.scene];
        let actual = crate::features::ruler::drag::body_hit(
            measurement,
            (sample.point.0 as f32, sample.point.1 as f32),
            8.0 / scene.scale as f32,
            false,
        );
        assert_eq!(
            format!("{actual:?}"),
            sample.ruler_first,
            "sample={sample:?} scene={scene:?}"
        );
    }
}

/// 【标注命中测试】【已选正文参考】逐项比较非尺子已选正文与控制点；旋转把手由窗口入口单独验证，不计入本项
#[test]
#[ignore = "requires complete production Qt reference capture"]
fn selected_body_hits_match_complete_qt_production_matrix() {
    let reference = reference();
    let annotations: Vec<_> = reference.scenes.iter().map(annotation).collect();
    for sample in &reference.samples {
        let scene = &reference.scenes[sample.scene];
        if scene.index >= 11 || sample.selected == "Rotate" {
            continue;
        }
        let actual = crate::features::annotation_selection::body_hit::selected(
            &annotations[sample.scene],
            (sample.point.0 as f32, sample.point.1 as f32),
            crate::features::annotation_style::window::WindowUnits::new(1.0 / scene.scale as f32),
        );
        let actual = match actual {
            crate::types::SelectionHandle::LinePoint(0) => "LineStart".to_owned(),
            crate::types::SelectionHandle::LinePoint(1) => "LineEnd".to_owned(),
            crate::types::SelectionHandle::LinePoint(_) => "LineControl".to_owned(),
            _ => format!("{actual:?}"),
        };
        assert_eq!(actual, sample.selected, "sample={sample:?} scene={scene:?}");
    }
}
