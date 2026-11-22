//! 【长图拼接】【抗锯齿回归】同一正文切换灰度与子像素渲染时仍保持位置和连续续接

use shot_image::Image;
use shot_stitch::{Axis, Config, Status, Stitcher};

/// 【长图拼接】【独立字形】按 x、y 返回具有文档唯一纹理的灰度字形，空白返回白色
fn gray(x: i32, y: i32) -> u32 {
    if (16..270).contains(&x) && y.rem_euclid(52) < 20 && x % 7 < 4 {
        45 + ((y as u32).wrapping_mul(2_654_435_761) ^ (x as u32).wrapping_mul(2_246_822_519)) % 110
    } else {
        255
    }
}

/// 【长图拼接】【渲染视口】按 origin、lcd 返回独立列表视口，lcd 只改变同一字形的通道覆盖率
fn viewport(origin: i32, lcd: bool) -> Image {
    let pixels = (origin..origin + 420)
        .flat_map(|y| {
            (0..320).map(move |x| {
                let value = gray(x, y);
                let (red, blue) = if lcd && value != 255 {
                    (value - 36, value + 36)
                } else {
                    (value, value)
                };
                0xff00_0000 | red << 16 | value << 8 | blue
            })
        })
        .collect();
    Image::from_pixels(320, 420, pixels).unwrap()
}

/// 【长图拼接】【静止切换】无参数和返回值，双轴在抗锯齿切换后更新锚点，保持已有原图并继续增长
#[test]
fn rendering_switch_keeps_the_anchor_and_original_pixels() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let orient = |image: Image| {
            if axis == Axis::Horizontal {
                image.transpose().unwrap()
            } else {
                image
            }
        };
        let first = orient(viewport(0, false));
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        stitcher.set_axis(axis).unwrap();
        stitcher.push(first.clone()).unwrap();
        let revision = stitcher.revision();
        // 1. 【长图拼接】【静止重绘】同位置的渲染切换不增长、不覆盖清晰像素
        let result = stitcher.push(orient(viewport(0, true))).unwrap();
        assert_eq!((result.status, result.position), (Status::NoProgress, 0));
        assert_eq!(stitcher.revision(), revision);
        assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
        // 2. 【长图拼接】【后续续接】使用更新后的渲染帧继续移动，新增长度等于实际位移
        let result = stitcher.push(orient(viewport(83, true))).unwrap();
        assert_eq!((result.status, result.added), (Status::Appended, 83));
    }
}

/// 【长图拼接】【长图切换】无参数和返回值，一万像素后的渲染切换不影响持续续接，原始字形保持完整
#[test]
fn rendering_switches_keep_stitching_beyond_twenty_thousand_pixels() {
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    for index in 0..=250 {
        let origin = index * 83;
        let lcd = origin >= 10000 && index % 2 == 0;
        let result = stitcher.push(viewport(origin, lcd)).unwrap();
        assert_eq!(
            result.status,
            if index == 0 {
                Status::FirstFrame
            } else {
                Status::Appended
            },
            "origin={origin}: {result:?}"
        );
        assert_eq!(stitcher.dimensions(), (320, origin + 420));
    }
    // 1. 【长图拼接】【字形核验】灰度与子像素渲染的平均亮度一致，逐像素确认文档字形没有重排
    let image = stitcher.full_image().unwrap().unwrap();
    for y in 0..image.height() {
        for (x, &pixel) in image.row(y).iter().enumerate() {
            let mean = ((pixel >> 16 & 255) + (pixel >> 8 & 255) + (pixel & 255)) / 3;
            assert_eq!(mean, gray(x as i32, y), "pixel=({x},{y})");
        }
    }
}

/// 【长图拼接】【误匹配拒绝】无参数和返回值，抗锯齿容忍不能接收另一份正文或错位视口
#[test]
fn rendering_tolerance_rejects_changed_glyphs_and_unreachable_frames() {
    for origin in [0, 3000] {
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        let first = viewport(0, false);
        stitcher.push(first.clone()).unwrap();
        let mut other = viewport(origin, true);
        if origin == 0 {
            let pixels = (0..420)
                .flat_map(|y| {
                    other.row(y).iter().map(|&p| {
                        if p == 0xffff_ffff {
                            p
                        } else {
                            0xff00_0000 | (p ^ 0x00ff_ffff)
                        }
                    })
                })
                .collect();
            other = Image::from_pixels(320, 420, pixels).unwrap();
        }
        let result = stitcher.push(other).unwrap();
        assert_eq!(
            result.status,
            Status::NoMatch,
            "origin={origin}: {result:?}"
        );
        assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
    }
}

/// 【长图拼接】【通道范围反例】无参数和返回值，覆盖全部灰度值的彩色图案仍需保留字形，不能仅凭通道范围匹配
#[test]
fn broad_color_ranges_cannot_replace_glyph_geometry() {
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    let first = viewport(0, false);
    stitcher.push(first.clone()).unwrap();
    let pixels = (0..420)
        .flat_map(|y| (0..320).map(move |x| 0xff00_00ff | (((x * 37 + y * 71) % 256) as u32) << 8))
        .collect();
    let unrelated = Image::from_pixels(320, 420, pixels).unwrap();
    assert_eq!(stitcher.push(unrelated).unwrap().status, Status::NoMatch);
    assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
}

/// 【长图拼接】【相位反转视口】按 switched 返回独立交替字形，切换后反转亮暗并改变通道覆盖，局部平均亮度保持相同
fn reversed_phase(switched: bool) -> Image {
    let pixels = (0..420)
        .flat_map(|y| {
            (0..320).map(move |x| {
                let amplitude = 16 + y * 37 % 17;
                let value = if (x % 2 == 0) ^ switched {
                    128 - amplitude
                } else {
                    128 + amplitude
                };
                if switched {
                    0xff00_00ff | ((value * 3 - 255) as u32) << 8
                } else {
                    0xff00_0000 | (value as u32 * 0x0001_0101)
                }
            })
        })
        .collect();
    Image::from_pixels(320, 420, pixels).unwrap()
}

/// 【长图拼接】【相位反转拒绝】无参数和返回值，双轴的字形亮暗反转不能通过抗锯齿检查或错误增长
#[test]
fn opposite_edge_directions_cannot_match_through_equal_local_averages() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let orient = |image: Image| {
            if axis == Axis::Horizontal {
                image.transpose().unwrap()
            } else {
                image
            }
        };
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        stitcher.set_axis(axis).unwrap();
        let first = orient(reversed_phase(false));
        stitcher.push(first.clone()).unwrap();
        assert_eq!(
            stitcher.push(orient(reversed_phase(true))).unwrap().status,
            Status::NoMatch
        );
        assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
    }
}

/// 【长图拼接】【字形消失拒绝】无参数和返回值，双轴在强字形全部消失时必须拒绝，平坦颜色不能借弱边缘规则匹配
#[test]
fn strong_edges_cannot_disappear_into_flat_color() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let orient = |image: Image| {
            if axis == Axis::Horizontal {
                image.transpose().unwrap()
            } else {
                image
            }
        };
        let first = orient(reversed_phase(false));
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        stitcher.set_axis(axis).unwrap();
        stitcher.push(first.clone()).unwrap();
        let flat = orient(Image::from_pixels(320, 420, vec![0xff00_81ff; 320 * 420]).unwrap());
        assert_eq!(stitcher.push(flat).unwrap().status, Status::NoMatch);
        assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
    }
}
