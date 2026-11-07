//! 【OCR 翻译测试】【提示像素】实际译文绘制必须按几何换行并限制在提示矩形内，不把有高度误认为正确呈现

use super::super::state::Notice;
use super::*;

/// 【OCR 翻译测试】【实际长提示】按 width 创建显示过期提示的私有窗口及布局，返回目录和窗口，不发起任务
fn stale(width: f32) -> (tempfile::TempDir, ResultWindow) {
    let (root, mut window) = super::super::controller::tests::window("exit 8");
    window.layout.size = (width, 502.0);
    window.translation.state.visible = true;
    window.translation.state.notice = Some(Notice::Stale);
    window.refresh_layout();
    (root, window)
}

/// 【OCR 翻译测试】【像素计数】在 pixels 的 rect 内返回非透明像素数，参数为真实逻辑画布，不依赖截图文字识别
fn count(pixels: &tiny_skia::Pixmap, rect: Rect) -> usize {
    let mut count = 0;
    for y in rect.top() as u32..rect.bottom() as u32 {
        for x in rect.left() as u32..rect.right() as u32 {
            count += usize::from(pixels.pixel(x, y).unwrap().alpha() != 0);
        }
    }
    count
}

/// 【OCR 翻译测试】【越界回归】原版两行提示必须两行都有字形且右边界外没有文字，六宽度均不允许越界，无参数、无返回值
#[test]
fn wrapped_translation_notice_has_glyphs_on_each_line_and_no_overflow() {
    for width in [380.0, 480.0, 500.0, 619.0, 620.0, 760.0] {
        let (_root, mut window) = stale(width);
        let rect = window.layout.translation.as_ref().unwrap().notice.unwrap();
        let mut canvas = tiny_skia::Pixmap::new(width as u32 + 480, 502).unwrap();
        draw(&mut window, &mut canvas, 1.0);
        let outside = Rect::from_xywh(rect.right(), rect.top(), 450.0, rect.height()).unwrap();
        assert_eq!(
            count(&canvas, outside),
            0,
            "notice exceeded pane at {width}"
        );
        if rect.height() > 17.0 {
            for y in [rect.top(), rect.top() + 17.0] {
                assert!(
                    count(
                        &canvas,
                        Rect::from_xywh(rect.left(), y, rect.width(), 17.0).unwrap()
                    ) > 8,
                    "notice omitted wrapped row at {width}"
                );
            }
        }
    }
}
