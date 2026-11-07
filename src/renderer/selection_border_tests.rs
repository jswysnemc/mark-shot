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
