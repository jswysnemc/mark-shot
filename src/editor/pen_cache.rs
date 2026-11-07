//! 【画笔缓存】【编辑连接】恢复缓存笔迹前的原始像素，并按显示器通知重绘

use crate::editor::dirty::mark_dirty;
use crate::editor::{DamageZone, EditorState};
use tiny_skia::{Color, IntRect, PathBuilder, Rect};

impl EditorState {
    /// 【画笔缓存】【缓存恢复】清除未提交增量笔迹并精确恢复原始层；dirty_mask 返回损坏显示器，不改变标注或历史
    pub(crate) fn restore_pen_segments(&mut self, dirty_mask: &mut u32) {
        for (monitor_idx, pixels) in self.pen_backup.restore(&mut self.annotations_layer) {
            // 1. 【画笔缓存】【恢复后归还】恢复已经完成，仅回收全零页，完整原始标注及下一次烘焙所需非零像素继续保留
            crate::utils::transparent_pixmap::reclaim_region(
                &mut self.annotations_layer[monitor_idx],
                pixels,
            );
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

    /// 【画笔缓存】【片段绘制】缓存start、control、end构成的笔迹；color、stroke_width控制样式，返回全局重绘区域
    pub fn bake_pen_segment(
        &mut self,
        start: (f32, f32),
        control: Option<(f32, f32)>,
        end: (f32, f32),
        color: Color,
        stroke_width: f32,
    ) -> Rect {
        self.ensure_annotation_cache();
        let mut pb = PathBuilder::new();
        pb.move_to(start.0, start.1);
        if let Some(ctrl) = control {
            pb.quad_to(ctrl.0, ctrl.1, end.0, end.1);
        } else {
            pb.line_to(end.0, end.1);
        }

        let pad = crate::renderer::visual_pad(stroke_width);
        let min_x = start
            .0
            .min(end.0)
            .min(control.map(|c| c.0).unwrap_or(start.0))
            - pad;
        let min_y = start
            .1
            .min(end.1)
            .min(control.map(|c| c.1).unwrap_or(start.1))
            - pad;
        let max_x = start
            .0
            .max(end.0)
            .max(control.map(|c| c.0).unwrap_or(start.0))
            + pad;
        let max_y = start
            .1
            .max(end.1)
            .max(control.map(|c| c.1).unwrap_or(start.1))
            + pad;
        let segment_bbox = Rect::from_ltrb(min_x, min_y, max_x, max_y).unwrap_or_else(|| {
            Rect::from_xywh(start.0 - pad, start.1 - pad, pad * 2.0, pad * 2.0).unwrap()
        });

        if let Some(path) = pb.finish() {
            for (i, placement) in self.placements.iter().enumerate() {
                let offset = placement.offset();
                let visual = Rect::from_ltrb(
                    segment_bbox.left() - offset.0,
                    segment_bbox.top() - offset.1,
                    segment_bbox.right() - offset.0,
                    segment_bbox.bottom() - offset.1,
                );
                if let (Some(vis), Some(mon)) = (visual, placement.rect())
                    && crate::utils::rects_overlap(&vis, &mon)
                {
                    let pixels = crate::renderer::scale_rect(&vis, placement.scale)
                        .and_then(|pixel_rect| {
                            crate::renderer::rect_bounds(
                                &pixel_rect,
                                self.annotations_layer[i].width(),
                                self.annotations_layer[i].height(),
                            )
                        })
                        .and_then(|(x, y, width, height)| {
                            IntRect::from_xywh(x as i32, y as i32, width, height)
                        });
                    // 1. 【画笔缓存】【片段绘制】先保存当前像素，提交和取消时才能精确还原
                    if let Some(rect) = pixels {
                        self.pen_backup
                            .snapshot(i, &self.annotations_layer[i], rect);
                    }
                    crate::renderer::stroke_pen_segment(
                        &mut crate::renderer::Painter::new(&mut self.annotations_layer[i])
                            .with_scale(placement.scale)
                            .with_origin(offset),
                        &path,
                        color,
                        stroke_width,
                    );
                    // 2. 【画笔缓存】【片段透明回收】保留完整笔迹和备份，只检查本次片段触及的全零页
                    if let Some(rect) = pixels {
                        crate::utils::transparent_pixmap::reclaim_region(
                            &mut self.annotations_layer[i],
                            rect,
                        );
                    }
                }
            }
        }
        segment_bbox
    }
}
