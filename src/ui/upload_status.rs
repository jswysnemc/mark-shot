//! 【图片上传】【进度胶囊】在后台上传期间于选区旁绘制轻量进行中微提示，不改动结果通知契约

use crate::{
    editor::EditorState,
    renderer::Painter,
    ui::{
        language::Language,
        micro_badge::{self, BadgeKind},
    },
};
use tiny_skia::Rect;

/// 【图片上传】【覆盖层胶囊绘制】当 monitor 存在活跃上传任务时在选区附近绘制进度与取消提示，无返回值
pub(crate) fn draw(state: &mut EditorState, monitor: usize, dirty: Option<Rect>) {
    if state.ui_hidden || !state.upload.uploading() {
        return;
    }
    let placement = &state.placements[monitor];
    let Some(screen) = placement.rect() else {
        return;
    };
    let anchor = state
        .selection
        .zone
        .and_then(|zone| state.canvas_local_rect(zone, monitor))
        .and_then(|local| local.intersect(&screen));
    if anchor.is_none() && state.input.pointer.monitor_idx != monitor {
        return;
    }

    let label = if Language::current() == Language::Chinese {
        "正在上传图片… · Esc 取消"
    } else {
        "Uploading image... · Esc to cancel"
    };
    let (w, h) = micro_badge::measure(
        &mut state.text.font_system,
        label,
        "",
        BadgeKind::Busy,
        (screen.width() - 16.0).max(64.0),
    );
    let (x, y) = if let Some(sel) = anchor {
        let cx = sel.left() + (sel.width() - w) * 0.5;
        let below = sel.top() + (sel.height() - h) * 0.5;
        (
            cx.clamp(8.0, (screen.width() - w - 8.0).max(8.0)),
            below.clamp(8.0, (screen.height() - h - 8.0).max(8.0)),
        )
    } else {
        ((screen.width() - w) * 0.5, (screen.height() - h) * 0.5)
    };
    let Some(rect) = Rect::from_xywh(x.round(), y.round(), w, h) else {
        return;
    };
    if let Some(clip) = dirty
        && clip.intersect(&rect).is_none()
    {
        return;
    }

    let mut painter = Painter::new(&mut state.canvas[monitor])
        .with_scale(placement.scale)
        .with_text(&mut state.text.font_system, &mut state.text.swash_cache);
    painter.clipped_scene(dirty, |p| {
        micro_badge::draw(p, rect, BadgeKind::Busy, label, "", 1.0);
    });
}
