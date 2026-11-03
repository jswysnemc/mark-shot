use crate::editor::dirty::damage_annotation;
use crate::editor::history_snapshot::HistorySnapshot;
use crate::editor::{DamageZone, EditorState};

enum Step {
    Undo,
    Redo,
}

impl EditorState {
    /// 【编辑历史】【快照构造】复制持久标注、选中对象与下一编号；返回统一快照，不包含草稿或工具偏好
    pub fn snapshot(&self) -> HistorySnapshot {
        HistorySnapshot {
            annotations: self.annotations.clone(),
            number_sequence: self.number_sequence,
            selected_id: self.ann_drag.as_ref().map(|drag| drag.orig.id).or_else(|| {
                self.selected_annotation
                    .and_then(|index| self.annotations.get(index))
                    .map(|annotation| annotation.id)
            }),
            next_id: self.next_id,
        }
    }

    /// 【编辑历史】【事务开始】保存当前统一快照并清除重做记录；无返回值
    pub fn push_undo(&mut self) {
        self.commit_text_size_wheel();
        self.undo_stack.push(self.snapshot());
        self.redo_stack.clear();
    }

    /// 【编辑历史】【事务提交】比较 snapshot 与当前标注及编号，只有实际变化才记录撤销；无返回值
    pub fn commit_snapshot(&mut self, snapshot: Option<HistorySnapshot>) {
        if self.text_size_history.pending.is_some() {
            self.commit_text_size_wheel();
        }
        if let Some(snapshot) = snapshot
            && (self.annotations != snapshot.annotations
                || self.number_sequence != snapshot.number_sequence)
        {
            self.undo_stack.push(snapshot);
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self, dirty_mask: &mut u32) {
        super::text_session::commit(self, dirty_mask);
        if self.cancel_unfinished_edit(dirty_mask) {
            return;
        }
        self.step_history(Step::Undo, dirty_mask);
    }

    pub fn redo(&mut self, dirty_mask: &mut u32) {
        super::text_session::commit(self, dirty_mask);
        if self.pending.is_some() || self.ann_drag.is_some() {
            return;
        }
        self.step_history(Step::Redo, dirty_mask);
    }

    /// 【标注】【未完成撤销】撤去草稿或恢复拖动前快照；dirty_mask 返回重绘显示器，返回是否已经处理
    fn cancel_unfinished_edit(&mut self, dirty_mask: &mut u32) -> bool {
        if let Some(drag) = self.ann_drag.take() {
            if let Some(ann) = self.pending.take() {
                damage_annotation(&mut self.damage_rects, &mut self.layer_damage_rects, &ann);
                let insert_idx = drag.orig_index.min(self.annotations.len());
                damage_annotation(
                    &mut self.damage_rects,
                    &mut self.layer_damage_rects,
                    &drag.orig,
                );
                self.annotations.insert(insert_idx, drag.orig);
                self.selected_annotation = Some(insert_idx);
            }
            self.prev_pending = None;
            self.line_session.clear();
            // 1. 【标注】【未完成撤销】恢复拖动快照后结束输入，后续工具不继承旧按钮状态
            self.input.mouse_down = false;
            self.input.drag_start = None;
            self.input.straight = false;
            self.tool_active = false;
            self.annotations_dirty = true;
            *dirty_mask = u32::MAX;
            return true;
        }

        if let Some(ann) = self.pending.take() {
            self.filters.remove(ann.id);
            self.damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
            self.prev_pending = None;
            self.selected_annotation = None;
            self.restore_pen_segments(dirty_mask);
            self.pending_pen_baked = 0;
            self.input.mouse_down = false;
            self.tool_active = false;
            *dirty_mask = u32::MAX;
            return true;
        }

        false
    }

    fn step_history(&mut self, step: Step, dirty_mask: &mut u32) {
        self.commit_text_size_wheel();
        // Commit any uncommitted settings/color stepper or slider snapshot
        let settings_snapshot = self.settings_panel.pre_edit_snapshot.take();
        self.commit_snapshot(settings_snapshot);
        let color_snapshot = self.color_popover.pre_edit_snapshot.take();
        self.commit_snapshot(color_snapshot);

        let restored = match step {
            Step::Undo => self.undo_stack.pop(),
            Step::Redo => self.redo_stack.pop(),
        };
        let Some(restored) = restored else {
            return;
        };

        if let Some(sel_idx) = self.selected_annotation
            && let Some(ann) = self.annotations.get(sel_idx)
        {
            self.damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
        }
        Self::record_history_damage(
            &mut self.damage_rects,
            &mut self.layer_damage_rects,
            &self.annotations,
            &restored.annotations,
        );

        let previous_selected_id = self
            .selected_annotation
            .and_then(|index| self.annotations.get(index))
            .map(|annotation| annotation.id);
        let replaced = HistorySnapshot {
            annotations: std::mem::replace(&mut self.annotations, restored.annotations),
            number_sequence: std::mem::replace(&mut self.number_sequence, restored.number_sequence),
            selected_id: previous_selected_id,
            next_id: std::mem::replace(&mut self.next_id, restored.next_id),
        };
        match step {
            Step::Undo => self.redo_stack.push(replaced),
            Step::Redo => self.undo_stack.push(replaced),
        }

        self.reselect_by_id(restored.selected_id);
        self.line_session.clear();

        self.ann_drag = None;
        self.filters.retain_live(&self.annotations, None);
        self.text.editing = None;
        self.annotations_dirty = true;

        self.relayout_text_annotations();

        *dirty_mask = u32::MAX;
    }

    /// Annotations returns as copies, selection is followed by id.
    fn reselect_by_id(&mut self, selected_id: Option<u64>) {
        self.selected_annotation =
            selected_id.and_then(|id| self.annotations.iter().position(|ann| ann.id == id));

        if let Some(idx) = self.selected_annotation {
            self.damage_rects
                .push(DamageZone::Global(self.annotations[idx].damage_bbox(true)));
        }
    }

    fn relayout_text_annotations(&mut self) {
        for ann in &mut self.annotations {
            if matches!(ann.shape, crate::types::AnnotationShape::Text { .. }) {
                let editor = crate::tools::text::ensure_text_editor(
                    ann,
                    &mut self.text.editors,
                    &mut self.text.font_system,
                );
                crate::tools::text::update_text_bbox_inline(
                    ann,
                    editor,
                    &mut self.text.font_system,
                );
            }
        }
    }
}
