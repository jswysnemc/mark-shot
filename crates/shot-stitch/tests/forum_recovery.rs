//! 【论坛长截图】【局部更新回归】无固定区域的列表中，计数更新不能中断一万像素后的拼接

use shot_image::Image;
use shot_stitch::{Config, Status, Stitcher};

/// 【论坛长截图】【列表生成】按 origin、width、height 和 revision 返回标题、头像和计数构成的视口，revision 只改变右侧局部计数
fn forum(origin: i32, width: i32, height: i32, revision: i32) -> Image {
    let pixels = (origin..origin + height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let hash = (y as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                    ^ (x as u32).wrapping_mul(2_246_822_519);
                let line = y.rem_euclid(52);
                let value = if line == 51 {
                    238
                } else if (16..width * 2 / 3).contains(&x) && line < 18 && x % 7 < 4 {
                    30 + hash % 100
                } else if (width * 3 / 4..width * 3 / 4 + 20).contains(&x)
                    && (2..22).contains(&line)
                {
                    50 + hash % 160
                } else if (width - 50..width - 20).contains(&x) && (5..15).contains(&line) {
                    80 + (y / 52 + revision * 17).rem_euclid(100) as u32
                } else {
                    255
                };
                0xff00_0000 | value << 16 | value << 8 | value
            })
        })
        .collect();
    Image::from_pixels(width, height, pixels).unwrap()
}

/// 【论坛长截图】【持续续接】无参数和返回值，达到一万或一万五千像素后计数更新仍可连续拼接，稳定正文逐像素不变
#[test]
fn list_counter_updates_keep_stitching_beyond_fifteen_thousand_pixels() {
    // 1. 【论坛长截图】【长列表输入】相同标题及头像按文档坐标移动，局部计数在指定高度开始更新
    for threshold in [10000, 15000] {
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        for index in 0..=220 {
            let origin = index * 83;
            let revision = if origin >= threshold { index } else { 0 };
            let result = stitcher.push(forum(origin, 320, 420, revision)).unwrap();
            assert_eq!(
                result.status,
                if index == 0 {
                    Status::FirstFrame
                } else {
                    Status::Appended
                },
                "threshold={threshold}, origin={origin}, result={result:?}"
            );
            assert_eq!(stitcher.dimensions(), (320, origin + 420));
        }
        // 2. 【论坛长截图】【稳定正文核验】计数允许显示采集时值，所有标题和头像必须保留原始像素
        let image = stitcher.full_image().unwrap().unwrap();
        let reference = forum(0, 320, image.height(), 0);
        for y in 0..image.height() {
            assert_eq!(&image.row(y)[..270], &reference.row(y)[..270], "row={y}");
        }
    }
}

/// 【论坛长截图】【局部变化重访】无参数和返回值，更新后的历史位置可以恢复锚点，异常过渡帧不能阻碍随后补采
#[test]
fn changed_list_rows_recover_after_a_rejected_transition() {
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    // 1. 【论坛长截图】【原始长图】建立没有固定区域的稳定列表
    for origin in (0..=15000).step_by(100) {
        stitcher.push(forum(origin, 320, 420, 0)).unwrap();
    }
    let revision = stitcher.revision();
    let unrelated = Image::from_pixels(320, 420, vec![0xff70_90b0; 320 * 420]).unwrap();
    assert_eq!(stitcher.push(unrelated).unwrap().status, Status::NoMatch);
    assert_eq!(stitcher.revision(), revision);
    // 2. 【论坛长截图】【历史恢复】重新识别分散正文，不能要求所有计数行都保持不变
    for origin in [15000, 10137, 15000] {
        let result = stitcher.push(forum(origin, 320, 420, 4)).unwrap();
        assert_eq!(
            (result.status, result.position),
            (Status::NoProgress, origin),
            "{result:?}"
        );
    }
    let result = stitcher.push(forum(15083, 320, 420, 5)).unwrap();
    assert_eq!(
        (result.status, result.added),
        (Status::Appended, 83),
        "{result:?}"
    );
}

/// 【论坛长截图】【相同排版误拼】无参数和返回值，重复头像及空白间隔不能掩盖所有主题文字已改变的另一份列表
#[test]
fn matching_avatars_and_blank_gaps_cannot_hide_unrelated_titles() {
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    let first = forum(0, 320, 420, 0);
    stitcher.push(first.clone()).unwrap();
    let mut pixels = Vec::new();
    // 1. 【论坛长截图】【标题替换】保留相同排版、头像和计数，只替换每行所有标题像素
    for y in 0..420 {
        for (x, &pixel) in first.row(y).iter().enumerate() {
            let changed = (16..320 * 2 / 3).contains(&(x as i32)) && y % 52 < 18 && x % 7 < 4;
            pixels.push(if changed {
                0xff00_0000 | (pixel ^ 0x00ff_ffff)
            } else {
                pixel
            });
        }
    }
    let other = Image::from_pixels(320, 420, pixels).unwrap();
    let result = stitcher.push(other).unwrap();
    assert_eq!(result.status, Status::NoMatch, "{result:?}");
    // 2. 【论坛长截图】【原图保持】误匹配不能修改原图或增长长度
    assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
}

/// 【论坛长截图】【统计碰撞页面】按 origin、switched、period 返回视口，标题反转保留均值与方差，头像保持独立文档纹理
fn colliding_titles(origin: i32, switched: bool, period: i32) -> Image {
    let pixels = (origin..origin + 448)
        .flat_map(|y| {
            (0..320).map(move |x| {
                let line = y.rem_euclid(period);
                let value = if line == period - 1 {
                    238
                } else if (16..212).contains(&x) && line < 18 {
                    if (x % 2 == 0) ^ switched { 20 } else { 230 }
                } else if (240..270).contains(&x) && (2..28).contains(&line) {
                    20 + (y * 37 + x * 17).rem_euclid(160) as u32
                } else {
                    255
                };
                0xff00_0000 | value << 16 | value << 8 | value
            })
        })
        .collect();
    Image::from_pixels(320, 448, pixels).unwrap()
}

/// 【论坛长截图】【分散纹理误拼】无参数和返回值，头像延伸到标题下方也不能掩盖整份正文替换，双轴都必须拒绝
#[test]
fn matching_avatar_texture_cannot_hide_titles_with_identical_statistics() {
    for axis in [shot_stitch::Axis::Vertical, shot_stitch::Axis::Horizontal] {
        for period in [52, 64, 80, 100] {
            let mut stitcher = Stitcher::new(Config::default()).unwrap();
            stitcher.set_axis(axis).unwrap();
            let orient = |frame: Image| {
                if axis == shot_stitch::Axis::Horizontal {
                    frame.transpose().unwrap()
                } else {
                    frame
                }
            };
            let first = orient(colliding_titles(0, false, period));
            stitcher.push(first.clone()).unwrap();
            // 1. 【论坛长截图】【主要正文替换】重复头像提供清晰纹理，所有标题变成另一份内容，不能选择空白段强行续接
            let revision = stitcher.revision();
            let result = stitcher
                .push(orient(colliding_titles(83, true, period)))
                .unwrap();
            assert_eq!(
                result.status,
                Status::NoMatch,
                "{axis:?}, {period}: {result:?}"
            );
            assert_eq!(stitcher.revision(), revision);
            assert_eq!(stitcher.full_image().unwrap().unwrap(), first);
        }
    }
}
