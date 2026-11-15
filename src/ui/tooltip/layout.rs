//! 【工具提示】【边缘布局】以按钮为中心，优先避开整个工具栏并限制在当前显示器内

use super::Target;
use tiny_skia::Rect;

const MARGIN: f32 = 8.0;
const GAP: f32 = 8.0;

/// 【工具提示】【位置计算】按 target、文字宽度 width、字号 font 与显示器 size 返回提示矩形；无可用面积时返回空值
pub(crate) fn place(target: Target, width: f32, font: f32, size: (f32, f32)) -> Option<Rect> {
    let width = (width + 24.0).min(size.0 - MARGIN * 2.0);
    let height = (font * crate::theme::font::line_height() + 16.0).min(size.1 - MARGIN * 2.0);
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    // 1. 【工具提示】【方向避让】横栏优先下方，竖栏优先右侧；空间不足时切换到相反一侧
    let (x, y) = if target.vertical {
        let right = target.toolbar.right() + GAP;
        (
            if right + width <= size.0 - MARGIN {
                right
            } else {
                target.toolbar.left() - GAP - width
            },
            target.cell.top() + (target.cell.height() - height) / 2.0,
        )
    } else {
        let below = target.toolbar.bottom() + GAP;
        (
            target.cell.left() + (target.cell.width() - width) / 2.0,
            if below + height <= size.1 - MARGIN {
                below
            } else {
                target.toolbar.top() - GAP - height
            },
        )
    };
    // 2. 【工具提示】【边界限制】沿当前显示器的本地坐标收敛，不套用图片缩放或画布平移
    Rect::from_xywh(
        x.clamp(MARGIN, size.0 - width - MARGIN),
        y.clamp(MARGIN, size.1 - height - MARGIN),
        width,
        height,
    )
}
