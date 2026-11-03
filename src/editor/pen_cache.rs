//! 【画笔缓存】【编辑连接】恢复缓存笔迹前的原始像素，并按显示器通知重绘

use crate::editor::dirty::mark_dirty;
use crate::editor::{DamageZone, EditorState};
use tiny_skia::Rect;

impl EditorState {
    /// 【画笔缓存】【缓存恢复】清除未提交增量笔迹并精确恢复原始层；dirty_mask 返回损坏显示器，不改变标注或历史
    pub(crate) fn restore_pen_segments(&mut self, dirty_mask: &mut u32) {
        for (monitor_idx, pixels) in self.pen_backup.restore(&mut self.annotations_layer) {
            let scale = self.placements[monitor_idx].scale;
            if let Some(rect) = Rect::from_xywh(
                pixels.x() as f32 / scale,
                pixels.y() as f32 / scale,
                pixels.width() as f32 / scale,
                pixels.height() as f32 / scale,
            ) {
                self.damage_rects
                    .push(DamageZone::Local { monitor_idx, rect });
                mark_dirty(dirty_mask, monitor_idx);
            }
        }
    }
}
