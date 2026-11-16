//! 【X11界面】【分数缩放】从桌面字体 DPI 获取逻辑比例，捕获来源始终使用原生物理像素

use super::connection::Display;
use x11rb::protocol::xproto::*;

/// 【X11界面】【资源比例】按 resources 解析 Xft.dpi，返回有效的分数比例或空值，不采用服务器虚拟毫米尺寸
fn resource_scale(resources: &str) -> Option<f32> {
    resources.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim().eq_ignore_ascii_case("Xft.dpi") {
            value
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|dpi| dpi.is_finite() && *dpi >= 48.0 && *dpi <= 768.0)
                .map(|dpi| normalize(dpi / 96.0))
        } else {
            None
        }
    })
}

/// 【X11界面】【桌面比例】按 display 读取桌面资源，缺少资源时使用有效的 GDK 比例或一倍，返回逻辑比例
pub(super) fn read(display: &Display) -> f32 {
    let query = || -> crate::AppResult<Option<f32>> {
        let resources = display
            .connection
            .get_property(
                false,
                display.screen().root,
                display.atom("RESOURCE_MANAGER")?,
                AtomEnum::STRING,
                0,
                16_384,
            )?
            .reply()?;
        Ok(resource_scale(&String::from_utf8_lossy(&resources.value)))
    };
    query()
        .ok()
        .flatten()
        .or_else(|| {
            let base = std::env::var("GDK_SCALE").ok()?.parse::<f32>().ok()?;
            let font = std::env::var("GDK_DPI_SCALE")
                .ok()
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(1.0);
            let scale = base * font;
            (scale.is_finite() && (0.5..=8.0).contains(&scale)).then(|| normalize(scale))
        })
        .unwrap_or(1.0)
}

/// 【X11界面】【比例统一】按 scale 返回一百二十分之一精度，保证输入、输出描述与截图绘制采用相同比例
fn normalize(scale: f32) -> f32 {
    (scale * 120.0).round() / 120.0
}

/// 【X11界面】【逻辑尺寸】按 size 与 scale 将物理尺寸换为逻辑尺寸，返回非空尺寸
pub(super) fn logical(size: (u32, u32), scale: f32) -> (u32, u32) {
    (
        (size.0 as f32 / scale).round().max(1.0) as u32,
        (size.1 as f32 / scale).round().max(1.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【X11缩放测试】【桌面资源】无参数，检查非整数 DPI、无效数据与源像素尺寸换算，无返回值
    #[test]
    fn fractional_desktop_dpi_keeps_exact_logical_geometry() {
        assert_eq!(
            resource_scale("Xcursor.size: 24\nXft.dpi:\t120\n"),
            Some(1.25)
        );
        assert_eq!(resource_scale("Xft.dpi: 144"), Some(1.5));
        assert_eq!(resource_scale("Xft.dpi: 110"), Some(1.15));
        assert_eq!(logical((2560, 1440), 1.25), (2048, 1152));
        for value in ["Xft.dpi: NaN", "Xft.dpi: -96", "Xft.dpi: 0", "Other: 120"] {
            assert_eq!(resource_scale(value), None);
        }
    }

    /// 【X11缩放测试】【原生捕获】无参数，在私有服务器设置一百二十 DPI，要求逻辑布局缩放且源像素不缩小，无返回值
    #[test]
    #[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated X11 server"]
    fn fractional_density_scales_layout_without_resampling_capture() {
        use crate::backend::CaptureMethod;
        use x11rb::wrapper::ConnectionExt;
        let name = std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap();
        assert_eq!(std::env::var("DISPLAY").unwrap(), name);
        let original = Display::connect().unwrap();
        let atom = original.atom("RESOURCE_MANAGER").unwrap();
        let size = (
            u32::from(original.screen().width_in_pixels),
            u32::from(original.screen().height_in_pixels),
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        for (dpi, scale) in [(120, 1.25), (168, 1.75), (110, 1.15)] {
            original
                .connection
                .change_property8(
                    PropMode::REPLACE,
                    original.screen().root,
                    atom,
                    AtomEnum::STRING,
                    format!("Xft.dpi: {dpi}\n").as_bytes(),
                )
                .unwrap()
                .check()
                .unwrap();
            let display = Display::connect().unwrap();
            assert_eq!(display.ui_scale, scale);
            let outputs = display.outputs().unwrap();
            let expected = logical(size, scale);
            assert_eq!(
                outputs[0].info.logical_size,
                Some((expected.0 as i32, expected.1 as i32))
            );
            let result = runtime
                .block_on(
                    super::super::capture::Method {
                        display: display.clone(),
                        include_cursor: false,
                    }
                    .capture_frame(&outputs),
                )
                .unwrap();
            let captures = crate::app::build_captures(result.frames).unwrap();
            assert_eq!(captures[0].scale, scale);
            assert_eq!(
                (captures[0].pixmap.width(), captures[0].pixmap.height()),
                size
            );
            // 1. 【X11缩放测试】【完整场景】真实表面接收原生网格，1366像素等非整除尺寸不能变成1367像素
            use crate::backend::ScreenOverlay;
            let placements = crate::app::build_placements(&outputs, true);
            assert_eq!(placements[0].px_size(), size);
            let mut foreground = tiny_skia::Pixmap::new(size.0, size.1).unwrap();
            let mut overlay = super::super::overlay::Overlay::new(display).unwrap();
            overlay.present(&[0]).unwrap();
            overlay
                .stage_capture_scene(
                    0,
                    crate::types::capture_scene::Frame {
                        foreground: &mut foreground,
                        background: &captures[0].pixmap,
                        damage: None,
                        selection: None,
                        hovered: None,
                        opaque: false,
                    },
                )
                .unwrap();
            overlay.flush().unwrap();
        }
        original
            .connection
            .delete_property(original.screen().root, atom)
            .unwrap()
            .check()
            .unwrap();
    }
}
