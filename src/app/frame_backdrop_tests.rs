//! 【原生背景测试】【前景保存】实际编辑器渲染后仍保留原始透明前景，不把冻结原图写回CPU画布

use crate::app::region_rendering_test_support::{editor, paint};
use tiny_skia::Rect;

/// 【原生背景测试】【提交前景】无参数，原生背景模式下提交前的CPU前景必须与编辑器帧完全相同，无返回值
#[test]
fn native_background_keeps_the_rendered_cpu_foreground() {
    let mut editor = editor(1.0, true);
    editor.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    paint(&mut editor, None, false);
    let foreground = editor.canvas[0].clone();
    assert_eq!(foreground.pixel(300, 240).unwrap().alpha(), 0);
    editor.region_rendering.opaque_background = true;
    editor.region_rendering.native_background = true;
    super::compose(&mut editor, 0, None);
    crate::app::region_rendering_test_support::equal(
        &editor.canvas[0],
        &foreground,
        "native foreground",
    );
}
