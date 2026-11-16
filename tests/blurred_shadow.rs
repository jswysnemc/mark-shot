//! 【阴影测试】【实际绘制】验证窗口单位、有限缓冲、局部裁剪和完整损坏恢复

use mark_shot::{
    features::annotation_style::window::WindowUnits,
    renderer::{Painter, blurred_shadow, draw_annotation, rebuild_annotations_layer},
    types::{Annotation, AnnotationShape},
};
use std::{collections::HashMap, sync::Once};
use tiny_skia::{Color, IntRect, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};

/// 【阴影测试】【配置初始化】无参数，以三十二窗口像素模糊测试较大滤波范围，无返回值
fn configure() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        mark_shot::config::init(None, |config| config.annotations.shadow.blur = 32.0)
    });
}

/// 【阴影测试】【轮廓构造】按 curved 返回曲线或矩形路径，无需画布参数
fn outline(curved: bool) -> Path {
    let mut builder = PathBuilder::new();
    if curved {
        builder.move_to(20.75, 130.5);
        builder.cubic_to(150.25, 310.75, 350.5, -30.25, 620.25, 230.5);
    } else {
        builder.push_rect(Rect::from_xywh(20.75, 30.5, 599.5, 230.0).unwrap());
    }
    builder.finish().unwrap()
}

/// 【阴影测试】【误差检查】比较 actual 与 expected 的每个预乘通道，context 标识场景；至多一级误差，无返回值
fn equal(actual: &Pixmap, expected: &Pixmap, context: &str) {
    assert_eq!(actual.width(), expected.width());
    assert_eq!(actual.height(), expected.height());
    let worst = actual
        .data()
        .iter()
        .zip(expected.data())
        .map(|(actual, expected)| actual.abs_diff(*expected))
        .max()
        .unwrap_or(0);
    assert!(worst <= 1, "{context}: worst channel error {worst}");
}

/// 【阴影测试】【窗口等价】无参数，设备比例与等价画布坐标同时生成相同偏移、模糊及透明度，无返回值
#[test]
fn coordinate_and_device_scaling_keep_the_same_shadow() {
    configure();
    let color = Color::from_rgba8(15, 30, 45, 150);
    for curved in [false, true] {
        let path = outline(curved);
        let stroke = curved.then(|| Stroke {
            width: 7.0,
            ..Default::default()
        });
        let mut viewport = Pixmap::new(1360, 720).unwrap();
        blurred_shadow::draw(
            &mut Painter::new(&mut viewport).with_scale(2.0),
            &path,
            stroke.as_ref(),
            color,
        );
        let scaled = path.transform(Transform::from_scale(2.0, 2.0)).unwrap();
        let stroke = stroke.map(|stroke| Stroke {
            width: stroke.width * 2.0,
            ..stroke
        });
        let mut original = Pixmap::new(1360, 720).unwrap();
        blurred_shadow::draw(
            &mut Painter::new(&mut original).with_window_units(WindowUnits::new(2.0)),
            &scaled,
            stroke.as_ref(),
            color,
        );
        equal(&original, &viewport, &format!("window curved={curved}"));
    }
}

/// 【阴影测试】【裁剪一致】无参数，窄条带、块边界和窗口边缘与完整画布裁切一致，无返回值
#[test]
fn clipped_filled_and_stroked_shadows_match_full_pixels() {
    configure();
    let color = Color::from_rgba8(15, 30, 45, 150);
    for scale in [1.0, 1.25, 1.75, 2.0] {
        for curved in [false, true] {
            let path = outline(curved);
            let stroke = curved.then(|| Stroke {
                width: 7.0,
                ..Default::default()
            });
            let mut full = Pixmap::new(1360, 720).unwrap();
            blurred_shadow::draw(
                &mut Painter::new(&mut full).with_scale(scale),
                &path,
                stroke.as_ref(),
                color,
            );
            for region in [
                IntRect::from_xywh(173, 131, 3, 157).unwrap(),
                IntRect::from_xywh(471, 17, 533, 611).unwrap(),
                IntRect::from_xywh(0, 0, 90, 70).unwrap(),
            ] {
                let mut clipped = Pixmap::new(region.width(), region.height()).unwrap();
                blurred_shadow::draw(
                    &mut Painter::new(&mut clipped)
                        .with_scale(scale)
                        .with_origin((region.x() as f32 / scale, region.y() as f32 / scale)),
                    &path,
                    stroke.as_ref(),
                    color,
                );
                equal(
                    &clipped,
                    &full.clone_rect(region).unwrap(),
                    &format!("clip scale={scale}, curved={curved}, region={region:?}"),
                );
            }
        }
    }
}

