//! 【长截图回归】【侧栏功能】沿用 Qt 版的侧栏与目录高亮输入，核对位移和正文像素

use shot_image::Image;
use shot_stitch::{Config, Status, Stitcher};

/// 【长截图回归】【正文颜色】按 row 返回旧版测试的无短周期文档颜色，正文坐标必须非负
fn color(row: i32) -> u32 {
    0xff00_0000
        | (((row * 7 + 31) % 251) as u32) << 16
        | (((row * 11 + 59) % 241) as u32) << 8
        | ((row * 13 + 83) % 239) as u32
}

/// 【长截图回归】【侧栏帧】按 offset、sidebar、right 和 header 返回八百像素宽视口，侧栏及顶栏固定
fn frame(offset: i32, sidebar: i32, right: bool, header: i32) -> Image {
    let pixels = (0..600)
        .flat_map(|y| {
            (0..800).map(move |x| {
                if y < header {
                    0xffd2_dce6
                } else if if right {
                    x >= 800 - sidebar
                } else {
                    x < sidebar
                } {
                    color(y * 3 + 701)
                } else {
                    color(offset + y - header)
                }
            })
        })
        .collect();
    Image::from_pixels(800, 600, pixels).unwrap()
}

/// 【长截图回归】【固定导航】无参数和返回值，左右侧栏、宽侧栏和浏览器顶栏保持完整正文并支持往返补采
#[test]
fn fixed_navigation_preserves_content_and_revisits() {
    // 1. 【长截图回归】【固定区域矩阵】逐帧核验各侧栏及顶栏组合的真实位移
    for (sidebar, right, header) in [
        (224, false, 0),
        (224, true, 0),
        (320, false, 0),
        (224, false, 60),
    ] {
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        for offset in [0, 180, 360, 540, 720] {
            let result = stitcher
                .push(frame(offset, sidebar, right, header))
                .unwrap();
            assert_eq!(
                result.status,
                if offset == 0 {
                    Status::FirstFrame
                } else {
                    Status::Appended
                },
                "sidebar={sidebar}, right={right}, header={header}, offset={offset}, result={result:?}"
            );
            assert_eq!(result.position, offset);
        }
        // 2. 【长截图回归】【往返与补采】检查历史正文像素，再返回任意已知位置并继续增长
        let before = stitcher.full_image().unwrap().unwrap();
        assert_eq!(stitcher.dimensions(), (800, 1320));
        let content_x = if right { 100 } else { 700 };
        for y in header..before.height() {
            assert_eq!(before.row(y)[content_x], color(y - header));
        }
        for offset in [137, 0, 360, 720] {
            let result = stitcher
                .push(frame(offset, sidebar, right, header))
                .unwrap();
            assert_eq!(
                (result.status, result.position),
                (Status::NoProgress, offset),
                "{result:?}"
            );
            assert_eq!(stitcher.full_image().unwrap().unwrap(), before);
        }
        let result = stitcher.push(frame(900, sidebar, right, header)).unwrap();
        assert_eq!((result.status, result.added), (Status::Appended, 180));
    }
}

/// 【长截图回归】【目录高亮】按 offset 和 section 返回旧版动态目录视口，正文宽九百二十像素
fn highlighted_frame(offset: i32, section: i32) -> Image {
    let pixels = (0..800)
        .flat_map(|y| {
            (0..1280).map(move |x| {
                if x >= 360 {
                    color(offset + y)
                } else if (16..344).contains(&x)
                    && (40 + section * 48..80 + section * 48).contains(&y)
                {
                    0xff46_78dc
                } else if y / 24 % 2 == 0 {
                    0xffe8_ebf0
                } else {
                    0xfff5_f6f8
                }
            })
        })
        .collect();
    Image::from_pixels(1280, 800, pixels).unwrap()
}

/// 【长截图回归】【大步长导航】无参数和返回值，目录高亮变化不能影响不等步长拼接或正文像素
#[test]
fn changing_navigation_highlights_keep_large_scroll_steps() {
    // 1. 【长截图回归】【动态目录输入】保留旧版不等步长和目录高亮变化
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    stitcher.push(highlighted_frame(0, 0)).unwrap();
    let mut offset = 0;
    for (index, step) in [240, 410, 180, 460, 300, 520].into_iter().enumerate() {
        offset += step;
        let result = stitcher
            .push(highlighted_frame(offset, index as i32 + 1))
            .unwrap();
        assert_eq!(
            (result.status, result.added, result.position),
            (Status::Appended, step, offset),
            "{result:?}"
        );
    }
    // 2. 【长截图回归】【正文校验】所有累计行必须对应原文档坐标
    let image = stitcher.full_image().unwrap().unwrap();
    assert_eq!(image.height(), 800 + offset);
    for y in 0..image.height() {
        assert_eq!(image.row(y)[700], color(y));
    }
}

/// 【长截图回归】【页面切换】无参数和返回值，只保留相同侧栏的另一份正文不能作为重叠，也不能替换锚点
#[test]
fn unrelated_content_cannot_match_only_the_fixed_navigation() {
    // 1. 【长截图回归】【页面切换拒绝】保持侧栏不变，仅切换到无重叠正文
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    for origin in [0, 180, 360] {
        stitcher.push(frame(origin, 224, false, 60)).unwrap();
    }
    let before = stitcher.full_image().unwrap().unwrap();
    let result = stitcher.push(frame(20000, 224, false, 60)).unwrap();
    assert_eq!(result.status, Status::NoMatch, "{result:?}");
    assert_eq!(stitcher.full_image().unwrap().unwrap(), before);
    // 2. 【长截图回归】【合法帧恢复】原锚点仍支持返回原页面继续增长
    let result = stitcher.push(frame(540, 224, false, 60)).unwrap();
    assert_eq!((result.status, result.added), (Status::Appended, 180));
}
