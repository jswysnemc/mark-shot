//! 【截图起始】【紧凑布局】绘制与命中共用逻辑坐标，不随鼠标位置避让

use super::Control;
use tiny_skia::Rect;

/// 提示面板单行高度
pub(super) const HINT_ROW: f32 = 20.0;
/// 提示面板内边距
const HINT_PADDING: f32 = 8.0;
/// 提示面板两列之间的间距
const HINT_GAP: f32 = 14.0;
/// 提示面板启用两列所需的最小宽度
const HINT_TWO_COLUMNS: f32 = 330.0;

pub(crate) struct Grid {
    pub bounds: Rect,
    pub columns: usize,
    pub rows: usize,
}

/// 【截图起始】【矩形构造】按位置和正尺寸返回有效矩形
pub(super) fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width.max(1.0), height.max(1.0)).unwrap()
}

/// 【截图起始】【上方入口】按 size 和 active 返回顶部居中单行栏，录制活动时加宽以容纳状态与控制
pub(crate) fn top(size: (i32, i32), active: bool) -> Rect {
    let wanted: f32 = if active { 484.0 } else { 308.0 };
    let width = wanted.min((size.0 as f32 - 16.0).max(1.0));
    rect((size.0 as f32 - width) / 2.0, 12.0, width, 38.0)
}

/// 【截图起始】【消息条】按 size 返回紧贴入口下方的录制提示条，仅在没有活动录制时使用
pub(crate) fn message(size: (i32, i32)) -> Rect {
    let top = top(size, false);
    rect(top.left(), top.bottom() + 4.0, top.width(), 30.0)
}

/// 【截图起始】【快捷面板】按 size 返回左下角提示的最大范围，宽高分别不超过窗口四分之一
pub(crate) fn hints(size: (i32, i32)) -> Rect {
    let width = 430.0_f32.min(size.0 as f32 / 4.0).max(1.0);
    let height = 208.0_f32.min(size.1 as f32 / 4.0).max(1.0);
    rect(
        12.0,
        (size.1 as f32 - height - 12.0).max(0.0),
        width,
        height,
    )
}

/// 【截图起始】【提示网格】按 size 与两组条目数返回实际面板，空间不足时裁剪行数
///
/// 参数：size 输出逻辑尺寸、counts 选择组与录制组的条目数；
/// 返回底部对齐的网格，没有可显示的行时返回空值
pub(crate) fn hint_grid(size: (i32, i32), counts: (usize, usize)) -> Option<Grid> {
    let limit = hints(size);
    let columns = if limit.width() >= HINT_TWO_COLUMNS {
        2
    } else {
        1
    };
    let wanted = if columns == 2 {
        counts.0.max(counts.1)
    } else {
        counts.0 + counts.1
    };
    let fit = ((limit.height() - HINT_PADDING * 2.0) / HINT_ROW).max(0.0) as usize;
    let rows = wanted.min(fit);
    if rows == 0 {
        return None;
    }
    let height = rows as f32 * HINT_ROW + HINT_PADDING * 2.0;
    Some(Grid {
        bounds: rect(limit.left(), limit.bottom() - height, limit.width(), height),
        columns,
        rows,
    })
}

/// 【截图起始】【提示单元】按 grid、column 和 row 返回一条提示的矩形，绘制键位和说明时共用
pub(super) fn hint_cell(grid: &Grid, column: usize, row: usize) -> Rect {
    let columns = grid.columns as f32;
    let width = (grid.bounds.width() - 20.0 - (columns - 1.0) * HINT_GAP) / columns;
    rect(
        grid.bounds.left() + 10.0 + column as f32 * (width + HINT_GAP),
        grid.bounds.top() + HINT_PADDING + row as f32 * HINT_ROW,
        width,
        HINT_ROW,
    )
}

/// 【截图起始】【入口按钮】按 size 和 active 返回按钮边界，录制活动时以暂停和停止替换录制入口
pub(crate) fn controls(size: (i32, i32), active: bool) -> Vec<(Control, Rect)> {
    let top = top(size, active);
    if !active {
        let width = (top.width() - 14.0) / 4.0;
        return [
            Control::Region,
            Control::Fullscreen,
            Control::Video,
            Control::Gif,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, control)| {
            (
                control,
                rect(
                    top.left() + 4.0 + index as f32 * (width + 2.0),
                    top.top() + 4.0,
                    width,
                    30.0,
                ),
            )
        })
        .collect();
    }
    // 1. 【截图起始】【窄屏收缩】固定控件总宽超过栏宽时等比缩小，状态文字区域随之收窄
    let scale = ((top.width() - 8.0) / 332.0).min(1.0);
    let (entry, pause, stop) = (66.0 * scale, 78.0 * scale, 104.0 * scale);
    let y = top.top() + 4.0;
    let stop_left = top.right() - 4.0 - stop;
    vec![
        (Control::Region, rect(top.left() + 4.0, y, entry, 30.0)),
        (
            Control::Fullscreen,
            rect(top.left() + 6.0 + entry, y, entry, 30.0),
        ),
        (
            Control::Pause,
            rect(stop_left - 4.0 * scale - pause, y, pause, 30.0),
        ),
        (Control::Stop, rect(stop_left, y, stop, 30.0)),
    ]
}

/// 【截图起始】【状态区域】按 size 返回录制活动时栏内状态文字的范围，位于截图入口与控制按钮之间
pub(crate) fn status(size: (i32, i32)) -> Rect {
    let items = controls(size, true);
    let left = items[1].1.right() + 12.0;
    let right = items[2].1.left() - 8.0;
    let top = top(size, true);
    rect(left, top.top(), right - left, top.height())
}

/// 【截图起始】【控件命中】按 size、active 和 point 返回顶部按钮或栏内空白；提示面板与消息条不拦截输入
pub(crate) fn hit(size: (i32, i32), active: bool, point: (f64, f64)) -> Option<Control> {
    let contains = |bounds| crate::ui::selection_size::state::contains(bounds, point);
    controls(size, active)
        .into_iter()
        .find(|(_, area)| contains(*area))
        .map(|(control, _)| control)
        .or_else(|| contains(top(size, active)).then_some(Control::Background))
}
