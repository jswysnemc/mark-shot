//! 【标注多选】【状态衔接】选择采用稳定编号，单项索引在调用时由当前图层查询

use crate::{
    editor::{EditorState, dirty::damage_annotation, history_snapshot::HistorySnapshot},
    features::annotation_collection::selection::Selection,
    types::SelectionHandle,
};
use tiny_skia::Rect;

#[derive(Default)]
pub(crate) struct State {
    pub selection: Selection,
    pub marquee: Option<Marquee>,
    pub drag: Option<Drag>,
    pub revision: u64,
}

pub(crate) struct Marquee {
    pub start: (f32, f32),
    pub bounds: Option<Rect>,
}

pub(crate) struct Drag {
    pub before: HistorySnapshot,
    pub bounds: Rect,
    pub start: (f64, f64),
    pub handle: SelectionHandle,
    pub updated: bool,
}

impl EditorState {
    /// 【标注多选】【有序编号】返回所选编号切片，调用者不保存图层下标，无参数
    pub fn selected_annotation_ids(&self) -> &[u64] {
        self.collection.selection.ids()
    }

    /// 【标注多选】【单项索引】返回单项选择在当前图层的索引，多选、空选择及已移出的草稿返回空值，无参数
    pub fn selected_annotation_index(&self) -> Option<usize> {
        let id = self.collection.selection.single()?;
        self.annotations
            .iter()
            .position(|annotation| annotation.id == id)
    }

    /// 【标注多选】【完整选择】按ids设置有效有序集合，记录旧新控制框损坏，返回是否发生选择变化
    pub fn select_annotation_ids(&mut self, ids: impl IntoIterator<Item = u64>) -> bool {
        let previous = self.collection.selection.clone();
        let mut next = previous.clone();
        if !next.replace(ids, |id| {
            self.annotations
                .iter()
                .any(|annotation| annotation.id == id)
        }) {
            return false;
        }
        self.commit_text_size_wheel();
        self.collection.selection = next;
        self.collection.revision = self.collection.revision.wrapping_add(1);
        self.line_session.clear();
        // 1. 【标注多选】【旧新损坏】成员正文与控制点分别登记，联合区域覆盖组框和旋转点
        let mut area: Option<Rect> = None;
        for annotation in &self.annotations {
            if previous.ids().contains(&annotation.id)
                || self.collection.selection.ids().contains(&annotation.id)
            {
                damage_annotation(
                    &mut self.damage_rects,
                    &mut self.layer_damage_rects,
                    annotation,
                );
                let bounds = annotation.damage_bbox(true);
                area = Some(area.and_then(|area| area.join(&bounds)).unwrap_or(bounds));
            }
        }
        if let Some(area) = area.and_then(|area| area.outset(32.0, 32.0)) {
            self.damage_rects
                .push(crate::editor::DamageZone::Global(area));
        }
        true
    }

    /// 【标注多选】【单项兼容】按index选择当前图层中的对象，空值或无效下标清除整个集合，返回值为空
    pub fn select_annotation_index(&mut self, index: Option<usize>) {
        let id = index
            .and_then(|index| self.annotations.get(index))
            .map(|annotation| annotation.id);
        self.select_annotation_ids(id);
    }

    /// 【标注多选】【退出单项】返回原单项索引并清除整个选择集合，多项选择同样清除，无参数
    pub fn take_selected_annotation(&mut self) -> Option<usize> {
        let index = self.selected_annotation_index();
        self.select_annotation_ids([]);
        index
    }
}
