//! 【选框测试】【局部一致】选区边框在实际分数视口中局部重绘必须与整帧绘制一致

use super::draw_selection_border;
use crate::{renderer::Painter, types::SelectionEdges};
use tiny_skia::{Color, Pixmap, Rect};

/// 【选框测试】【真实分数视口】使用九百乘五百六十原图的半像素视口，恢复顶部损坏后重绘必须逐像素一致，无参数、无返回值
#[test]
fn fractional_selection_border_damage_matches_a_whole_frame() {
    let edges = SelectionEdges {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };
    let selection = Rect::from_xywh(83.5, 53.5, 900.0, 560.0).unwrap();
    let dirty = Rect::from_xywh(75.1, 52.1, 920.0, 48.0).unwrap();
    for scale in [1.0, 1.2, 1.25, 2.0] {
        let mut expected = Pixmap::new((1067.0 * scale) as u32, (667.0 * scale) as u32).unwrap();
        let background = Color::from_rgba8(205, 205, 205, 255);
        expected.fill(background);
        draw_selection_border(
            &mut Painter::new(&mut expected).with_scale(scale),
            &selection,
            Some(&edges),
            8.0,
        );
        let mut actual = expected.clone();
        for _ in 0..3 {
            Painter::new(&mut actual)
                .with_scale(scale)
                .clipped(Some(dirty), |painter| {
                    // 1. 【选框测试】【损坏恢复】与真实帧一样先恢复损坏背景，然后绘制同一个选框
                    painter.canvas().fill(background);
                    draw_selection_border(painter, &selection, Some(&edges), 8.0);
                });
            let changed = actual
                .data()
                .iter()
                .zip(expected.data())
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(changed, 0, "selection border differs at scale {scale}");
        }
    }
}

/// 【选框测试】【初始直角】截图透明区域四个角保留原图，图片预览允许独立圆角配置，无参数、无返回值
#[test]
fn capture_selection_keeps_square_corners_while_image_preview_can_be_rounded() {
    let edges = SelectionEdges {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };
    let selection = Rect::from_xywh(40.0, 30.0, 120.0, 80.0).unwrap();
    let mut square = Pixmap::new(220, 150).unwrap();
    super::init_dimming(&mut square, Some(&selection), Some(&edges), 1.0);
    for (x, y) in [(40, 30), (159, 30), (40, 109), (159, 109)] {
        assert_eq!(square.pixel(x, y).unwrap().alpha(), 0);
    }
    let mut rounded = square.clone();
    super::update_dimming_delta(
        &mut rounded,
        Some(&selection),
        Some(&selection),
        Some(&edges),
        1.0,
        8.0,
    );
    assert!(rounded.pixel(40, 30).unwrap().alpha() > 0);
    super::update_dimming_delta(
        &mut rounded,
        Some(&selection),
        Some(&selection),
        Some(&edges),
        1.0,
        0.0,
    );
    assert_eq!(rounded.data(), square.data());
}

/// 【选框测试】【旧栅格对照】无参数，对比完整路径旧分块与空心边框，覆盖分数比例、负坐标和圆角，全部像素必须相同
#[test]
fn hollow_border_tiles_preserve_the_previous_full_path_raster() {
    let edges = SelectionEdges {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };
    let config = &crate::config::get().selection;
    let stroke = tiny_skia::Stroke {
        width: config.border_width,
        ..Default::default()
    };
    for scale in [1.0, 1.2, 1.25, 1.5, 2.0] {
        for (left, top, radius) in [(83.5, 53.5, 0.0), (83.5, 53.5, 8.0), (-61.25, -13.5, 8.0)] {
            let selection = Rect::from_xywh(left, top, 900.0, 560.0).unwrap();
            let mut expected =
                Pixmap::new((1067.0 * scale) as u32, (667.0 * scale) as u32).unwrap();
            expected.fill(Color::from_rgba8(81, 113, 149, 213));
            let mut actual = expected.clone();
            let outer = selection
                .outset(stroke.width / 2.0, stroke.width / 2.0)
                .unwrap();
            let path =
                crate::renderer::paths::rounded_rect_path(&outer, radius, true, true, true, true)
                    .unwrap();
            let outline = path.stroke(&stroke, scale).unwrap();
            crate::renderer::fixed_path::fill(
                &mut Painter::new(&mut expected).with_scale(scale),
                &outline,
                config.border.get().color(),
            );
            draw_selection_border(
                &mut Painter::new(&mut actual).with_scale(scale),
                &selection,
                Some(&edges),
                radius,
            );
            assert!(
                actual.data() == expected.data(),
                "scale={scale}, position={left},{top}, radius={radius}"
            );
        }
    }
}

/// 【选框测试】【边缘裁切】无参数，比较四边单独裁切与原有完整损坏缓冲，覆盖窄选区、负坐标和分数比例，无返回值
#[test]
fn edge_only_damage_preserves_full_damage_pixels() {
    let edges = SelectionEdges {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };
    for scale in [1.0, 1.2, 1.25, 1.5, 2.0] {
        for (left, top, width, height) in [
            (23.5, 13.5, 320.0, 200.0),
            (-61.25, -13.5, 320.0, 200.0),
            (70.0, 45.0, 4.0, 8.0),
        ] {
            let selection = Rect::from_xywh(left, top, width, height).unwrap();
            for dirty in [
                None,
                Rect::from_xywh(32.1, 22.2, 120.7, 110.0),
                Rect::from_xywh(10.2, 10.2, 335.0, 100.0),
            ] {
                let mut expected =
                    Pixmap::new((400.0 * scale) as u32, (240.0 * scale) as u32).unwrap();
                expected.fill(Color::from_rgba8(81, 113, 149, 213));
                let mut actual = expected.clone();
                Painter::new(&mut expected)
                    .with_scale(scale)
                    .clipped(dirty, |painter| {
                        draw_selection_border(painter, &selection, Some(&edges), 8.0);
                    });
                crate::renderer::selection_border_damage::draw(
                    &mut actual,
                    scale,
                    dirty,
                    &selection,
                    Some(&edges),
                    8.0,
                );
                assert!(
                    actual.data() == expected.data(),
                    "scale={scale}, selection={selection:?}, dirty={dirty:?}"
                );
            }
        }
    }
}
