//! 【画布滚动条】【样式绘制】主题轨道、细边、悬停加宽和青色拖动状态

use super::geometry::{hit, parts};
use crate::{editor::EditorState, renderer::Painter};
use tiny_skia::Stroke;

/// 【画布滚动条】【输出绘制】按state、monitor和dirty合成原生比例控件，不修改原图，无返回值
pub(crate) fn draw(state: &mut EditorState, monitor: usize, dirty: Option<tiny_skia::Rect>) {
    let Some(window) = state.navigation.window else {
        return;
    };
    let bars = state.navigation.bars;
    let point = state.navigation_point();
    let drag = state.navigation.slider;
    let offset = state.placements[monitor].offset();
    let scale = state.placements[monitor].scale;
    let mut painter = Painter::new(&mut state.canvas[monitor])
        .with_scale(scale)
        .with_origin((offset.0 - window.left(), offset.1 - window.top()));
    painter.clipped(dirty, |painter| {
        for (axis, horizontal) in [(bars.horizontal, true), (bars.vertical, false)] {
            let pressed = drag.is_some_and(|drag| drag.horizontal == horizontal);
            let hover = hit(axis, point) || pressed;
            let Some((track, thumb)) = parts(axis, horizontal, hover) else {
                continue;
            };
            let colors = super::palette::colors(
                crate::theme::Mode::current(),
                pressed,
                thumb.contains(point) && hover,
                hover,
            );
            if hover && let Some(rect) = track.raster() {
                painter.fill_rounded(rect, 5.0, colors.track.color());
            }
            let Some(rect) = thumb.raster() else { continue };
            let radius = if hover { 4.0 } else { 3.0 };
            painter.fill_rounded(rect, radius, colors.fill.color());
            if let Some(path) = crate::renderer::rounded_rect_path(
                &rect.inset(0.5, 0.5).unwrap_or(rect),
                radius,
                true,
                true,
                true,
                true,
            ) {
                painter.stroke_path(
                    &path,
                    colors.border.color(),
                    &Stroke {
                        width: 1.0,
                        ..Default::default()
                    },
                );
            }
        }
    });
}
