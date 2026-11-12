//! 【X11呈现测试】【真实服务端】独立显示器验证局部上传和曝光副本，测试不连接用户桌面

use super::*;

/// 【X11呈现测试】【服务器像素】读取 surface 的原生窗口并解码为 RGBA，返回完整帧，用于验证真实协议结果
fn read(surface: &Surface) -> Vec<u8> {
    let reply = surface
        .display
        .connection
        .get_image(
            ImageFormat::Z_PIXMAP,
            surface.window,
            0,
            0,
            surface.size.0,
            surface.size.1,
            u32::MAX,
        )
        .unwrap()
        .reply()
        .unwrap();
    pixels::Format::new(&surface.display, reply.depth, reply.visual)
        .unwrap()
        .decode(&reply.data, surface.size.0.into(), surface.size.1.into())
        .unwrap()
}

/// 【X11呈现测试】【损坏与曝光】真实小范围上传必须保留全部其他像素，曝光重绘必须恢复已提交结果
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated Xvfb"]
fn partial_upload_and_exposure_keep_unchanged_pixels() {
    let display_name =
        std::env::var("MARK_SHOT_TEST_X11_DISPLAY").expect("private display required");
    assert_eq!(std::env::var("DISPLAY").unwrap(), display_name);
    let display = Display::connect().unwrap();
    let mut surface = Surface::new(display, (0, 0), (1024, 700), true).unwrap();
    let mut image = tiny_skia::Pixmap::new(1024, 700).unwrap();
    image.fill(tiny_skia::Color::from_rgba8(73, 91, 177, 255));
    surface.present(image.data()).unwrap();
    assert_eq!(read(&surface), image.data());
    let mut changed = image.clone();
    changed.fill(tiny_skia::Color::from_rgba8(191, 117, 33, 255));
    surface
        .present_region(changed.data(), Some((31, 17, 79, 53)))
        .unwrap();
    for row in 17..70 {
        let first = (row * 1024 + 31) * 4;
        let last = first + 79 * 4;
        image.data_mut()[first..last].copy_from_slice(&changed.data()[first..last]);
    }
    assert_eq!(read(&surface), image.data());
    surface.repaint().unwrap();
    assert_eq!(read(&surface), image.data());
}
