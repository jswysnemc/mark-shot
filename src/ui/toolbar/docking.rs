//! 【工具栏】【停靠几何】实际工具栏与设置预览共用屏幕边缘和双栏避让计算

use crate::config::capture::ToolbarPosition as Position;

const MARGIN: f32 = 8.0;
const GAP: f32 = 8.0;

/// 【工具栏】【双栏定位】按边缘、屏幕和两栏尺寸返回位置；放不下一行时副栏向屏幕内部错开
pub(crate) fn positions(
    edge: Position,
    bounds: (f32, f32),
    main: (f32, f32),
    side: Option<(f32, f32)>,
) -> ((f32, f32), (f32, f32)) {
    let clamp = |point: (f32, f32), size: (f32, f32)| {
        (
            point
                .0
                .clamp(MARGIN, (bounds.0 - size.0 - MARGIN).max(MARGIN)),
            point
                .1
                .clamp(MARGIN, (bounds.1 - size.1 - MARGIN).max(MARGIN)),
        )
    };
    let other = side.unwrap_or((0.0, 0.0));
    let gap = if side.is_some() { GAP } else { 0.0 };
    let (a, b) = if matches!(edge, Position::Left | Position::Right) {
        let left = edge == Position::Left;
        let height = main.1 + gap + other.1;
        let x = if left {
            MARGIN
        } else {
            bounds.0 - main.0 - MARGIN
        };
        if height <= bounds.1 - 2.0 * MARGIN {
            let y = (bounds.1 - height) / 2.0;
            (
                (x, y),
                (
                    if left {
                        MARGIN
                    } else {
                        bounds.0 - other.0 - MARGIN
                    },
                    y + main.1 + gap,
                ),
            )
        } else {
            (
                (x, (bounds.1 - main.1) / 2.0),
                (
                    if left {
                        x + main.0 + gap
                    } else {
                        x - other.0 - gap
                    },
                    (bounds.1 - other.1) / 2.0,
                ),
            )
        }
    } else {
        let top = edge == Position::Top;
        let y = if top {
            MARGIN
        } else {
            bounds.1 - main.1 - MARGIN
        };
        let width = main.0 + gap + other.0;
        if width <= bounds.0 - 2.0 * MARGIN {
            let x = (bounds.0 - width) / 2.0;
            (
                (x, y),
                (
                    x + main.0 + gap,
                    if top {
                        MARGIN
                    } else {
                        bounds.1 - other.1 - MARGIN
                    },
                ),
            )
        } else {
            (
                ((bounds.0 - main.0) / 2.0, y),
                (
                    (bounds.0 - other.0) / 2.0,
                    if top {
                        y + main.1 + gap
                    } else {
                        y - other.1 - gap
                    },
                ),
            )
        }
    };
    (clamp(a, main), clamp(b, other))
}
