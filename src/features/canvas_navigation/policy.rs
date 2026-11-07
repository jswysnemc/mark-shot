//! 【画布导航】【输入职责】原版编辑阶段开放导航，选中标注粗细优先于图片缩放

use crate::{tools::Tool, types::ScrollEvent};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    pub image_enabled: bool,
    pub editing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelTarget {
    None,
    ImageZoom,
    AnnotationWidth,
}

impl Policy {
    /// 【画布导航】【可用判断】返回当前是否启用图片导航或已经进入截图编辑阶段，无参数
    pub fn available(self) -> bool {
        self.image_enabled || self.editing
    }

    /// 【画布导航】【工具缩放】按tool返回是否优先把无选中成员的滚轮用于缩放，图片编辑对所有工具开放
    pub fn wheel_zooms(self, tool: Tool) -> bool {
        self.image_enabled || self.editing && tool == Tool::Pick
    }

    /// 【画布导航】【滚轮目标】按tool、selected_count、editing_existing_text、blocked与event返回缩放、粗细或忽略，选中成员始终优先
    pub fn wheel_target(
        self,
        tool: Tool,
        selected_count: usize,
        editing_existing_text: bool,
        blocked: bool,
        event: ScrollEvent,
    ) -> WheelTarget {
        if blocked || !self.editing || event.canvas_steps().abs() <= 1e-12 {
            return WheelTarget::None;
        }
        let selected_width = (matches!(tool, Tool::Pick | Tool::Ruler) && selected_count > 0)
            || editing_existing_text;
        if !selected_width && self.wheel_zooms(tool) {
            WheelTarget::ImageZoom
        } else {
            WheelTarget::AnnotationWidth
        }
    }
}

/// 【画布导航】【滚轮倍率】按event返回原版每刻度1.12倍的连续倍率，显式纯像素使用八十单位一刻度
pub fn wheel_factor(event: ScrollEvent) -> f64 {
    1.12_f64.powf(event.canvas_steps())
}
