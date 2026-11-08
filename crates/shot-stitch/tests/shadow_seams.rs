//! 【滚动截图回归】【阴影接缝】通过真实拼接入口检查正文像素，不以平均误差掩盖灰带

use shot_image::Image;
use shot_stitch::{Axis, Config, Status, Stitcher};

const WIDTH: i32 = 128;
const HEIGHT: i32 = 300;
const FOOTER: i32 = 24;
const SHADOW: i32 = 28;

/// 【滚动截图回归】【正文生成】按 origin 返回确定性正文视口，同一文档坐标始终产生相同像素
fn document(origin: i32) -> Image {
    let pixels = (origin..origin + HEIGHT).flat_map(document_row).collect();
    Image::from_pixels(WIDTH, HEIGHT, pixels).unwrap()
}

/// 【滚动截图回归】【正文行生成】按 vertical 返回文档原始行，供视口生成与导出断言共用
fn document_row(vertical: i32) -> Vec<u32> {
    (0..WIDTH)
        .map(|horizontal| {
            let hash = (vertical as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                ^ (horizontal as u32).wrapping_mul(2_246_822_519);
            0xff00_0000 | (hash & 0x00ff_ffff)
        })
        .collect()
}

/// 【滚动截图回归】【底栏合成】按 origin 生成正文，底部叠加不透明栏及渐变阴影，返回采集帧
fn capture(origin: i32) -> Image {
    styled_capture(origin, false, false, false)
}

/// 【滚动截图回归】【透明变体】按 origin、透明模式、亮色阴影和局部覆盖生成视口，返回真实合成像素
fn styled_capture(origin: i32, translucent: bool, glow: bool, partial: bool) -> Image {
    let source = document(origin);
    let mut pixels = Vec::new();
    for vertical in 0..HEIGHT {
        for (horizontal, &pixel) in source.row(vertical).iter().enumerate() {
            let color = if vertical >= HEIGHT - FOOTER {
                if translucent {
                    blend(pixel, 190, glow)
                } else {
                    0xffee_eeee
                }
            } else if vertical >= HEIGHT - FOOTER - SHADOW {
                let opacity =
                    (vertical - (HEIGHT - FOOTER - SHADOW) + 1) as u32 * 96 / SHADOW as u32;
                if !partial || horizontal < WIDTH as usize / 3 {
                    blend(pixel, opacity, glow)
                } else {
                    pixel
                }
            } else {
                pixel
            };
            pixels.push(color);
        }
    }
    Image::from_pixels(WIDTH, HEIGHT, pixels).unwrap()
}

/// 【滚动截图回归】【遮挡混色】把 pixel 按 opacity 混合至黑色或白色，返回不透明合成像素
fn blend(pixel: u32, opacity: u32, glow: bool) -> u32 {
    let channel = |shift: u32| {
        (((pixel >> shift) & 255_u32) * (255 - opacity) + if glow { 255 * opacity } else { 0 })
            / 255
    };
    0xff00_0000 | channel(16) << 16 | channel(8) << 8 | channel(0)
}

/// 【滚动截图回归】【垂直翻转】按 image 返回逐行翻转的独立图像，用于对称验证顶部阴影
fn flip(image: Image) -> Image {
    let pixels = (0..image.height())
        .rev()
        .flat_map(|vertical| image.row(vertical).iter().copied())
        .collect();
    Image::from_pixels(image.width(), image.height(), pixels).unwrap()
}

/// 【滚动截图回归】【正文断言】按 last_origin 验证 actual 的全部正文和最后一次真实尾部，无返回值
fn check(actual: &Image, last_origin: i32, last: &Image) {
    assert_eq!(
        (actual.width(), actual.height()),
        (WIDTH, HEIGHT + last_origin)
    );
    for vertical in 0..actual.height() {
        let expected = document_row(vertical);
        let row = if vertical < last_origin + HEIGHT - FOOTER - SHADOW {
            &expected
        } else {
            last.row(vertical - last_origin)
        };
        assert!(
            actual.row(vertical) == row,
            "正文或末帧尾部不一致，行 {vertical}"
        );
    }
}

/// 【滚动截图回归】【旧阴影替换】无参数，两帧足以揭露旧帧阴影残留，末帧真实尾部仍保留一次
#[test]
fn bottom_shadow_is_replaced_by_clean_overlap() {
    let mut stitcher = Stitcher::new(Config {
        min_overlap: 64,
        ..Config::default()
    })
    .unwrap();
    stitcher.push(capture(0)).unwrap();
    let result = stitcher.push(capture(72)).unwrap();
    assert_eq!(result.status, Status::Appended);
    assert_eq!(result.added, 72);
    let actual = stitcher.full_image().unwrap().unwrap();
    assert_eq!(actual.height(), HEIGHT + 72);
    let expected = document(0);
    for vertical in 0..HEIGHT {
        assert!(
            actual.row(vertical) == expected.row(vertical),
            "旧帧阴影残留于正文行 {vertical}"
        );
    }
}

/// 【滚动截图回归】【固定区限制】无参数，固定主体检测只识别不透明底栏，阴影必须由正文重叠替换
#[test]
fn fixed_detector_stops_at_the_translucent_shadow() {
    let observed =
        shot_stitch::fixed_regions::FixedRegions::default().observe(&capture(0), &capture(72), 72);
    assert_eq!(observed.footer, FOOTER);
}

/// 【滚动截图回归】【多轴多步】无参数，正反双轴、大小步长和深浅局部阴影都保持原始正文像素
#[test]
fn repeated_shadows_preserve_content_in_both_directions_and_axes() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            for step in [16, 24, 72, 120] {
                for (translucent, glow, partial) in [
                    (false, false, false),
                    (true, false, false),
                    (true, true, false),
                    (false, false, true),
                ] {
                    let mut stitcher = Stitcher::new(Config {
                        min_overlap: 64,
                        ..Config::default()
                    })
                    .unwrap();
                    stitcher.set_axis(axis).unwrap();
                    for index in 0..=4 {
                        let mut image = styled_capture(index * step, translucent, glow, partial);
                        if reverse {
                            image = flip(image);
                        }
                        if axis == Axis::Horizontal {
                            image = image.transpose().unwrap();
                        }
                        let result = stitcher.push(image).unwrap();
                        let expected = if index == 0 {
                            Status::FirstFrame
                        } else {
                            Status::Appended
                        };
                        assert_eq!(
                            result.status, expected,
                            "axis={axis:?}, reverse={reverse}, step={step}, frame={index}, translucent={translucent}, glow={glow}, partial={partial}"
                        );
                        if index > 0 {
                            assert_eq!(result.added, step);
                        }
                    }
                    let mut actual = stitcher.full_image().unwrap().unwrap();
                    if axis == Axis::Horizontal {
                        actual = actual.transpose().unwrap();
                    }
                    if reverse {
                        actual = flip(actual);
                    }
                    check(
                        &actual,
                        step * 4,
                        &styled_capture(step * 4, translucent, glow, partial),
                    );
                }
            }
        }
    }
}

