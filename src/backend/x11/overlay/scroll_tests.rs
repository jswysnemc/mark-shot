//! 【X11长图测试】【局部窗口像素】真实 Xorg 核对全屏覆盖层切换后的面板交付

use super::*;
use crate::backend::ScreenOverlay;
use x11rb::protocol::xproto::{ConnectionExt, ImageFormat};

/// 【X11长图测试】【角色切换】无参数，在明确标记的隔离服务器把已映射覆盖层缩为局部面板，服务器像素必须等于提交帧
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated X11 server"]
fn resized_scroll_panel_keeps_uploaded_server_pixels() {
    assert_eq!(
        std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap(),
        std::env::var("DISPLAY").unwrap()
    );
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_none());
    let display = Display::connect().unwrap();
    let mut overlay = Overlay::new(display.clone()).unwrap();
    overlay.present(&[0]).unwrap();
    let size = overlay.surfaces[0].1.size;
    overlay
        .stage_frame(
            0,
            &vec![255; usize::from(size.0) * usize::from(size.1) * 4],
            None,
        )
        .unwrap();
    overlay.begin_scroll(0).unwrap();
    for height in [220, 440] {
        let region = tiny_skia::Rect::from_xywh(80.0, 100.0, 320.0, 320.0).unwrap();
        let bounds = tiny_skia::IntRect::from_xywh(412, 100, 340, height).unwrap();
        let pixels = [16, 185, 129, 255].repeat((340 * height) as usize);
        overlay
            .configure_scroll_panel(bounds.to_rect(), false)
            .unwrap();
        overlay
            .stage_scroll_panel(
                0,
                &crate::types::scroll_panel::ScrollPanelFrame {
                    pixels: &pixels,
                    size: (340, height),
                    bounds,
                    region,
                    scale: 1.0,
                },
            )
            .unwrap();
        let surface = &overlay.surfaces[0].1;
        let image = display
            .connection
            .get_image(
                ImageFormat::Z_PIXMAP,
                surface.window,
                0,
                0,
                340,
                height as u16,
                u32::MAX,
            )
            .unwrap()
            .reply()
            .unwrap();
        let actual = super::super::pixels::Format::new(&display, image.depth, image.visual)
            .unwrap()
            .decode(&image.data, 340, height as usize)
            .unwrap();
        assert_eq!(&actual[..4], &pixels[..4], "height={height}");
        assert_eq!(actual, pixels);
    }
}
