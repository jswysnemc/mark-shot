//! 【X11窗口缩放测试】【真实尺寸】设置窗口契约在分数 DPI 下保持逻辑输入和物理绘制一致

use super::*;
use std::sync::{Arc, Mutex};

struct Content(Arc<Mutex<Vec<(u32, u32, f32)>>>);

impl WindowContent for Content {
    /// 【X11窗口缩放测试】【绘制记录】按 canvas 与 scale 记录真实表面尺寸并绘制样本，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        self.0
            .lock()
            .unwrap()
            .push((canvas.width(), canvas.height(), scale));
        canvas.fill(tiny_skia::Color::WHITE);
    }

    /// 【X11窗口缩放测试】【逻辑输入】按 event 检查尺寸通知采用逻辑单位，返回不修改窗口的动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        if let WindowEvent::Resize { size } = event {
            assert_eq!(size, (200, 100));
        }
        WindowAction::None
    }
}

/// 【X11窗口缩放测试】【实际表面】无参数，在私有显示验证一百二十 DPI 下的窗口、最小尺寸与绘制比例，无返回值
#[test]
#[ignore = "requires MARK_SHOT_TEST_X11_DISPLAY on an isolated X11 server"]
fn fractional_density_scales_settings_surface_and_minimum_size() {
    let name = std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap();
    assert_eq!(std::env::var("DISPLAY").unwrap(), name);
    let mut display = Display::connect().unwrap();
    display.ui_scale = 1.25;
    let report = Arc::new(Mutex::new(Vec::new()));
    let wake = task_wake::Source::new().unwrap();
    let mut node = Node::new(
        display.clone(),
        WindowSpec {
            title: "Density test".into(),
            app_id: "mark-shot-test".into(),
            size: (200, 100),
            minimum: (100, 50),
        },
        Box::new(Content(report.clone())),
        &wake,
    )
    .unwrap();
    node.sync(&wake).unwrap();
    assert_eq!(node.surface.size, (250, 125));
    assert_eq!(*report.lock().unwrap(), [(250, 125, 1.25)]);
    let hints = display
        .property(node.surface.window, "WM_NORMAL_HINTS")
        .unwrap();
    assert_eq!((hints[5], hints[6]), (125, 63));
}
