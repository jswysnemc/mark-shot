//! 【论坛长截图】【浏览器像素】真实浏览器渲染的长列表在局部计数更新后仍能续接，稳定正文必须逐像素一致

use mark_shot::scroll::ScrollSession;
use shot_image::Image;
use shot_stitch::{Axis, Config, Edge, Status, Stitcher};
use tiny_skia::Rect;

const HEIGHT: i32 = 640;
const LAST_ORIGIN: i32 = 25000;

/// 【论坛长截图】【浏览器帧读取】按 phase 读取该阶段的完整浏览器页面，返回独立 RGB 原始像素
fn page(phase: usize) -> image::RgbImage {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scroll-forum");
    // 1. 【论坛长截图】【共享正文】三个阶段共用同一份原始正文，只保存各阶段不同的右侧字段
    let body = image::open(root.join("body.png")).unwrap().to_rgb8();
    let fields = image::open(root.join(format!("fields-{phase}.png")))
        .unwrap()
        .to_rgb8();
    let mut page = image::RgbImage::new(1000, body.height());
    image::imageops::replace(&mut page, &body, 0, 0);
    image::imageops::replace(&mut page, &fields, 690, 0);
    page
}

/// 【论坛长截图】【视口裁切】按 source、origin 返回浏览器原图的六百四十行视口，不重采样像素
fn viewport(source: &image::RgbImage, origin: i32) -> Image {
    let pixels = (origin..origin + HEIGHT)
        .flat_map(|y| {
            (0..source.width()).map(move |x| {
                let pixel = source.get_pixel(x, y as u32).0;
                0xff00_0000
                    | u32::from(pixel[0]) << 16
                    | u32::from(pixel[1]) << 8
                    | u32::from(pixel[2])
            })
        })
        .collect();
    Image::from_pixels(source.width() as i32, HEIGHT, pixels).unwrap()
}

/// 【论坛长截图】【正文核验】按 source 和 view 比较全部标题及标签像素，无返回值，右侧头像及更新字段允许显示采集时值
fn check_body(source: &image::RgbImage, view: shot_stitch::StitchedView<'_>) {
    assert_eq!((view.width(), view.height()), (1000, LAST_ORIGIN + HEIGHT));
    for y in 0..view.height() {
        for x in 0..690 {
            let pixel = source.get_pixel(x as u32, y as u32).0;
            let expected = 0xff00_0000
                | u32::from(pixel[0]) << 16
                | u32::from(pixel[1]) << 8
                | u32::from(pixel[2]);
            assert_eq!(view.pixel(x, y), expected, "x={x}, y={y}");
        }
    }
}

/// 【论坛长截图】【最小复现】无参数和返回值，仅两帧的局部头像变化也必须正确拼接，累计高度不是拒绝的必要条件
#[test]
fn two_browser_frames_with_local_updates_have_enough_overlap() {
    let before = page(0);
    let changed = page(1);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for (first, next, edge) in [(0, 200, Edge::End), (200, 0, Edge::Start)] {
            let mut stitcher = Stitcher::new(Config::default()).unwrap();
            stitcher.set_axis(axis).unwrap();
            let orient = |frame: Image| {
                if axis == Axis::Horizontal {
                    frame.transpose().unwrap()
                } else {
                    frame
                }
            };
            // 1. 【论坛长截图】【清晰段分散】正文和头像只占一行的一部分，干净重叠被局部变化切成短段
            stitcher.push(orient(viewport(&before, first))).unwrap();
            let result = stitcher.push(orient(viewport(&changed, next))).unwrap();
            assert_eq!(
                (result.status, result.position, result.added, result.edge),
                (Status::Appended, next, 200, edge),
                "{axis:?}: {result:?}"
            );
            // 2. 【论坛长截图】【双向正文】横向转回页面坐标后，前插和追加都必须保留全部稳定标题像素
            let mut image = stitcher.full_image().unwrap().unwrap();
            if axis == Axis::Horizontal {
                image = image.transpose().unwrap();
            }
            assert_eq!((image.width(), image.height()), (1000, HEIGHT + 200));
            for y in 0..image.height() {
                for x in 0..690 {
                    let pixel = before.get_pixel(x as u32, y as u32).0;
                    let expected = 0xff00_0000
                        | u32::from(pixel[0]) << 16
                        | u32::from(pixel[1]) << 8
                        | u32::from(pixel[2]);
                    assert_eq!(
                        image.row(y)[x as usize],
                        expected,
                        "{axis:?}, {edge:?}: {x}, {y}"
                    );
                }
            }
        }
    }
}

/// 【论坛长截图】【真实长列表】无参数和返回值，超过一万或一万五千像素后每帧更新回复和时间，续接位置及正文保持准确
#[test]
fn browser_forum_counters_keep_growing_after_ten_and_fifteen_thousand_pixels() {
    let pages = [page(0), page(1), page(2)];
    // 1. 【论坛长截图】【局部更新矩阵】同一长页面仅改变右侧头像和字段，所有行都随正文滚动
    for threshold in [10000, 15000] {
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        for (index, origin) in (0..=LAST_ORIGIN).step_by(200).enumerate() {
            let phase = if origin < threshold { 0 } else { 1 + index % 2 };
            let result = stitcher.push(viewport(&pages[phase], origin)).unwrap();
            assert_eq!(
                result.status,
                if origin == 0 {
                    Status::FirstFrame
                } else {
                    Status::Appended
                },
                "threshold={threshold}, origin={origin}, result={result:?}"
            );
            assert_eq!(
                (result.position, result.added),
                (origin, if origin == 0 { HEIGHT } else { 200 })
            );
        }
        // 2. 【论坛长截图】【完整正文】逐像素检查整个长图，不能用追加状态替代正确位置和原始内容
        check_body(&pages[0], stitcher.image_view().unwrap());
    }
}

/// 【论坛长截图】【采集会话】无参数和返回值，生产重复检测、失败提示及恢复流程不能阻碍真实列表继续拼接
#[test]
fn browser_forum_session_recovers_changed_rows_after_a_bad_capture() {
    let pages = [page(0), page(1), page(2)];
    let mut session =
        ScrollSession::new(Rect::from_xywh(0.0, 0.0, 1000.0, HEIGHT as f32).unwrap(), 0).unwrap();
    // 1. 【论坛长截图】【生产捕获】通过会话入口累积长图并注入没有正文的异常过渡帧
    for origin in (0..=LAST_ORIGIN).step_by(200) {
        if origin == 15000 {
            let revision = session.stitcher.revision();
            let bad = Image::from_pixels(1000, HEIGHT, vec![0xff90_a0b0; 1000 * HEIGHT as usize])
                .unwrap();
            assert!(!session.push(bad));
            assert_eq!(session.stitcher.revision(), revision);
        }
        let phase = if origin < 10000 {
            0
        } else {
            1 + (origin / 200) as usize % 2
        };
        assert!(
            session.push(viewport(&pages[phase], origin)),
            "origin={origin}, status={}",
            session.status
        );
        assert_eq!(session.capture_range, Some((origin, HEIGHT)));
    }
    // 2. 【论坛长截图】【恢复后交付】暂停和预览仍保留完整原图，稳定正文与浏览器参考一致
    session.set_paused(true);
    assert!(session.preview().is_some());
    check_body(&pages[0], session.source().unwrap());
}
