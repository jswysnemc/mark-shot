//! 【滚动截图测试】【悬停列表】用随指针切换背景的列表项验证真实帧拼接

use shot_image::Image;
use shot_stitch::{Axis, Config, Edge, Status, Stitcher};

const WIDTH: i32 = 240;
const HEIGHT: i32 = 256;
const STEP: i32 = 144;

/// 【滚动截图测试】【列表视口】按文档起点、悬停项和正文版本生成带边距、文字及悬停底色的视口
fn viewport(origin: i32, hovered: Option<i32>, revision: u32) -> Image {
    let pixels = (origin..origin + HEIGHT)
        .flat_map(|row| {
            (0..WIDTH).map(move |column| {
                let seed = (row as u32).wrapping_mul(2654435761).rotate_left(13)
                    ^ (column as u32 / 3).wrapping_mul(2246822519)
                    ^ revision;
                if (20..190).contains(&column) && (6..28).contains(&(row % 40)) && seed % 7 < 3 {
                    0xff303438
                } else if (12..222).contains(&column) && hovered == Some(row / 40) {
                    0xffc7dcff
                } else {
                    0xfffafafa
                }
            })
        })
        .collect();
    Image::from_pixels(WIDTH, HEIGHT, pixels).unwrap()
}

/// 【滚动截图测试】【误拼保护】静止悬停不能增长，另一份正文不能冒充底色变化，下一帧仍可接续
#[test]
fn hover_only_is_stationary_and_replaced_text_is_rejected() {
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    stitcher.push(viewport(0, None, 0)).unwrap();
    let revision = stitcher.revision();
    for item in [4, 2] {
        let result = stitcher.push(viewport(0, Some(item), 0)).unwrap();
        assert_eq!((result.status, result.position), (Status::NoProgress, 0));
    }
    let other = stitcher.push(viewport(STEP, Some(8), 0xa05b6c4d)).unwrap();
    assert_eq!(other.status, Status::NoMatch);
    assert_eq!(stitcher.revision(), revision);
    assert_eq!(
        stitcher.push(viewport(STEP, Some(8), 0)).unwrap().added,
        STEP
    );
}

/// 【滚动截图测试】【长列表连续高亮】逐帧改变指针下的列表项，验证累计长度超过一万五千像素后仍保持正确位移
#[test]
fn moving_hover_keeps_its_alignment_over_a_long_list() {
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    for origin in (0..=16000).step_by(STEP as usize) {
        let result = stitcher
            .push(viewport(origin, Some((origin + 180) / 40), 0))
            .unwrap();
        assert_eq!(result.position, origin, "{result:?}");
        assert_eq!(stitcher.dimensions(), (WIDTH, origin + HEIGHT));
    }
}

/// 【滚动截图测试】【短重叠】验证两个滚动方向及两轴的增长位置，拼接后每行文字仍与原页面一致
#[test]
fn hovered_list_item_does_not_break_a_short_overlap() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for (first, next, edge) in [(0, STEP, Edge::End), (STEP, 0, Edge::Start)] {
            let orient = |image: Image| {
                if axis == Axis::Vertical {
                    image
                } else {
                    image.transpose().unwrap()
                }
            };
            let mut stitcher = Stitcher::new(Config::default()).unwrap();
            stitcher.set_axis(axis).unwrap();
            // 1. 【滚动截图测试】【悬停切换】滚动前后指针下方的列表项不同，重叠中少于一百行底色保持原样
            stitcher
                .push(orient(viewport(first, Some((first + 180) / 40), 0)))
                .unwrap();
            let result = stitcher
                .push(orient(viewport(next, Some((next + 180) / 40), 0)))
                .unwrap();
            assert_eq!(
                (result.status, result.edge, result.added, result.position),
                (Status::Appended, edge, STEP, next),
                "{axis:?} {edge:?}"
            );
            // 2. 【滚动截图测试】【逐行字形】允许保留采集时的背景，但不能重复、遗漏或挪动任何正文像素
            let output = orient(stitcher.full_image().unwrap().unwrap());
            assert_eq!((output.width(), output.height()), (WIDTH, HEIGHT + STEP));
            for origin in [0, STEP] {
                let reference = viewport(origin, None, 0);
                for row in 0..HEIGHT {
                    for (&actual, &expected) in
                        output.row(origin + row).iter().zip(reference.row(row))
                    {
                        assert_eq!(actual == 0xff303438, expected == 0xff303438);
                    }
                }
            }
        }
    }
}
