//! 【标注选框】【通用外框】普通标注和线条复用完整虚线与方块缩放点

use crate::{renderer::Painter, ui::annotation_selection::draw};
use tiny_skia::Rect;

/// 【标注选框】【普通选中框】按 bbox 绘入 p，无返回值，不为正文外框添加旧式角线留白
pub(super) fn draw_annotation_handles(p: &mut Painter<'_>, bbox: &Rect) {
    draw::rectangle(p, *bbox, 1.0);
}

/// 【标注选框】【指定外框】按 bbox 绘入 p，无返回值，调用工具已经提供自身控制框范围
pub(super) fn draw_exact_frame(p: &mut Painter<'_>, bbox: &Rect) {
    draw::rectangle(p, *bbox, 1.0);
}
