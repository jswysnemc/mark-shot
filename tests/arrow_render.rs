//! 【箭头测试】【组合绘制】验证四种头部、三种线型、曲线、透明度、原图比例和固定像素分块

use mark_shot::features::{
    arrow::{ArrowStyle, draw, metrics},
    line::{LineSkeleton, LineStyle, geometry},
};
use mark_shot::renderer::Painter;
use tiny_skia::{Color, IntRect, Pixmap};

/// 【箭头测试】【参考骨架】返回原版箭头绘制测试中的直线或单控制点曲线
fn skeleton(curved: bool) -> LineSkeleton {
    let mut line = LineSkeleton::new((30.0, 70.0), (420.0, 70.0)).unwrap();
    if curved {
        line.controls.push((225.0, 180.0));
    }
    line
}

/// 【箭头测试】【图像绘制】将 line 以 style、pattern、width、color 绘入透明图；返回 RGBA 预乘图像
fn render(
    line: &LineSkeleton,
    style: ArrowStyle,
    pattern: LineStyle,
    width: f32,
    color: Color,
) -> Pixmap {
    let mut image = Pixmap::new(460, 240).unwrap();
    draw::draw(
        &mut Painter::new(&mut image),
        line,
        style,
        pattern,
        width,
        color,
    );
    image
}

/// 【箭头测试】【中心覆盖】统计 image 在 line 中段的一百个采样点覆盖数量，不依赖燕尾主体宽度
fn center_ink(image: &Pixmap, line: &LineSkeleton) -> usize {
    let sampled = geometry::sampled(line).unwrap();
    (0..100)
        .filter(|index| {
            let point = sampled.at(sampled.length * (0.25 + *index as f32 * 0.005));
            image
                .pixel(point.0.round() as u32, point.1.round() as u32)
                .unwrap()
                .alpha()
                > 64
        })
        .count()
}

/// 【箭头测试】【区域覆盖】返回 image 在 area 中非透明像素数量
fn ink(image: &Pixmap, area: IntRect) -> usize {
    (area.y()..area.bottom())
        .flat_map(|y| (area.x()..area.right()).map(move |x| (x, y)))
        .filter(|(x, y)| image.pixel(*x as u32, *y as u32).unwrap().alpha() > 64)
        .count()
}

/// 【箭头测试】【全部组合】四种头部与直线、曲线组合时，虚线和点线只减少箭身覆盖，不删除头部
#[test]
fn every_head_and_curve_supports_all_shaft_patterns() {
    for style in ArrowStyle::ALL {
        for curved in [false, true] {
            let line = skeleton(curved);
            let images =
                LineStyle::ALL.map(|pattern| render(&line, style, pattern, 4.0, Color::BLACK));
            let counts = images.each_ref().map(|image| center_ink(image, &line));
            assert_eq!(counts[0], 100, "{style:?} curved={curved}: {counts:?}");
            assert!(
                counts[1] > 0 && counts[1] < counts[0],
                "{style:?} curved={curved}: {counts:?}"
            );
            assert!(
                counts[2] > 0 && counts[2] < counts[1],
                "{style:?} curved={curved}: {counts:?}"
            );
            for image in &images {
                assert!(ink(image, IntRect::from_xywh(416, 66, 8, 8).unwrap()) > 0);
                if style.bidirectional() {
                    assert!(ink(image, IntRect::from_xywh(26, 66, 8, 8).unwrap()) > 0);
                }
            }
        }
    }
}

/// 【箭头测试】【透明度上限】一条半透明箭头任何头部或接缝都不得超过指定透明度
#[test]
fn semi_transparent_heads_and_shafts_are_composited_once() {
    for style in ArrowStyle::ALL {
        for pattern in LineStyle::ALL {
            let image = render(
                &skeleton(true),
                style,
                pattern,
                7.0,
                Color::from_rgba8(208, 36, 44, 120),
            );
            let maximum = image
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[3])
                .max()
                .unwrap();
            assert_eq!(maximum, 120, "{style:?} {pattern:?}");
            assert!(
                image
                    .data()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|pixel| pixel[0] <= pixel[3])
            );
        }
    }
}

/// 【箭头测试】【头部接缝】虚线和点线的实心头部连接处保持连续，不因相反路径绕向出现缺口
#[test]
fn filled_head_connections_do_not_cancel_the_shaft() {
    for style in [ArrowStyle::Fletched, ArrowStyle::BidirectionalFletched] {
        for pattern in [LineStyle::Dashed, LineStyle::Dotted] {
            let line = skeleton(false);
            let sampled = geometry::sampled(&line).unwrap();
            let size = metrics::metrics(sampled.length, 4.0, 1.0, style);
            let image = render(&line, style, pattern, 4.0, Color::BLACK);
            let point = sampled.at(sampled.length - size.length + 1.0);
            assert!(
                image
                    .pixel(point.0.round() as u32, point.1.round() as u32)
                    .unwrap()
                    .alpha()
                    > 64,
                "{style:?} {pattern:?}"
            );
        }
    }
}