/// 【滚动截图回归】【覆盖不足】无参数，缺少干净连接区域时拒绝污染帧，返回较近位置后仍可继续
#[test]
fn insufficient_clean_overlap_preserves_image_and_recovers() {
    let mut stitcher = Stitcher::new(Config {
        min_overlap: 64,
        ..Config::default()
    })
    .unwrap();
    stitcher.push(capture(0)).unwrap();
    let revision = stitcher.revision();
    let before = stitcher.full_image().unwrap().unwrap();
    assert_eq!(stitcher.push(capture(228)).unwrap().status, Status::NoMatch);
    assert_eq!(stitcher.revision(), revision);
    assert_eq!(stitcher.full_image().unwrap().unwrap(), before);
    assert_eq!(stitcher.push(capture(72)).unwrap().status, Status::Appended);
    check(&stitcher.full_image().unwrap().unwrap(), 72, &capture(72));
}

/// 【滚动截图回归】【反向重访】无参数，访问已拼接内容再向两端增长时，不把中间帧的底栏写入正文
#[test]
fn revisiting_and_switching_growth_edges_keeps_the_original_document() {
    let mut stitcher = Stitcher::new(Config {
        min_overlap: 64,
        ..Config::default()
    })
    .unwrap();
    for origin in [300, 372, 444, 372, 300, 228] {
        let result = stitcher.push(capture(origin)).unwrap();
        assert_ne!(result.status, Status::NoMatch, "origin={origin}");
    }
    let actual = stitcher.full_image().unwrap().unwrap();
    assert_eq!(actual.height(), HEIGHT + 444 - 228);
    let last = capture(444);
    for vertical in 0..actual.height() {
        let expected = document_row(vertical + 228);
        let row = if vertical + 228 < 444 + HEIGHT - FOOTER - SHADOW {
            &expected
        } else {
            last.row(vertical + 228 - 444)
        };
        assert!(
            actual.row(vertical) == row,
            "反向重访后正文行 {vertical} 发生变化"
        );
    }
}

