//! 【长截图回归】【尺寸与恢复】通过生产拼接入口核对宽背景窄正文、累计高度和深度重访

use shot_image::Image;
use shot_stitch::{Config, Status, Stitcher};

/// 【长截图回归】【页面纹理】按 origin、width、height 和 narrow 返回确定性文档视口，窄正文保持四十像素宽
fn document(origin: i32, width: i32, height: i32, narrow: bool) -> Image {
    document_at(origin, width, height, narrow.then_some(width / 2 - 100))
}

/// 【长截图回归】【正文位置】按 origin、width、height 和 left 返回纹理视口，left 为空时生成全宽正文
fn document_at(origin: i32, width: i32, height: i32, left: Option<i32>) -> Image {
    let pixels = (origin..origin + height)
        .flat_map(|vertical| {
            (0..width).map(move |horizontal| {
                let hash = (vertical as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                    ^ (horizontal as u32).wrapping_mul(2_246_822_519);
                if let Some(left) = left {
                    let value = if (left..left + 40).contains(&horizontal) && vertical % 28 < 16 {
                        20 + hash % 140
                    } else {
                        250
                    };
                    0xff00_0000 | value << 16 | value << 8 | value
                } else {
                    0xff00_0000 | hash & 0x00ff_ffff
                }
            })
        })
        .collect();
    Image::from_pixels(width, height, pixels).unwrap()
}

/// 【长截图回归】【正文分布】无参数和返回值，窄正文位于选区边缘或采样带之间时也不能遗漏滚动
#[test]
fn narrow_content_at_capture_edges_and_band_boundaries() {
    // 1. 【长截图回归】【位置矩阵】改变正文在选区中的横向位置和实际滚动步长
    for width in [64, 128, 320, 1280, 3840] {
        for left in [0, width / 4, width / 2 - 20, width - 40] {
            let mut stitcher = Stitcher::new(Config::default()).unwrap();
            for (index, origin) in [0, 83, 196, 257, 440].into_iter().enumerate() {
                let result = stitcher
                    .push(document_at(origin, width, 400, Some(left)))
                    .unwrap();
                assert_eq!(
                    result.status,
                    if index == 0 {
                        Status::FirstFrame
                    } else {
                        Status::Appended
                    },
                    "width={width}, left={left}, origin={origin}, result={result:?}"
                );
                assert_eq!(stitcher.dimensions(), (width, origin + 400));
            }
            // 2. 【长截图回归】【完整核验】每个位置都必须保留整幅原始文档
            assert_eq!(
                stitcher.full_image().unwrap().unwrap(),
                document_at(0, width, 840, Some(left))
            );
        }
    }
}

/// 【长截图回归】【统计碰撞帧】按 origin、switched、width、strip 返回窄纹理和横向交替色块，色块反转不改变亮度统计
fn colliding_frame(origin: i32, switched: bool, width: i32, strip: i32) -> Image {
    let pixels = (0..400)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let value = if x < strip {
                    ((origin + y) * 37 + x * 13).rem_euclid(251) as u32
                } else if (x % 2 == 0) ^ switched {
                    20
                } else {
                    230
                };
                0xff00_0000 | value << 16 | value << 8 | value
            })
        })
        .collect();
    Image::from_pixels(width, 400, pixels).unwrap()
}

/// 【长截图回归】【窄纹理误拼】无参数和返回值，少量吻合条纹不能掩盖其余正文像素反转，拒绝后仍能继续补采
#[test]
fn matching_strip_cannot_hide_changes_with_identical_luminance_statistics() {
    // 1. 【长截图回归】【反例矩阵】正文均值和方差相同，原始像素证据必须发现页面变化
    for width in [160, 320, 1280] {
        for strip in [8, 12] {
            let mut stitcher = Stitcher::new(Config::default()).unwrap();
            let first = colliding_frame(0, false, width, strip);
            stitcher.push(first.clone()).unwrap();
            let result = stitcher
                .push(colliding_frame(120, true, width, strip))
                .unwrap();
            assert_eq!(
                result.status,
                Status::NoMatch,
                "width={width}, strip={strip}, result={result:?}"
            );
            assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
            // 2. 【长截图回归】【锚点恢复】拒绝帧不能影响随后具有真实重叠的原页面
            let result = stitcher
                .push(colliding_frame(83, false, width, strip))
                .unwrap();
            assert_eq!(
                (result.status, result.added),
                (Status::Appended, 83),
                "{result:?}"
            );
        }
    }
}

/// 【长截图回归】【重复分隔行】按 origin 返回三十二像素周期的相同分隔线与独立正文，用于检验历史候选歧义
fn separated_frame(origin: i32) -> Image {
    let pixels = (origin..origin + 400)
        .flat_map(|y| {
            let mut hash = (y as u32 + 1).wrapping_mul(2_654_435_761);
            hash ^= hash >> 16;
            hash = hash.wrapping_mul(2_246_822_519);
            hash ^= hash >> 13;
            let value = match y % 32 {
                0 => 0,
                1 => 250,
                _ => 50 + hash % 150,
            };
            std::iter::repeat_n(0xff00_0000 | value << 16 | value << 8 | value, 32)
        })
        .collect();
    Image::from_pixels(32, 400, pixels).unwrap()
}