/// 【箭头测试】【分数缩放】局部重绘从完整正文的固定像素原点取样，裁切不改变路径相位
#[test]
fn fractional_scale_clips_match_the_full_render_exactly() {
    let mut line = LineSkeleton::new((33.4, 55.7), (510.6, 207.3)).unwrap();
    line.controls.push((275.25, 289.1));
    let region = IntRect::from_xywh(173, 131, 211, 157).unwrap();
    for style in ArrowStyle::ALL {
        for pattern in LineStyle::ALL {
            let color = Color::from_rgba8(200, 40, 70, 160);
            let mut full = Pixmap::new(720, 440).unwrap();
            draw::draw(
                &mut Painter::new(&mut full).with_scale(1.25),
                &line,
                style,
                pattern,
                7.35,
                color,
            );
            let mut clipped = Pixmap::new(region.width(), region.height()).unwrap();
            draw::draw(
                &mut Painter::new(&mut clipped)
                    .with_scale(1.25)
                    .with_origin((region.x() as f32 / 1.25, region.y() as f32 / 1.25)),
                &line,
                style,
                pattern,
                7.35,
                color,
            );
            assert!(
                clipped.data() == full.clone_rect(region).unwrap().data(),
                "{style:?} {pattern:?}"
            );
        }
    }
}

/// 【箭头测试】【窗口比例】几何换坐标与设备换比例在相同窗口单位下生成相同像素，头部限制随窗口单位独立换算
#[test]
fn coordinate_and_device_scaling_match_with_equivalent_window_units() {
    let line = skeleton(true);
    for style in ArrowStyle::ALL {
        for pattern in LineStyle::ALL {
            let mut viewport = Pixmap::new(920, 480).unwrap();
            let mut original = Pixmap::new(920, 480).unwrap();
            draw::draw(
                &mut Painter::new(&mut viewport).with_scale(2.0),
                &line,
                style,
                pattern,
                4.0,
                Color::BLACK,
            );
            draw::draw(
                &mut Painter::new(&mut original).with_window_units(
                    mark_shot::features::annotation_style::window::WindowUnits::new(2.0),
                ),
                &line.scaled(2.0),
                style,
                pattern,
                8.0,
                Color::BLACK,
            );
            assert!(original.data() == viewport.data(), "{style:?} {pattern:?}");
        }
    }
}

/// 【箭头测试】【小视区】大曲线只为当前小视区生成有限像素块，不分配完整曲线外框图像
#[test]
fn huge_curves_render_into_small_visible_canvases() {
    let mut line = LineSkeleton::new((-1200.0, 3000.0), (6200.0, -1000.0)).unwrap();
    line.controls = vec![(4200.0, 9500.0), (5100.0, -4500.0)];
    let sampled = geometry::sampled(&line).unwrap();
    let midpoint = sampled.at(sampled.length * 0.5);
    let origin = (midpoint.0.floor() - 80.0, midpoint.1.floor() - 64.0);
    let mut image = Pixmap::new(160, 128).unwrap();
    draw::draw(
        &mut Painter::new(&mut image).with_origin(origin),
        &line,
        ArrowStyle::BidirectionalKde,
        LineStyle::Solid,
        12.0,
        Color::BLACK,
    );
    assert!(
        image
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] > 64)
    );
}

/// 【箭头测试】【退化输入】重合端点、非法控制点或比例不产生残留图像
#[test]
fn invalid_and_degenerate_arrows_leave_no_pixels() {
    let mut line = LineSkeleton::new((30.0, 70.0), (30.0, 70.0)).unwrap();
    for style in ArrowStyle::ALL {
        assert!(
            render(&line, style, LineStyle::Solid, 4.0, Color::BLACK)
                .data()
                .iter()
                .all(|byte| *byte == 0)
        );
    }
    line.end = (420.0, 70.0);
    line.controls.push((f32::NAN, 100.0));
    assert!(
        render(
            &line,
            ArrowStyle::Fletched,
            LineStyle::Solid,
            4.0,
            Color::BLACK
        )
        .data()
        .iter()
        .all(|byte| *byte == 0)
    );
}

/// 【箭头测试】【无效尺寸】头部公开计算接口对非有限长度、笔宽和比例返回有限尺寸，不发生钳制异常
#[test]
fn head_metrics_do_not_panic_on_non_finite_parameters() {
    for style in ArrowStyle::ALL {
        for (length, width, scale) in [
            (f32::NAN, 4.0, 1.0),
            (100.0, f32::NAN, 1.0),
            (100.0, 4.0, f32::INFINITY),
        ] {
            let size = metrics::metrics(length, width, scale, style);
            assert!(size.length.is_finite() && size.length >= 0.0);
            assert!(size.half_width.is_finite() && size.half_width >= 0.0);
        }
    }
}

/// 【箭头测试】【头部编号】四种头部的配置名称和原版编号保持一致，未知值不覆盖默认偏好
#[test]
fn head_styles_keep_reference_configuration_ids() {
    for (index, style) in ArrowStyle::ALL.into_iter().enumerate() {
        assert_eq!(style as usize, index);
        assert_eq!(ArrowStyle::parse(style.key()), Some(style));
        assert_eq!(ArrowStyle::parse(&index.to_string()), Some(style));
    }
    assert_eq!(ArrowStyle::parse("unknown"), None);
}
