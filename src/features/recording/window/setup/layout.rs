//! 【录制设置】【控件布局】绘制与命中共用紧凑几何，分段行随窗口宽度伸缩

use super::{
    Setup,
    options::{self, Row},
};
use crate::ui::controls;
use tiny_skia::Rect;

/// 准备窗口的初始与最小逻辑尺寸
pub(super) const SIZE: (u32, u32) = (400, 372);
/// 标题栏高度，同时是窗口拖动范围
pub(super) const TITLE: f32 = 44.0;
/// 左右内边距
pub(super) const PAD: f32 = 16.0;
/// 配置行高度
const ROW: f32 = 32.0;
/// 左侧标签列宽度
const LABEL: f32 = 68.0;
/// 底部操作区高度
const FOOTER: f32 = 50.0;
/// 光标开关所在行序号，位于全部分段行之后
const CURSOR_ROW: usize = Row::ALL.len();

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Control {
    Close,
    Choice(Row, usize),
    Cursor,
    Settings,
    Copy,
    Start,
}

/// 【录制设置】【矩形构造】按 x、y、width、height 返回有效控件矩形
pub(super) fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width.max(1.0), height.max(1.0)).unwrap()
}

/// 【录制设置】【行标签】按 index 返回第 index 行左侧标签的范围
pub(super) fn label(index: usize) -> Rect {
    rect(PAD, TITLE + index as f32 * ROW, LABEL, ROW)
}

/// 【录制设置】【行控件】按 size 和 index 返回第 index 行右侧控件的范围，宽度随窗口伸缩
pub(super) fn field(size: (u32, u32), index: usize) -> Rect {
    rect(
        PAD + LABEL + 8.0,
        TITLE + index as f32 * ROW + 3.0,
        size.0 as f32 - PAD * 2.0 - LABEL - 8.0,
        ROW - 6.0,
    )
}

/// 【录制设置】【整行范围】按 size 和 row 返回该行包含标签与控件的范围，滚轮微调按整行命中
pub(super) fn row(size: (u32, u32), row: Row) -> Rect {
    let index = Row::ALL.iter().position(|value| *value == row).unwrap_or(0);
    rect(
        PAD,
        TITLE + index as f32 * ROW,
        size.0 as f32 - PAD * 2.0,
        ROW,
    )
}

/// 【录制设置】【光标行】按 size 返回光标开关整行的点击范围，标签与开关同属一个目标
pub(super) fn cursor(size: (u32, u32)) -> Rect {
    rect(
        PAD,
        TITLE + CURSOR_ROW as f32 * ROW,
        size.0 as f32 - PAD * 2.0,
        ROW,
    )
}

/// 【录制设置】【提示区域】按 size 返回选项与操作区之间的说明或错误范围，高度随窗口伸缩
pub(super) fn notice(size: (u32, u32)) -> Rect {
    let top = TITLE + (CURSOR_ROW + 1) as f32 * ROW + 6.0;
    rect(
        PAD,
        top,
        size.0 as f32 - PAD * 2.0,
        size.1 as f32 - FOOTER - top - 6.0,
    )
}

/// 【录制设置】【操作分隔】按 size 返回底部操作区上缘的纵坐标
pub(super) fn footer(size: (u32, u32)) -> f32 {
    size.1 as f32 - FOOTER
}

/// 【录制设置】【控件列表】按 state 当前尺寸与配置返回全部可点击控件，错误存在时增加复制入口
pub(super) fn controls(state: &Setup) -> Vec<(Control, Rect)> {
    let size = state.size;
    let mut items = vec![(Control::Close, rect(size.0 as f32 - 40.0, 8.0, 28.0, 28.0))];
    // 1. 【录制设置】【分段行】每行的段数来自选项模型，自定义帧率或倒计时会多出一段
    for (index, row) in Row::ALL.into_iter().enumerate() {
        let count = options::count(row, &state.config);
        let bounds = field(size, index);
        for segment in 0..count {
            items.push((
                Control::Choice(row, segment),
                controls::segment(bounds, count, segment),
            ));
        }
    }
    items.push((Control::Cursor, cursor(size)));
    // 2. 【录制设置】【底部操作】次要入口靠左，主操作靠右，错误详情的复制入口位于提示区右上
    let y = footer(size) + 10.0;
    items.push((Control::Settings, rect(PAD, y, 96.0, 30.0)));
    items.push((
        Control::Start,
        rect(size.0 as f32 - PAD - 128.0, y, 128.0, 30.0),
    ));
    if state.failure.is_some() {
        let area = notice(size);
        items.push((
            Control::Copy,
            rect(area.right() - 76.0, area.top(), 76.0, 22.0),
        ));
    }
    items
}

/// 【录制设置】【指针命中】按 state 和 point 返回控件身份，没有命中返回空值
pub(super) fn hit(state: &Setup, point: (f64, f64)) -> Option<Control> {
    controls(state)
        .into_iter()
        .find(|(_, area)| contains(*area, point))
        .map(|(control, _)| control)
}

/// 【录制设置】【范围判断】按 area 和 point 返回指针是否位于矩形内，右下边界不包含
pub(super) fn contains(area: Rect, point: (f64, f64)) -> bool {
    point.0 >= f64::from(area.left())
        && point.0 < f64::from(area.right())
        && point.1 >= f64::from(area.top())
        && point.1 < f64::from(area.bottom())
}
