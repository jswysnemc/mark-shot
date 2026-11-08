//! 【滚动截图回归】【浏览器像素】真实 CSS 阴影和文字通过生产拼接器后必须保持正文原像素

use shot_image::Image;
use shot_stitch::{Config, Status, Stitcher};
use std::path::Path;

/// 【滚动截图回归】【帧读取】从 path 解析截图并转换为拼接器 ARGB 像素，返回图像
fn read(path: &Path) -> Image {
    let image = image::open(path).unwrap().to_rgb8();
    let pixels = image
        .pixels()
        .map(|pixel| {
            0xff00_0000 | u32::from(pixel[0]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[2])
        })
        .collect();
    Image::from_pixels(image.width() as i32, image.height() as i32, pixels).unwrap()
}

/// 【滚动截图回归】【结果留存】按 case 保存 image 到显式指定的诊断目录，未设置环境变量时不产生文件
fn artifact(case: &str, image: &Image) {
    if let Some(root) = std::env::var_os("MARK_SHOT_SHADOW_OUTPUT") {
        let root = Path::new(&root);
        std::fs::create_dir_all(root).unwrap();
        let result = image::RgbImage::from_fn(
            image.width() as u32,
            image.height() as u32,
            |horizontal, vertical| {
                let pixel = image.row(vertical as i32)[horizontal as usize];
                image::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            },
        );
        result
            .save(root.join(format!("stitched-{case}.png")))
            .unwrap();
    }
}

/// 【滚动截图回归】【浏览器对照】无参数，比较全部正文并保留首尾浮层一次，卡片原生阴影不能改变
#[test]
fn browser_shadows_do_not_repeat_in_document_content() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scroll-shadow");
    for case in ["light", "dark", "translucent", "both"] {
        let directory = root.join(case);
        let reference = read(&directory.join("reference.png"));
        let first = read(&directory.join("frame-0.png"));
        let last = read(&directory.join("frame-288.png"));
        let mut stitcher = Stitcher::new(Config::default()).unwrap();
        let mut session = mark_shot::scroll::session::ScrollSession::new(
            tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
            0,
        )
        .unwrap();
        for origin in [0, 96, 192, 288] {
            let frame = read(&directory.join(format!("frame-{origin}.png")));
            assert!(
                session.push(frame.clone()),
                "应用层未接收 case={case}, origin={origin}"
            );
            let result = stitcher.push(frame).unwrap();
            assert_eq!(
                result.status,
                if origin == 0 {
                    Status::FirstFrame
                } else {
                    Status::Appended
                },
                "case={case}, origin={origin}"
            );
            assert_eq!(result.added, if origin == 0 { 480 } else { 96 });
        }
        let image = stitcher.full_image().unwrap().unwrap();
        assert!(session.result().unwrap() == image);
        assert!(session.preview().is_some());
        artifact(case, &image);
        assert_eq!((image.width(), image.height()), (640, 768));
        for vertical in 0..image.height() {
            let expected = if vertical >= image.height() - 96 {
                last.row(vertical - 288)
            } else if case == "both" && vertical < 96 {
                first.row(vertical)
            } else {
                reference.row(vertical)
            };
            assert!(
                image.row(vertical) == expected,
                "case={case}, row={vertical}: 阴影残留或正文发生变化"
            );
        }
        let view = stitcher.image_view().unwrap();
        assert_eq!(view.pixel(320, 400), reference.row(400)[320]);
        let owned = stitcher.take_image().unwrap();
        assert_eq!((owned.width(), owned.height()), (640, 768));
        let mut row = 0;
        owned
            .for_each_row(|pixels| {
                assert!(pixels == image.row(row), "分块交付改变了第 {row} 行");
                row += 1;
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!(row, image.height());
    }
}

/// 【滚动截图回归】【原生分数缩放】无参数，真实 Wayland 相邻帧相位变化必须保持连续增长和原重叠像素
#[test]
fn native_fractional_frames_keep_growing_without_overwriting_overlap() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scroll-shadow/fractional");
    let mut stitcher = Stitcher::new(Config::default()).unwrap();
    let mut expected = Vec::new();
    for (index, added) in [336, 58, 57].into_iter().enumerate() {
        let frame = read(&root.join(format!("frame-{index}.png")));
        let result = stitcher.push(frame.clone()).unwrap();
        assert_eq!(
            result.status,
            if index == 0 {
                Status::FirstFrame
            } else {
                Status::Appended
            },
            "frame={index}"
        );
        assert_eq!(result.added, added);
        for vertical in frame.height() - added..frame.height() {
            expected.push(frame.row(vertical).to_vec());
        }
        let actual = stitcher.full_image().unwrap().unwrap();
        assert_eq!(actual.height() as usize, expected.len());
        for (vertical, row) in expected.iter().enumerate() {
            assert!(
                actual.row(vertical as i32) == row,
                "frame={index}, row={vertical}"
            );
        }
    }
}
