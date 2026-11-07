//! 【文字识别】【滚动条区域】原版横向 Fusion 子控件与纵向样式表滑块，命中和绘制共用几何

use super::{layout::contains, table_scroll_model::Range};
use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Slider,
    PreviousLine,
    NextLine,
    PreviousPage,
    NextPage,
}

pub(super) struct Bar {
    pub axis: Axis,
    pub bounds: Rect,
    pub groove: Rect,
    pub slider: Rect,
    pub previous_line: Option<Rect>,
    pub next_line: Option<Rect>,
    pub previous_page: Option<Rect>,
    pub next_page: Option<Rect>,
    pub maximum: usize,
}

impl Axis {
    /// 【文字识别】【方向索引】返回两方向数组中的索引，无参数，横向为零，纵向为一
    pub fn index(self) -> usize {
        match self {
            Self::Horizontal => 0,
            Self::Vertical => 1,
        }
    }

    /// 【文字识别】【指针轴值】从 position 返回本方向坐标，另一方向不参与滑块定位
    pub fn coordinate(self, position: (f64, f64)) -> f32 {
        match self {
            Self::Horizontal => position.0 as f32,
            Self::Vertical => position.1 as f32,
        }
    }

    /// 【文字识别】【区域轴值】从 rect 返回本方向起点和长度，供轨道与滑块共用计算
    pub fn geometry(self, rect: Rect) -> (f32, f32) {
        match self {
            Self::Horizontal => (rect.x(), rect.width()),
            Self::Vertical => (rect.y(), rect.height()),
        }
    }
}

/// 【文字识别】【轴向矩形】按 axis、bounds、start、length 创建同一横截面的子区域，零长度返回空值
fn strip(axis: Axis, bounds: Rect, start: f32, length: f32) -> Option<Rect> {
    if length <= 0.0 {
        return None;
    }
    match axis {
        Axis::Horizontal => Rect::from_xywh(start, bounds.y(), length, bounds.height()),
        Axis::Vertical => Rect::from_xywh(bounds.x(), start, bounds.width(), length),
    }
}

impl Bar {
    /// 【文字识别】【滚动条布局】按 axis、bounds、range、value 返回原版真实子区域；横向保留箭头，纵向使用两像素边距且没有箭头
    pub fn new(axis: Axis, bounds: Rect, range: &Range, value: usize) -> Self {
        let (start, length) = axis.geometry(bounds);
        let (groove, previous_line, next_line) = match axis {
            Axis::Horizontal => (
                Rect::from_xywh(
                    bounds.x() + 14.0,
                    bounds.y(),
                    (bounds.width() - 28.0).max(1.0),
                    bounds.height(),
                )
                .unwrap(),
                strip(axis, bounds, start, 14.0_f32.min(length)),
                strip(axis, bounds, start + length - 14.0, 14.0_f32.min(length)),
            ),
            Axis::Vertical => (
                Rect::from_xywh(
                    bounds.x() + 2.0,
                    bounds.y() + 2.0,
                    2.0,
                    (bounds.height() - 4.0).max(1.0),
                )
                .unwrap(),
                None,
                None,
            ),
        };
        let (groove_start, groove_length) = axis.geometry(groove);
        let minimum = match axis {
            Axis::Horizontal => 26.0,
            Axis::Vertical => 24.0,
        };
        let slider_length = (groove_length * range.page as f32
            / (range.maximum + range.page) as f32)
            .floor()
            .max(minimum)
            .min(groove_length);
        let available = groove_length - slider_length;
        let offset = if range.maximum == 0 {
            0.0
        } else {
            (value.min(range.maximum) as f64 * f64::from(available) / range.maximum as f64).round()
                as f32
        };
        let slider = strip(axis, groove, groove_start + offset, slider_length).unwrap();
        // 1. 【文字识别】【样式表端点】纵向样式采用包含端点的区域，滑块优先命中，避免轨道与滑块边界冲突
        let overlap = if axis == Axis::Vertical { 1.0 } else { 0.0 };
        Self {
            axis,
            bounds,
            groove,
            slider,
            previous_line,
            next_line,
            previous_page: strip(axis, groove, groove_start, offset + overlap),
            next_page: strip(
                axis,
                groove,
                groove_start + offset + slider_length - overlap,
                available - offset + overlap,
            ),
            maximum: range.maximum,
        }
    }

    /// 【文字识别】【部件命中】将 position 转换为滑块、箭头或轨道，空白边距返回空值，滑块优先
    pub fn part(&self, position: (f64, f64)) -> Option<Part> {
        [
            (Some(self.slider), Part::Slider),
            (self.previous_line, Part::PreviousLine),
            (self.next_line, Part::NextLine),
            (self.previous_page, Part::PreviousPage),
            (self.next_page, Part::NextPage),
        ]
        .into_iter()
        .find_map(|(rect, part)| rect.filter(|rect| contains(*rect, position)).map(|_| part))
    }

    /// 【文字识别】【滑块索引】按 pointer、anchor 把鼠标位置映射到真实滚动索引，超出轨道固定在边缘
    pub fn value_at(&self, pointer: f32, anchor: f32) -> usize {
        let (start, length) = self.axis.geometry(self.groove);
        let (_, slider_length) = self.axis.geometry(self.slider);
        if length <= slider_length || self.maximum == 0 {
            return 0;
        }
        (((pointer - anchor - start).clamp(0.0, length - slider_length) as f64
            * self.maximum as f64)
            / f64::from(length - slider_length))
        .round() as usize
    }
}

#[cfg(test)]
mod tests;