/// 【滚动截图回归】【动态底栏】无参数，不透明、透明、亮色及局部遮挡切换后，旧阴影不能固化在正文
#[test]
fn changing_footer_appearance_does_not_leave_old_shadows() {
    let mut stitcher = Stitcher::new(Config {
        min_overlap: 64,
        ..Config::default()
    })
    .unwrap();
    let styles = [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (false, false, true),
        (false, false, false),
    ];
    for (index, (translucent, glow, partial)) in styles.into_iter().enumerate() {
        let result = stitcher
            .push(styled_capture(
                index as i32 * 72,
                translucent,
                glow,
                partial,
            ))
            .unwrap();
        assert_eq!(
            result.status,
            if index == 0 {
                Status::FirstFrame
            } else {
                Status::Appended
            },
            "frame={index}"
        );
    }
    check(&stitcher.full_image().unwrap().unwrap(), 288, &capture(288));
}

/// 【滚动截图回归】【重采样视口】按 origin 和 phase 生成均衡像素误差，模拟分数缩放的采样相位变化
fn resampled_frame(origin: i32, phase: i32) -> Image {
    let pixels = (origin..origin + HEIGHT)
        .flat_map(|vertical| {
            document_row(vertical)
                .into_iter()
                .enumerate()
                .map(move |(horizontal, pixel)| {
                    let delta = if (horizontal as i32 + phase) % 2 == 0 {
                        3
                    } else {
                        -3
                    };
                    let channel =
                        |shift: u32| ((((pixel >> shift) & 255) as i32 / 2 + 64) + delta) as u32;
                    0xff00_0000 | channel(16) << 16 | channel(8) << 8 | channel(0)
                })
        })
        .collect();
    Image::from_pixels(WIDTH, HEIGHT, pixels).unwrap()
}

/// 【滚动截图回归】【均衡重采样】无参数，正常相位误差不能拒绝新帧，也不能无故覆盖已有清晰内容
#[test]
fn fractional_sampling_keeps_the_original_overlap() {
    let first = resampled_frame(0, 0);
    let second = resampled_frame(72, 1);
    let mut stitcher = Stitcher::new(Config {
        min_overlap: 64,
        ..Config::default()
    })
    .unwrap();
    stitcher.push(first.clone()).unwrap();
    let result = stitcher.push(second.clone()).unwrap();
    assert_eq!((result.status, result.added), (Status::Appended, 72));
    let actual = stitcher.full_image().unwrap().unwrap();
    for vertical in 0..actual.height() {
        let expected = if vertical < HEIGHT {
            first.row(vertical)
        } else {
            second.row(vertical - 72)
        };
        assert!(
            actual.row(vertical) == expected,
            "重采样场景改变了已有正文行 {vertical}"
        );
    }
}
