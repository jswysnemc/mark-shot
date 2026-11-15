//! 【工具提示】【面板绘制】复用统一圆角面板和文字缓存，不参与按钮命中与图片导出

use super::State;
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{font, radius},
};
use tiny_skia::Rect;

/// 【工具提示】【局部绘制】将 state 中属于 monitor 的提示绘入 painter，clip 限制本帧更新范围，无返回值
pub(crate) fn draw(
    painter: &mut Painter<'_>,
    state: &mut State,
    monitor: usize,
    clip: Option<Rect>,
) {
    let Some(view) = state
        .view
        .as_mut()
        .filter(|view| view.monitor_idx == monitor)
    else {
        return;
    };
    if clip.is_some_and(|clip| !crate::utils::rects_overlap(&clip, &view.rect)) {
        return;
    }
    let theme = crate::theme::Mode::current();
    let redraw = view.theme != Some(theme);
    let rect = view.rect;
    let size = (rect.width(), rect.height());
    let text = &view.text;
    // 1. 【工具提示】【统一材质】提示采用现有提示颜色与面板边框，主题变化时重建缓存
    painter.clipped_panel(Some(rect), clip, |painter| {
        painter.panel(
            &mut view.cache,
            (rect.left(), rect.top()),
            size,
            1.0,
            redraw,
            |painter| {
                let local = Rect::from_xywh(0.0, 0.0, size.0, size.1).unwrap();
                let colors = &crate::config::get().toasts;
                painter.fill_rounded(local, radius::panel(), colors.background.get());
                painter.text(
                    local,
                    text,
                    TextStyle::new(font::label(), colors.text.get()).center(),
                );
            },
        );
    });
    view.theme = Some(theme);
}
