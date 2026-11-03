use crate::editor::{DamageZone, EditorState};
use crate::tools::selection::global_selection_to_local;
use crate::types::Placement;
use crate::types::annotations::Annotation;
use crate::ui::magnifier::MagnifierState;
use crate::utils::get_overlapping_monitors;

use tiny_skia::Rect;

impl EditorState {
    pub fn monitor_dirty_rect(&self, monitor_idx: usize) -> Option<Rect> {
        let placement = &self.placements[monitor_idx];
        let mut dirty: Option<Rect> = None;

        dirty = union_rect(dirty, self.calc_selection_dirty(placement));
        dirty = union_rect(dirty, self.calc_magnifier_dirty(monitor_idx, placement));
        dirty = union_rect(dirty, self.calc_damage_zones_dirty(monitor_idx, placement));

        dirty
    }

    fn calc_selection_dirty(&self, placement: &Placement) -> Option<Rect> {
        if self.selection.zone == self.selection.prev_zone {
            return None;
        }
        let mut dirty = None;
        let border_width = crate::config::get().selection.border_width;
        let selection_pad = (border_width * 2.0 + 4.0).max(8.0);

        let local_sel = self
            .selection
            .zone
            .as_ref()
            .and_then(|sel| global_selection_to_local(sel, placement));

        let prev_local = self
            .selection
            .prev_zone
            .as_ref()
            .and_then(|sel| global_selection_to_local(sel, placement));

        if let Some(r) = local_sel
            .as_ref()
            .and_then(|sel| expand_rect(sel, selection_pad))
        {
            dirty = union_rect(dirty, Some(r));
        }
        if let Some(r) = prev_local
            .as_ref()
            .and_then(|sel| expand_rect(sel, selection_pad))
        {
            dirty = union_rect(dirty, Some(r));
        }
        dirty
    }

    fn calc_magnifier_dirty(&self, monitor_idx: usize, placement: &Placement) -> Option<Rect> {
        let mut dirty = None;
        if !self.magnifier_shown() {
            return dirty;
        }
        let (mw, mh) = (placement.size.0 as f32, placement.size.1 as f32);
        // color label, when picking a color
        let with_label = self.picking();
        if mw > 0.0 && mh > 0.0 {
            let mag_pad = crate::config::get().magnifier.outline_width / 2.0 + 1.0;

            let mut add_mag_dirty = |mag_state: &Option<MagnifierState>| {
                if let Some(mag) = mag_state.as_ref().filter(|m| m.monitor_idx == monitor_idx) {
                    let rect = crate::ui::magnifier::magnifier_rect(
                        (mag.pos.0 as f32, mag.pos.1 as f32),
                        mw,
                        mh,
                        with_label,
                    );
                    if let Some(r) = expand_rect(&rect, mag_pad) {
                        dirty = union_rect(dirty, Some(r));
                    }
                }
            };

            add_mag_dirty(&self.magnifier.current);
            add_mag_dirty(&self.magnifier.prev);
        }
        dirty
    }

    fn calc_damage_zones_dirty(&self, monitor_idx: usize, placement: &Placement) -> Option<Rect> {
        let mut dirty = None;
        let offset = placement.offset();
        let (mw, mh) = (placement.size.0 as f32, placement.size.1 as f32);

        fn global_to_local_padded(
            global_bbox: &Rect,
            offset: (f32, f32),
            pad: f32,
            mw: f32,
            mh: f32,
        ) -> Option<Rect> {
            let l = global_bbox.left() - offset.0 - pad;
            let t = global_bbox.top() - offset.1 - pad;
            let r = global_bbox.right() - offset.0 + pad;
            let b = global_bbox.bottom() - offset.1 + pad;

            let ix1 = l.max(0.0);
            let iy1 = t.max(0.0);
            let ix2 = r.min(mw);
            let iy2 = b.min(mh);

            if ix2 <= ix1 || iy2 <= iy1 {
                return None;
            }

            Rect::from_ltrb(ix1, iy1, ix2, iy2)
        }

        let is_pen_drawing = matches!(
            self.selected_tool,
            crate::tools::Tool::Pen | crate::tools::Tool::Highlighter
        ) && self.ann_drag.is_none();

        if !is_pen_drawing {
            if let Some(ann) = &self.pending {
                let pad = ann.visual_pad();
                dirty = union_rect(
                    dirty,
                    global_to_local_padded(&ann.bbox, offset, pad, mw, mh),
                );
            }
            if let Some(ann) = &self.prev_pending {
                let pad = ann.visual_pad();
                dirty = union_rect(
                    dirty,
                    global_to_local_padded(&ann.bbox, offset, pad, mw, mh),
                );
            }
        }
        if self.text_session.is_none()
            && let Some(ann_idx) = self.selected_annotation
            && let Some(ann) = self.annotations.get(ann_idx)
        {
            let pad = crate::renderer::selection_chrome_pad().max(ann.visual_pad());
            dirty = union_rect(
                dirty,
                global_to_local_padded(&ann.bbox, offset, pad, mw, mh),
            );
        }

        for zone in &self.damage_rects {
            match zone {
                DamageZone::Global(rect) => {
                    dirty = union_rect(dirty, global_to_local_padded(rect, offset, 4.0, mw, mh));
                }
                DamageZone::Local {
                    monitor_idx: idx,
                    rect,
                } if *idx == monitor_idx => {
                    let local_clamped = Rect::from_ltrb(
                        (rect.left() - 4.0).max(0.0),
                        (rect.top() - 4.0).max(0.0),
                        (rect.right() + 4.0).min(mw),
                        (rect.bottom() + 4.0).min(mh),
                    );
                    dirty = union_rect(dirty, local_clamped);
                }
                DamageZone::Local { .. } => {}
            }
        }

        dirty
    }