/// 【阴影测试】【矩形构造】无参数，返回半透明空心矩形及启用的柔和阴影
fn rectangle() -> Annotation {
    let color = Color::from_rgba8(220, 30, 50, 190);
    let mut annotation = Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 7,
        shape: AnnotationShape::Rectangle {
            radius: 8.0,
            fixed_ratio: false,
            start: (100.75, 100.5),
            end: (430.25, 260.5),
            filled: false,
        },
        color,
        shadow_color: Color::from_rgba8(15, 30, 45, 150),
        stroke_width: 7.0,
        bbox: Rect::from_xywh(100.75, 100.5, 329.5, 160.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【阴影测试】【清除范围】无参数，按正式 damage_bbox 删除真实标注后所有阴影像素均清除，无返回值
#[test]
fn annotation_damage_clears_the_entire_blurred_tail() {
    configure();
    let annotation = rectangle();
    for scale in [1.0, 1.25, 2.0] {
        let mut canvas = Pixmap::new(1080, 720).unwrap();
        let mut editors = HashMap::new();
        let filters = HashMap::new();
        draw_annotation(
            &mut Painter::new(&mut canvas).with_scale(scale),
            &annotation,
            false,
            &mut editors,
            &filters,
            None,
        );
        assert!(canvas.pixels().iter().any(|pixel| pixel.alpha() > 0));
        rebuild_annotations_layer(
            &mut Painter::new(&mut canvas).with_scale(scale),
            &[],
            &mut editors,
            &filters,
            None,
            Some(annotation.damage_bbox(false)),
        );
        assert!(
            canvas.data().iter().all(|byte| *byte == 0),
            "scale={scale}: residual shadow after removal"
        );
    }
}

/// 【阴影测试】【小视区】无参数，超大画布外路径只为当前可见区生成阴影，无返回值
#[test]
fn huge_paths_render_only_the_small_visible_shadow() {
    configure();
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_xywh(-100_000.0, -100_000.0, 200_000.0, 200_000.0).unwrap());
    let mut canvas = Pixmap::new(31, 23).unwrap();
    blurred_shadow::draw(
        &mut Painter::new(&mut canvas),
        &builder.finish().unwrap(),
        None,
        Color::from_rgba8(15, 30, 45, 150),
    );
    assert!(canvas.pixels().iter().all(|pixel| pixel.alpha() == 150));
}

/// 【阴影测试】【四千像素测量】无参数，输出十次四千像素空心框阴影绘制耗时并核对内部透明，无返回值
#[test]
#[ignore = "手动测量绘制耗时，不设依赖机器性能的通过阈值"]
fn measure_4k_hollow_shadow() {
    let mut canvas = Pixmap::new(3840, 2160).unwrap();
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_xywh(30.0, 30.0, 3780.0, 2100.0).unwrap());
    let path = builder.finish().unwrap();
    let stroke = Stroke {
        width: 7.0,
        ..Default::default()
    };
    let started = std::time::Instant::now();
    for _ in 0..10 {
        canvas.fill(Color::TRANSPARENT);
        blurred_shadow::draw(
            &mut Painter::new(&mut canvas),
            &path,
            Some(&stroke),
            Color::from_rgba8(15, 30, 45, 150),
        );
    }
    eprintln!(
        "4k hollow shadow mean: {:.2} ms",
        started.elapsed().as_secs_f64() * 100.0
    );
    assert_eq!(canvas.pixel(1920, 1080).unwrap().alpha(), 0);
    assert!(canvas.pixel(34, 1080).unwrap().alpha() > 0);
}
