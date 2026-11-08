use crate::scroll::frame;
use shot_image::Image;

// test frame with coordinate-encoded pixels for verification
fn monitor_frame(width: u32, height: u32, stride: u32) -> crate::types::MonitorFrame {
    let mut pixels = vec![0u8; stride as usize * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let off = y * stride as usize + x * 4;
            pixels[off] = x as u8;
            pixels[off + 1] = y as u8;
            pixels[off + 2] = 7;
            pixels[off + 3] = 255;
        }
    }
    crate::types::MonitorFrame {
        output: 0,
        pixels,
        pw_width: width,
        pw_height: height,
        pw_stride: stride,
        info: crate::types::StreamInfo {
            node_id: 0,
            size: Some((width as i32, height as i32)),
            position: Some((0, 0)),
        },
    }
}

#[test]
fn crop_takes_the_requested_pixels() {
    let frame = monitor_frame(8, 6, 32);
    let image = frame::crop(&frame, (2, 1, 3, 2)).unwrap();
    assert_eq!((image.width(), image.height()), (3, 2));
    // top-left matches original (2, 1) in 0xAARRGGBB format: R=x, G=y, B=7
    assert_eq!(image.row(0)[0], 0xff02_0107);
    assert_eq!(image.row(1)[2], 0xff04_0207);
}

#[test]
fn crop_respects_a_padded_stride() {
    let frame = monitor_frame(4, 3, 24);
    let image = frame::crop(&frame, (1, 1, 2, 1)).unwrap();
    assert_eq!(image.row(0)[0], 0xff01_0107);
    assert_eq!(image.row(0)[1], 0xff02_0107);
}

#[test]
fn crop_rejects_regions_outside_the_frame() {
    let frame = monitor_frame(4, 3, 16);
    assert!(frame::crop(&frame, (0, 0, 5, 1)).is_err());
    assert!(frame::crop(&frame, (0, 2, 1, 2)).is_err());
    assert!(frame::crop(&frame, (0, 0, 0, 0)).is_err());
}

#[test]
fn duplicate_detection_ignores_tiny_noise() {
    let base = vec![10u8, 20, 30, 40];
    assert!(frame::is_duplicate(&base, &[10, 21, 30, 40]));
    assert!(!frame::is_duplicate(&base, &[10, 40, 30, 40]));
    assert!(!frame::is_duplicate(&[], &base));
}

#[test]
fn signature_is_stable_for_the_same_image() {
    let image = Image::from_pixels(4, 4, vec![0xff00_0000; 16]).unwrap();
    assert_eq!(
        frame::signature(&image, 4, 4),
        frame::signature(&image, 4, 4)
    );
}

/// 【滚动截图测试】【采样等价】无参数，验证原帧签名与裁切后签名在填充行距和小选区中逐字节相同，无返回值
#[test]
fn capture_signature_matches_cropped_image_without_pixel_copy() {
    let capture = monitor_frame(310, 220, 310 * 4 + 16);
    for rect in [(0, 0, 310, 220), (21, 19, 211, 177), (3, 7, 1, 1)] {
        let image = frame::crop(&capture, rect).unwrap();
        for (columns, rows) in [(18, 24), (0, 0), (33, 31)] {
            assert_eq!(
                frame::capture_signature(&capture, rect, columns, rows).unwrap(),
                frame::signature(&image, columns, rows)
            );
        }
    }
    assert!(frame::capture_signature(&capture, (u32::MAX, 0, 2, 1), 18, 24).is_err());
}

/// 【滚动截图测试】【重复捕获】无参数，验证生产捕获入口只在首帧建立像素，重复帧保持修订和原图不变，无返回值
#[test]
fn repeated_capture_does_not_grow_or_replace_the_stitched_image() {
    let mut session = crate::scroll::session::ScrollSession::new(
        tiny_skia::Rect::from_xywh(0.0, 0.0, 310.0, 220.0).unwrap(),
        0,
    )
    .unwrap();
    assert!(
        session
            .push_capture(monitor_frame(310, 220, 1240), (0, 0, 310, 220))
            .unwrap()
    );
    let revision = session.stitcher.revision();
    let storage = session.stitcher.pixel_storage();
    assert!(
        !session
            .push_capture(monitor_frame(310, 220, 1240), (0, 0, 310, 220))
            .unwrap()
    );
    assert_eq!(session.stitcher.revision(), revision);
    assert_eq!(session.stitcher.pixel_storage(), storage);
    assert_eq!(session.frame_count, 1);
}