/// 【长截图回归】【重复边缘恢复】无参数和返回值，多于二十四个相同分隔线不能挤掉具有不同正文的真实历史位置
#[test]
fn recurring_separator_edges_do_not_exhaust_history_candidates() {
    // 1. 【长截图回归】【历史准备】在不同累计高度内保留大量相同高对比分隔线
    for maximum in [800, 1600, 3200, 6400] {
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        for origin in (0..=maximum).step_by(100) {
            let result = stitcher.push(separated_frame(origin)).unwrap();
            assert_ne!(result.status, Status::NoMatch, "origin={origin}");
        }
        // 2. 【长截图回归】【正文定位】按真实正文恢复早期位置，不能仅凭分隔线的重复分数截断候选
        let result = stitcher.push(separated_frame(128)).unwrap();
        assert_eq!(
            (result.status, result.position),
            (Status::NoProgress, 128),
            "maximum={maximum}, result={result:?}"
        );
    }
}

/// 【长截图回归】【宽背景正文】无参数和返回值，增加空白宽度不能降低窄正文的位移识别或改变原始像素
#[test]
fn narrow_content_stitches_at_every_capture_width() {
    // 1. 【长截图回归】【宽度矩阵】正文宽度固定，仅改变两侧空白及选区总宽度
    for width in [320, 640, 1280, 2560, 3840] {
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        // 2. 【长截图回归】【连续拼接】每帧必须按真实位移增长，不允许失败后静默跳过正文
        for step in 0..=24 {
            let origin = step * 83;
            let result = stitcher.push(document(origin, width, 400, true)).unwrap();
            assert_eq!(
                result.status,
                if step == 0 {
                    Status::FirstFrame
                } else {
                    Status::Appended
                },
                "width={width}, frame={step}, result={result:?}"
            );
            assert_eq!(stitcher.dimensions(), (width, origin + 400));
        }
        // 3. 【长截图回归】【像素核验】导出长图必须与完整文档逐行一致
        assert_eq!(
            stitcher.full_image().unwrap().unwrap(),
            document(0, width, 2392, true)
        );
    }
}

/// 【长截图回归】【深度重访】无参数和返回值，长图中任意像素位置的重访都能恢复锚点并继续末端增长
#[test]
fn deep_revisits_find_positions_between_coarse_search_rows() {
    // 1. 【长截图回归】【高度矩阵】使用同一视口逐步生成不同累计高度的长图
    for last_origin in [1000, 4000, 10000] {
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        for origin in (0..=last_origin).step_by(100) {
            let result = stitcher.push(document(origin, 96, 400, false)).unwrap();
            assert_ne!(result.status, Status::NoMatch, "origin={origin}");
        }
        // 2. 【长截图回归】【任意重访】目标位置不对齐八像素网格，恢复时不能误报无重叠
        for origin in [137, 511, last_origin / 2 + 3, last_origin] {
            let result = stitcher.push(document(origin, 96, 400, false)).unwrap();
            assert_eq!(
                result.status,
                Status::NoProgress,
                "height={last_origin}, origin={origin}, result={result:?}"
            );
            assert_eq!(result.position, origin);
        }
        // 3. 【长截图回归】【继续增长】重访后再次追加，原有内容与新增内容都必须完整保留
        let result = stitcher
            .push(document(last_origin + 100, 96, 400, false))
            .unwrap();
        assert_eq!((result.status, result.added), (Status::Appended, 100));
        assert_eq!(
            stitcher.full_image().unwrap().unwrap(),
            document(0, 96, last_origin + 500, false)
        );
    }
}

/// 【长截图回归】【累计长图】无参数和返回值，三万像素以上的窄正文长图仍逐帧成功且像素完整
#[test]
fn narrow_content_keeps_growing_beyond_thirty_thousand_pixels() {
    // 1. 【长截图回归】【长文准备】固定宽背景和四百像素视口，覆盖累计高度阈值
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    // 2. 【长截图回归】【持续增长】每次追加二百像素，不因累计帧数下降为拒绝或重复
    for step in 0..=160 {
        let result = stitcher
            .push(document(step * 200, 1280, 400, true))
            .unwrap();
        assert_eq!(
            result.status,
            if step == 0 {
                Status::FirstFrame
            } else {
                Status::Appended
            },
            "frame={step}, result={result:?}"
        );
        assert_eq!(stitcher.dimensions(), (1280, step * 200 + 400));
    }
    // 3. 【长截图回归】【完整输出】独立文档生成器检查所有行，避免仅尺寸正确却漏拼或错拼
    let view = stitcher.image_view().unwrap();
    for vertical in 0..view.height() {
        let expected = document(vertical, 1280, 1, true);
        for horizontal in 0..view.width() {
            assert_eq!(
                view.pixel(horizontal, vertical),
                expected.row(0)[horizontal as usize],
                "x={horizontal}, y={vertical}"
            );
        }
    }
}
