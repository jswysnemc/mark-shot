//! 【标注多选】【共同手势】框选与群组变换使用独立事务，持久成员保持原始图层顺序

use super::state::{Drag, Marquee};
use crate::{
    editor::{DamageZone, EditorState, dirty::damage_annotation},
    features::annotation_collection::{geometry, transform},
    tools::drawing_space::DrawingSpace,
    types::SelectionHandle,
};
use tiny_skia::Rect;

impl EditorState {
    /// 【标注多选】【联合范围】返回当前有效成员可见范围，无参数，无绘图空间时返回空值
    pub(crate) fn collection_bounds(&self) -> Option<Rect> {
        geometry::selected_bounds(
            &self.annotations,
            self.selected_annotation_ids(),
            DrawingSpace::from_editor(self)?.bounds,
        )
    }

    /// 【标注多选】【开始框选】按当前指针清除旧选择并创建空框，不保存撤销历史，无参数及返回值
    pub(crate) fn begin_collection_marquee(&mut self) {
        let Some(start) = DrawingSpace::from_editor(self)
            .and_then(|space| space.clamp_point(self.canvas_point()))
        else {
            return;
        };
        self.select_annotation_ids([]);
        self.text.editing = None;
        self.ann_drag = None;
        self.collection.marquee = Some(Marquee {
            start,
            bounds: None,
        });
    }

    /// 【标注多选】【更新框选】以global更新归一化选择框，登记旧新外框损坏，无返回值
    pub(crate) fn update_collection_marquee(&mut self, global: (f64, f64)) {
        let Some(point) =
            DrawingSpace::from_editor(self).and_then(|space| space.clamp_point(global))
        else {
            return;
        };
        let Some(marquee) = self.collection.marquee.as_mut() else {
            return;
        };
        let previous = marquee.bounds;
        let current = Rect::from_ltrb(
            marquee.start.0.min(point.0),
            marquee.start.1.min(point.1),
            marquee.start.0.max(point.0),
            marquee.start.1.max(point.1),
        );
        if previous == current {
            return;
        }
        marquee.bounds = current;
        for bounds in previous.into_iter().chain(current) {
            if let Some(bounds) = bounds.outset(3.0, 3.0) {
                self.damage_rects.push(DamageZone::Global(bounds));
            }
        }
    }

    /// 【标注多选】【提交框选】按当前框与原图像素比例选择相交成员，返回是否结束框选，无参数
    pub(crate) fn finish_collection_marquee(&mut self) -> bool {
        let Some(marquee) = self.collection.marquee.take() else {
            return false;
        };
        let ids = marquee
            .bounds
            .zip(DrawingSpace::from_editor(self))
            .map_or_else(Vec::new, |(bounds, space)| {
                self.damage_rects.push(DamageZone::Global(
                    bounds.outset(3.0, 3.0).unwrap_or(bounds),
                ));
                geometry::in_rect(&self.annotations, bounds, space.bounds, space.pixel_scale)
            });
        self.select_annotation_ids(ids);
        true
    }

    /// 【标注多选】【交互结束】撤去state临时框选并结束群组拖动，保留已更新几何，返回是否存在手势，无参数
    pub(crate) fn end_collection_interaction(&mut self) -> bool {
        let marquee = self.collection.marquee.take();
        if let Some(bounds) = marquee
            .as_ref()
            .and_then(|marquee| marquee.bounds)
            .and_then(|bounds| bounds.outset(3.0, 3.0))
        {
            self.damage_rects.push(DamageZone::Global(bounds));
        }
        self.finish_collection_drag() || marquee.is_some()
    }

    /// 【标注多选】【开始变换】按handle保存全部成员快照、联合边界与当前指针，仅多项选择生效，无返回值
    pub(crate) fn begin_collection_drag(&mut self, handle: SelectionHandle) {
        if self.selected_annotation_ids().len() < 2 || handle == SelectionHandle::None {
            return;
        }
        let Some(bounds) = self.collection_bounds() else {
            return;
        };
        self.commit_text_size_wheel();
        self.collection.drag = Some(Drag {
            before: self.snapshot(),
            bounds,
            start: self.canvas_point(),
            handle,
            updated: false,
        });
    }

    /// 【标注多选】【损坏登记】记录所选成员正文与共同控制框旧范围，无参数及返回值，不强制重建整个缓存图层
    pub(crate) fn damage_collection(&mut self) {
        if let Some(bounds) = self
            .collection_bounds()
            .and_then(|bounds| bounds.outset(34.0, 34.0))
        {
            self.damage_rects.push(DamageZone::Global(bounds));
        }
        for annotation in &self.annotations {
            if self.collection.selection.ids().contains(&annotation.id) {
                damage_annotation(
                    &mut self.damage_rects,
                    &mut self.layer_damage_rects,
                    annotation,
                );
            }
        }
    }

    /// 【标注多选】【更新变换】从原始快照计算global位置的共同变换，首次更新保存一次历史，无返回值
    pub(crate) fn update_collection_drag(&mut self, global: (f64, f64)) {
        let Some(space) = DrawingSpace::from_editor(self) else {
            return;
        };
        let Some(mut drag) = self.collection.drag.take() else {
            return;
        };
        // 1. 【标注多选】【首次历史】原版首次更新即捕获，零位移或无可靠旋转方向也保留一次历史
        if !drag.updated {
            self.undo_stack.push(drag.before.clone());
            self.redo_stack.clear();
            drag.updated = true;
        }
        self.damage_collection();
        for before in &drag.before.annotations {
            if !drag.before.selected_ids.contains(&before.id) {
                continue;
            }
            let Some(index) = self
                .annotations
                .iter()
                .position(|annotation| annotation.id == before.id)
            else {
                continue;
            };
            let next = transform::dragged(
                before,
                drag.bounds,
                drag.handle,
                drag.start,
                global,
                space.bounds,
                space.pixel_scale,
                self.input.ctrl,
                self.input.shift,
                &mut self.text,
            );
            self.annotations[index] = next;
        }
        self.damage_collection();
        self.collection.drag = Some(drag);
    }

    /// 【标注多选】【结束变换】结束当前共同手势，缓存更新使用已登记损坏范围，返回是否存在手势，无参数
    pub(crate) fn finish_collection_drag(&mut self) -> bool {
        let Some(drag) = self.collection.drag.take() else {
            return false;
        };
        for annotation in &drag.before.annotations {
            if drag.before.selected_ids.contains(&annotation.id) {
                damage_annotation(
                    &mut self.damage_rects,
                    &mut self.layer_damage_rects,
                    annotation,
                );
            }
        }
        self.damage_collection();
        true
    }

    /// 【标注多选】【整组删除】删除state全部选中成员，只形成一次历史，dirty返回旧正文及控制框损坏，无返回值
    pub(crate) fn delete_collection(&mut self, dirty: &mut u32) {
        if self.collection.drag.is_some() || self.selected_annotation_ids().is_empty() {
            return;
        }
        self.push_undo();
        self.damage_collection();
        let ids = self.selected_annotation_ids().to_vec();
        self.select_annotation_ids([]);
        self.annotations
            .retain(|annotation| !ids.contains(&annotation.id));
        self.filters.retain_live(&self.annotations, None);
        self.text.editing = None;
        crate::editor::dirty::apply_damage_rects(self, dirty);
    }
}
