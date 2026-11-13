//! 【Windows长截图测试】【实际启动】通过原生窗口验证共享长截图接口

use super::super::Overlay;
use crate::{backend::ScreenOverlay, types::scroll_panel::ScrollPanelFrame};

/// 【Windows长截图测试】【局部面板】创建绑定首个输出的长截图窗口，返回覆盖层供坐标与捕获隔离验证
fn panel() -> Overlay {
    let mut overlay = Overlay::new().unwrap();
    overlay.present(&[0]).unwrap();
    overlay
        .configure_scroll_panel(
            tiny_skia::Rect::from_xywh(420.0, 120.0, 200.0, 80.0).unwrap(),
            false,
        )
        .unwrap();
    overlay.begin_scroll(0).unwrap();
    paint(&mut overlay);
    overlay.scroll.repaint_after = None;
    while overlay.queue.pop().is_some() {}
    overlay.pending.clear();
    overlay
}

/// 【Windows长截图测试】【面板提交】向overlay提交已配置区域的原生像素，无返回值
fn paint(overlay: &mut Overlay) {
    let rect = overlay.scroll.rect.unwrap();
    overlay
        .stage_scroll_panel(
            0,
            &ScrollPanelFrame {
                pixels: &[255; 200 * 80 * 4],
                size: (200, 80),
                bounds: rect.round_out().unwrap(),
                region: tiny_skia::Rect::from_xywh(100.0, 120.0, 300.0, 300.0).unwrap(),
                scale: 1.0,
                border: tiny_skia::Color::BLACK,
            },
        )
        .unwrap();
    overlay.flush().unwrap();
}

/// 【Windows长截图测试】【控制窗口启动】创建截图覆盖层后切换长截图，验证面板可交付且保持原显示器绑定
#[test]
fn selected_region_starts_scroll_panel_on_its_original_output() {
    let mut overlay = Overlay::new().unwrap();
    overlay.present(&[0]).unwrap();
    let original = overlay.surface_output(0).unwrap().native.bounds;
    let rect = tiny_skia::Rect::from_xywh(420.0, 120.0, 200.0, 80.0).unwrap();
    overlay.configure_scroll_panel(rect, false).unwrap();
    overlay.begin_scroll(0).unwrap();
    assert!(overlay.is_scroll_window());
    let pixels = vec![255; 200 * 80 * 4];
    overlay
        .stage_scroll_panel(
            0,
            &ScrollPanelFrame {
                pixels: &pixels,
                size: (200, 80),
                bounds: rect.round_out().unwrap(),
                region: tiny_skia::Rect::from_xywh(100.0, 120.0, 300.0, 300.0).unwrap(),
                scale: 1.0,
                border: tiny_skia::Color::BLACK,
            },
        )
        .unwrap();
    overlay.flush().unwrap();
    let bounds = overlay.surfaces[0].1.bounds().unwrap();
    assert_eq!(
        (bounds.left, bounds.top),
        (original.left + 420, original.top + 120)
    );
    assert_eq!((bounds.width, bounds.height), (200, 80));
    assert_eq!(overlay.surface_output(0).unwrap().native.bounds, original);
}

/// 【Windows长截图测试】【无效目标】错误显示器在切换前返回失败，不销毁既有截图窗口
#[test]
fn invalid_scroll_target_keeps_the_capture_surface() {
    let mut overlay = Overlay::new().unwrap();
    overlay.present(&[0]).unwrap();
    let original = overlay.surfaces[0].1.id();
    overlay
        .configure_scroll_panel(
            tiny_skia::Rect::from_xywh(420.0, 120.0, 200.0, 80.0).unwrap(),
            false,
        )
        .unwrap();
    assert!(overlay.begin_scroll(1).is_err());
    assert_eq!(overlay.surfaces[0].1.id(), original);
    assert!(!overlay.is_scroll_window());
}

/// 【Windows长截图测试】【实际坐标】原生客户区鼠标消息转换为原显示器坐标，供共享面板按钮命中
#[test]
fn panel_pointer_is_translated_to_the_original_output() {
    let mut overlay = panel();
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
            overlay.surfaces[0].1.handle() as _,
            windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE,
            0,
            12 | (18 << 16),
        );
    }
    assert!(matches!(
        Some(overlay.next_event(Some(std::time::Duration::ZERO)).unwrap()),
        Some(crate::types::OverlayEvent::PointerMove {
            monitor_idx: 0,
            x: 432.0,
            y: 138.0,
            ..
        })
    ));
}

/// 【Windows长截图测试】【污染隔离】面板重叠选区时隐藏并拒绝在途帧，干净帧后才允许暂停界面恢复
#[test]
fn overlapping_panel_rejects_stale_frames_before_restoring_controls() {
    let mut overlay = panel();
    assert!(
        overlay
            .allow_scroll(tiny_skia::Rect::from_xywh(100.0, 120.0, 300.0, 300.0).unwrap())
            .unwrap()
    );
    let region = tiny_skia::Rect::from_xywh(400.0, 100.0, 300.0, 300.0).unwrap();
    assert!(!overlay.allow_scroll(region).unwrap());
    assert!(overlay.scroll_controls_hidden());
    let hwnd = overlay.surfaces[0].1.handle() as _;
    assert_eq!(
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) },
        0
    );
    overlay.scroll.repaint_after = Some(std::time::Instant::now());
    assert!(!overlay.allow_scroll(region).unwrap());
    assert!(overlay.allow_scroll(region).unwrap());
    overlay
        .configure_scroll_panel(overlay.scroll.rect.unwrap(), true)
        .unwrap();
    paint(&mut overlay);
    assert!(!overlay.scroll_controls_hidden());
    assert_ne!(
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) },
        0
    );
}

/// 【Windows长截图测试】【暂停恢复】保存对话框隐藏后恢复原面板，窗口身份及局部尺寸保持一致
#[test]
fn suspension_preserves_the_scroll_panel_identity_and_size() {
    let mut overlay = panel();
    let original = overlay.surfaces[0].1.id();
    overlay.suspend().unwrap();
    overlay.resume().unwrap();
    paint(&mut overlay);
    assert!(overlay.is_scroll_window());
    assert_eq!(overlay.surfaces[0].1.id(), original);
    let bounds = overlay.surfaces[0].1.bounds().unwrap();
    assert_eq!((bounds.width, bounds.height), (200, 80));
}
