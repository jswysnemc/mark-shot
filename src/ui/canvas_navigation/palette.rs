//! 【画布滚动条】【交互配色】轨道与手柄跟随主题，保留透明覆盖与青色拖动强调

use crate::theme::{Mode, Rgba, toolbar::Palette};

pub(super) struct Colors {
    pub track: Rgba,
    pub fill: Rgba,
    pub border: Rgba,
}

/// 【画布滚动条】【状态颜色】按 mode、pressed、thumb_hover 与 hover 返回控件配色，不读取原图
pub(super) fn colors(mode: Mode, pressed: bool, thumb_hover: bool, hover: bool) -> Colors {
    let palette = Palette::new(mode);
    if mode == Mode::Dark {
        return Colors {
            track: Rgba(15, 23, 42, 110),
            fill: if pressed {
                Rgba(94, 234, 212, 230)
            } else if thumb_hover {
                Rgba(241, 245, 249, 240)
            } else if hover {
                Rgba(226, 232, 240, 215)
            } else {
                Rgba(203, 213, 225, 150)
            },
            border: if pressed {
                Rgba(15, 118, 110, 160)
            } else if hover {
                Rgba(15, 23, 42, 120)
            } else {
                Rgba(15, 23, 42, 90)
            },
        };
    }
    Colors {
        track: Rgba(248, 250, 252, 180),
        fill: if pressed {
            palette.hover_fill
        } else if thumb_hover {
            Rgba(51, 65, 85, 240)
        } else if hover {
            Rgba(100, 116, 139, 215)
        } else {
            Rgba(100, 116, 139, 150)
        },
        border: if pressed {
            palette.hover_border
        } else {
            palette.control_border
        },
    }
}