    pub fn monitor_layer_dirty_rect(&self, monitor_idx: usize) -> Option<Rect> {
        let placement = &self.placements[monitor_idx];
        let offset = placement.offset();
        let (mw, mh) = (placement.size.0 as f32, placement.size.1 as f32);
        let mut dirty: Option<Rect> = None;

        for rect in &self.layer_damage_rects {
            let l = rect.left() - offset.0;
            let t = rect.top() - offset.1;
            let r = rect.right() - offset.0;
            let b = rect.bottom() - offset.1;

            let ix1 = l.max(0.0);
            let iy1 = t.max(0.0);
            let ix2 = r.min(mw);
            let iy2 = b.min(mh);

            if ix2 > ix1
                && iy2 > iy1
                && let Some(local_r) = Rect::from_ltrb(ix1, iy1, ix2, iy2)
            {
                dirty = union_rect(dirty, Some(local_r));
            }
        }

        dirty
    }

    pub fn record_history_damage(
        damage_rects: &mut Vec<DamageZone>,
        layer_damage_rects: &mut Vec<Rect>,
        state_a: &[Annotation],
        state_b: &[Annotation],
    ) {
        for ann_a in state_a {
            if let Some(ann_b) = state_b.iter().find(|b| b.id == ann_a.id) {
                if ann_a != ann_b {
                    damage_annotation(damage_rects, layer_damage_rects, ann_a);
                    damage_annotation(damage_rects, layer_damage_rects, ann_b);
                }
            } else {
                damage_annotation(damage_rects, layer_damage_rects, ann_a);
            }
        }

        for ann_b in state_b {
            if !state_a.iter().any(|a| a.id == ann_b.id) {
                damage_annotation(damage_rects, layer_damage_rects, ann_b);
            }
        }
    }

    pub fn damage_local(&mut self, monitor_idx: usize, rect: Rect) {
        self.damage_rects
            .push(DamageZone::Local { monitor_idx, rect });
    }
}

pub fn expand_rect(rect: &Rect, pad: f32) -> Option<Rect> {
    Rect::from_ltrb(
        rect.left() - pad,
        rect.top() - pad,
        rect.right() + pad,
        rect.bottom() + pad,
    )
}

fn union_rect(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (None, None) => None,
        (Some(r), None) | (None, Some(r)) => Some(r),
        (Some(r1), Some(r2)) => Rect::from_ltrb(
            r1.left().min(r2.left()),
            r1.top().min(r2.top()),
            r1.right().max(r2.right()),
            r1.bottom().max(r2.bottom()),
        ),
    }
}

/// marks an annotation dirty on both the main canvas and cached layer.
/// takes separate vector references to work while the annotation is borrowed.
pub fn damage_annotation(
    damage_rects: &mut Vec<DamageZone>,
    layer_damage_rects: &mut Vec<Rect>,
    ann: &Annotation,
) {
    damage_rects.push(DamageZone::Global(ann.damage_bbox(true)));
    layer_damage_rects.push(ann.damage_bbox(false));
}

pub fn mark_dirty(mask: &mut u32, idx: usize) {
    *mask |= 1 << idx;
}

pub fn mark_all_dirty(mask: &mut u32, monitors: usize) {
    for idx in 0..monitors {
        mark_dirty(mask, idx);
    }
}

pub fn is_dirty(mask: u32, idx: usize) -> bool {
    (mask & (1 << idx)) != 0
}

pub fn apply_damage_rects(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let is_pen_drawing = matches!(
        editor_state.selected_tool,
        crate::tools::Tool::Pen | crate::tools::Tool::Highlighter
    ) && editor_state.ann_drag.is_none();

    if !is_pen_drawing {
        if let Some(ann) = &editor_state.pending {
            *dirty_mask |= get_overlapping_monitors(&ann.bbox, &editor_state.placements);
        }
        if let Some(ann) = &editor_state.prev_pending {
            *dirty_mask |= get_overlapping_monitors(&ann.bbox, &editor_state.placements);
        }
    }
    for rect in &editor_state.layer_damage_rects {
        *dirty_mask |= get_overlapping_monitors(rect, &editor_state.placements);
    }
    for zone in &editor_state.damage_rects {
        match zone {
            DamageZone::Global(rect) => {
                *dirty_mask |= get_overlapping_monitors(rect, &editor_state.placements);
            }
            DamageZone::Local { monitor_idx, .. } => {
                mark_dirty(dirty_mask, *monitor_idx);
            }
        }
    }
}
