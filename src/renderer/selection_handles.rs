//! 【截图选框】【常驻把手】八个窗口尺寸固定的控制点独立于当前工具显示

use super::Painter;
use crate::{theme::Rgba, types::SelectionEdges};
use tiny_skia::{IntRect, Pixmap, PixmapPaint, Rect, Transform};

/// 【截图选框】【把手绘制】按 scale、selection 和可见 edges 绘入 canvas；仅更新 dirty 内的小矩形，无返回值
pub(super) fn draw(
    canvas: &mut Pixmap,
    scale: f32,
    dirty: Option<Rect>,
    selection: &Rect,
    edges: Option<&SelectionEdges>,
) {
    let Some(edges) = edges else { return };
    let Some(damage) = super::background_restore::damage(canvas, dirty.as_ref(), scale) else {
        return;
    };
    let center = (
        selection.left() + selection.width() / 2.0,
        selection.top() + selection.height() / 2.0,
    );
    for (horizontal, vertical, visible) in [
        (selection.left(), selection.top(), edges.left && edges.top),
        (center.0, selection.top(), edges.top),
        (selection.right(), selection.top(), edges.right && edges.top),
        (selection.left(), center.1, edges.left),
        (selection.right(), center.1, edges.right),
        (
            selection.left(),
            selection.bottom(),
            edges.left && edges.bottom,
        ),
        (center.0, selection.bottom(), edges.bottom),
        (
            selection.right(),
            selection.bottom(),
            edges.right && edges.bottom,
        ),
    ] {
        if !visible {
            continue;
        }
        let outer = Rect::from_xywh(horizontal - 3.5, vertical - 3.5, 7.0, 7.0).unwrap();
        let Some(bounds) = super::scale_rect(&outer, scale).and_then(|rect| rect.round_out())
        else {
            continue;
        };
        let Some(area) = bounds.intersect(&damage) else {
            continue;
        };
        // 1. 【截图选框】【固定相位】先绘制完整小把手再裁切损坏，局部抗锯齿不累积到未恢复的边缘
        let mut tile = Pixmap::new(bounds.width(), bounds.height()).unwrap();
        let mut painter = Painter::new(&mut tile)
            .with_scale(scale)
            .with_origin((bounds.x() as f32 / scale, bounds.y() as f32 / scale));
        painter.fill_rounded(outer, 1.0, Rgba(34, 181, 201, 255));
        painter.fill_rounded(
            outer.inset(1.0, 1.0).unwrap(),
            0.5,
            Rgba(255, 255, 255, 255),
        );
        let part = tile
            .clone_rect(
                IntRect::from_xywh(
                    area.x() - bounds.x(),
                    area.y() - bounds.y(),
                    area.width(),
                    area.height(),
                )
                .unwrap(),
            )
            .unwrap();
        canvas.draw_pixmap(
            area.x(),
            area.y(),
            part.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}
