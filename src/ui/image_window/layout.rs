//! 【图片窗口】【合并栏布局】工具与窗口按钮共享一行，空白区域承担窗口拖动

use super::Hit;
use tiny_skia::Rect;

pub const INSET: f32 = 8.0;
pub(crate) const BAR_INSET: f32 = 4.0;
pub(crate) const DRAG_GAP: f32 = 24.0;

/// 【图片窗口】【合并栏高度】按toolbar_height返回容纳工具及窗口按钮的单行高度
pub(crate) fn bar_height(toolbar_height: f32) -> f32 {
    toolbar_height
}

/// 【图片窗口】【窗口按钮占位】无参数，返回三个按钮及间距、右侧边距所需宽度，跟随工具栏图标尺寸
pub(crate) fn controls_width() -> f32 {
    crate::ui::toolbar::metrics::button(false) * 3.0 + button_gap() * 2.0 + BAR_INSET + 6.0
}

/// 【图片窗口】【按钮间距】无参数，返回与工具按钮相同的间距，至少保留两个逻辑像素
fn button_gap() -> f32 {
    crate::config::get().toolbar.button_gap.max(2.0)
}

/// 【图片窗口】【窗口按钮】按size、height与index返回从右至左的关闭、最大化、最小化按钮，按钮与工具垂直居中
pub(crate) fn button(size: (u32, u32), height: f32, index: usize) -> Rect {
    let edge = crate::ui::toolbar::metrics::button(false);
    Rect::from_xywh(
        size.0 as f32 - BAR_INSET - 6.0 - edge - (edge + button_gap()) * index as f32,
        (height - edge) * 0.5,
        edge,
        edge,
    )
    .unwrap()
}

/// 【图片窗口】【窗口命中】按size、height与point返回边框、窗口按钮或合并栏空白拖动，工具按钮由调用方区分
pub(crate) fn hit(size: (u32, u32), height: f32, point: (f64, f64)) -> Option<Hit> {
    use crate::backend::window::WindowEdge::*;
    let left = point.0 < 4.0;
    let right = point.0 >= f64::from(size.0) - 4.0;
    let top = point.1 < 4.0;
    let bottom = point.1 >= f64::from(size.1) - 4.0;
    // 1. 【图片窗口】【边框优先】窗口四边保留调整尺寸入口，不占用工具按钮的内部命中范围
    if let Some(edge) = match (left, right, top, bottom) {
        (true, _, true, _) => Some(TopLeft),
        (_, true, true, _) => Some(TopRight),
        (true, _, _, true) => Some(BottomLeft),
        (_, true, _, true) => Some(BottomRight),
        (true, _, _, _) => Some(Left),
        (_, true, _, _) => Some(Right),
        (_, _, true, _) => Some(Top),
        (_, _, _, true) => Some(Bottom),
        _ => None,
    } {
        return Some(Hit::Resize(edge));
    }
    // 2. 【图片窗口】【窗口按钮】右侧三个按钮独立处理，其余合并栏区域可用于拖动
    for (index, control) in [Hit::Close, Hit::Maximize, Hit::Minimize]
        .into_iter()
        .enumerate()
    {
        if crate::ui::selection_size::state::contains(button(size, height, index), point) {
            return Some(control);
        }
    }
    (point.1 < f64::from(height)).then_some(Hit::Move)
}
