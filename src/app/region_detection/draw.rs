//! 【截图区域识别】【候选绘制】原版薄荷色直角边框，独立透明填充与完整损坏范围

use crate::{
    editor::{DamageZone, EditorState},
    renderer::Painter,
};
use tiny_skia::{Color, PathBuilder, Rect, Stroke};

/// 【截图区域识别】【候选更新】按 state 与 next 更换候选，返回值通过损坏范围记录，正式选区不改变
pub(super) fn apply(state: &mut EditorState, next: Option<Rect>) {
    let old = state.region_detection.hovered;
    if old == next {
        return;
    }
    for rect in [old, next].into_iter().flatten() {
        if let Some(damage) = Rect::from_ltrb(
            rect.left() - 4.0,
            rect.top() - 4.0,
            rect.right() + 4.0,
            rect.bottom() + 4.0,
        ) {
            state.damage_rects.push(DamageZone::Global(damage));
        }
    }
    state.region_detection.hovered = next;
}

/// 【截图区域识别】【直角候选】按 painter 与 rect 绘制原版两逻辑像素边框和透明填充，无返回值
pub(crate) fn region(painter: &mut Painter<'_>, rect: Rect) {
    let path = PathBuilder::from_rect(rect);
    painter.fill_path(&path, Color::from_rgba8(94, 234, 212, 32));
    painter.stroke_path(
        &path,
        Color::from_rgba8(94, 234, 212, 255),
        &Stroke {
            width: 2.0,
            ..Stroke::default()
        },
    );
}
