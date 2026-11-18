//! 【箭头测试】【尾部轮廓】实际描边区分尖状与圆形尾部，双向保留两个头部

use super::*;
use crate::features::line::{LineSkeleton, LineStyle};
use tiny_skia::{Color, Pixmap};

/// 【箭头测试】【实际像素】按 style 和 pattern 返回水平箭头的生产渲染结果，不使用预览图代替正文
fn render(style: ArrowStyle, pattern: LineStyle) -> Pixmap {
    let line = LineSkeleton::new((40.0, 60.0), (180.0, 60.0)).unwrap();
    let mut canvas = Pixmap::new(220, 120).unwrap();
    draw::draw(
        &mut crate::renderer::Painter::new(&mut canvas),
        &line,
        style,
        pattern,
        8.0,
        Color::BLACK,
        Color::TRANSPARENT,
    );
    canvas
}

/// 【箭头测试】【尾端区分】无参数，各头部和线型的尖尾不越过起点，圆尾保留圆帽，双向时两种尾部不会覆盖头部，无返回值
#[test]
fn tail_options_change_single_arrow_pixels_and_preserve_double_heads() {
    for style in ArrowStyle::ALL {
        for pattern in LineStyle::ALL {
            let rounded = render(style, pattern);
            let pointed = render(
                ArrowStyle {
                    tail: ArrowTail::Pointed,
                    ..style
                },
                pattern,
            );
            assert!(rounded.pixel(37, 60).unwrap().alpha() > 0);
            assert_eq!(pointed.pixel(37, 60).unwrap().alpha(), 0);
            assert!(pointed.pixel(55, 60).unwrap().alpha() > 0);
            let double = ArrowStyle {
                bidirectional: true,
                ..style
            };
            assert_eq!(
                render(double, pattern).data(),
                render(
                    ArrowStyle {
                        tail: ArrowTail::Pointed,
                        ..double
                    },
                    pattern
                )
                .data()
            );
        }
    }
}

/// 【箭头测试】【原版尖尾】无参数，320长度24粗细的实线燕尾符合真实 Qt 列宽，三个头部的尖尾中线没有连接孔洞，无返回值
#[test]
fn pointed_solid_tail_matches_qt_taper_without_transparent_seams() {
    let line = LineSkeleton::new((40.0, 200.0), (360.0, 200.0)).unwrap();
    for style in ArrowStyle::ALL {
        let mut canvas = Pixmap::new(420, 400).unwrap();
        draw::draw(
            &mut crate::renderer::Painter::new(&mut canvas),
            &line,
            ArrowStyle {
                tail: ArrowTail::Pointed,
                ..style
            },
            LineStyle::Solid,
            24.0,
            Color::BLACK,
            Color::TRANSPARENT,
        );
        for x in 60..280 {
            assert!(
                canvas.pixel(x, 200).unwrap().alpha() > 0,
                "{style:?} has a tail seam at {x}"
            );
        }
        if style.head == ArrowHead::Fletched {
            // 1. 【箭头测试】【真实样本】旧 Qt 934d8f3 的生产路径在这些位置分别覆盖4、12、22个半透明以上像素
            for (x, width) in [(80, 4), (170, 12), (280, 22)] {
                assert_eq!(
                    (0..400)
                        .filter(|y| canvas.pixel(x, *y).unwrap().alpha() >= 128)
                        .count(),
                    width
                );
            }
        }
    }
}
