//! 【画布导航】【滚动条状态】使用原版整数范围与页长，靠近边缘或拖动时才显示

use super::{
    coordinates::{Point, Rect},
    model::Navigation,
};

pub const EXTENT: i32 = 14;
pub const REVEAL_DISTANCE: i32 = EXTENT + 18;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Axis {
    pub rect: Rect,
    pub maximum: i32,
    pub page_step: i32,
    pub single_step: i32,
    pub value: i32,
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scrollbars {
    pub horizontal: Axis,
    pub vertical: Axis,
}

impl Scrollbars {
    /// 【画布导航】【滚动条同步】按view、available、pointer与dragging更新原生几何和值，返回实际变化；停用时隐藏但保留范围
    pub fn update(
        &mut self,
        view: Navigation,
        available: bool,
        pointer: Point,
        dragging: (bool, bool),
    ) -> bool {
        let before = *self;
        if !available {
            self.horizontal.visible = false;
            self.vertical.visible = false;
            return *self != before;
        }
        let rect = view.projection().widget_rect;
        let window = view.window_size();
        let width = i64::from(window.0);
        let height = i64::from(window.1);
        let max_x = (rect.width.round() as i64 - width)
            .max(0)
            .min(i64::from(i32::MAX)) as i32;
        let max_y = (rect.height.round() as i64 - height)
            .max(0)
            .min(i64::from(i32::MAX)) as i32;
        let pointer = (pointer.0.round(), pointer.1.round());
        let inside = pointer.0 >= 0.0
            && pointer.1 >= 0.0
            && pointer.0 < width as f64
            && pointer.1 < height as f64;
        // 1. 【画布导航】【相邻避让】两轴内容范围独立计算，只在几何长度中扣除另一条滚动条的十四像素
        self.horizontal = Axis {
            rect: Rect {
                x: 0.0,
                y: height as f64 - f64::from(EXTENT),
                width: (width - if max_y > 0 { i64::from(EXTENT) } else { 0 }).max(0) as f64,
                height: f64::from(EXTENT),
            },
            maximum: max_x,
            page_step: width.max(1).min(i64::from(i32::MAX)) as i32,
            single_step: (width / 12).max(1).min(i64::from(i32::MAX)) as i32,
            value: (-rect.x).round().clamp(0.0, f64::from(max_x)) as i32,
            visible: max_x > 0
                && (inside && pointer.1.round() >= height as f64 - f64::from(REVEAL_DISTANCE)
                    || dragging.0),
        };
        self.vertical = Axis {
            rect: Rect {
                x: width as f64 - f64::from(EXTENT),
                y: 0.0,
                width: f64::from(EXTENT),
                height: (height - if max_x > 0 { i64::from(EXTENT) } else { 0 }).max(0) as f64,
            },
            maximum: max_y,
            page_step: height.max(1).min(i64::from(i32::MAX)) as i32,
            single_step: (height / 12).max(1).min(i64::from(i32::MAX)) as i32,
            value: (-rect.y).round().clamp(0.0, f64::from(max_y)) as i32,
            visible: max_y > 0
                && (inside && pointer.0.round() >= width as f64 - f64::from(REVEAL_DISTANCE)
                    || dragging.1),
        };
        *self != before
    }

    /// 【画布导航】【中心应用】按values与available把限制后的滚动值用于view，返回视图是否变化，不修改标注历史
    pub fn apply_values(self, view: &mut Navigation, values: (i32, i32), available: bool) -> bool {
        view.center_from_scrollbars(
            (
                values.0.clamp(0, self.horizontal.maximum),
                values.1.clamp(0, self.vertical.maximum),
            ),
            (self.horizontal.maximum > 0, self.vertical.maximum > 0),
            available,
        )
    }
}
