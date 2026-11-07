//! 【标注选框】【文字外框】选择工具下的文字沿用通用控制框，正文编辑框另行绘制

use crate::{renderer::Painter, ui::annotation_selection::draw};
use tiny_skia::Rect;

/// 【标注选框】【文字控制框】按 bbox 把完整虚线和缩放点绘入 p，无返回值，不改变正文尺寸
pub(super) fn draw_text_box(p: &mut Painter<'_>, bbox: &Rect) {
    draw::rectangle(p, *bbox, 1.0);
}
