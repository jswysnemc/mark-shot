//! 【线条】【样式预览】复用紧凑选择面板，以真实头部和线型路径绘制属性图标

use tiny_skia::{Color, Rect};

use crate::features::{
    arrow::{ArrowStyle, path},
    line::{LineSkeleton, LineStyle, draw},
};
use crate::renderer::Painter;
use crate::ui::choice_popover::{ChoiceOption, ChoicePopover, draw_choice_popover};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineChoice {
    Heads,
    Pattern,
}

impl LineChoice {
    pub const ALL: [Self; 2] = [Self::Heads, Self::Pattern];

    /// 【线条】【样式面板创建】按当前选择类型返回关闭的紧凑面板，不分配像素缓存
    pub fn popover(self) -> ChoicePopover {
        match self {
            Self::Heads => ChoicePopover::new(
                "Arrow style",
                ArrowStyle::ALL
                    .map(|style| ChoiceOption {
                        label: style.label(),
                    })
                    .to_vec(),
            ),
            Self::Pattern => ChoicePopover::new(
                "Line style",
                LineStyle::ALL
                    .map(|style| ChoiceOption {
                        label: style.label(),
                    })
                    .to_vec(),
            ),
        }
    }
}

/// 【线条】【属性图标】按 kind 和 index 在 rect 内绘制真实样式，使用 painter、color；越界或无空间时不绘制，无返回值
pub fn preview(
    painter: &mut Painter<'_>,
    kind: LineChoice,
    index: usize,
    rect: Rect,
    color: Color,
) {
    let center = (rect.top() + rect.bottom()) * 0.5;
    let Some(line) = LineSkeleton::new((rect.left() + 1.0, center), (rect.right() - 1.0, center))
    else {
        return;
    };
    let outline = match kind {
        LineChoice::Heads => ArrowStyle::ALL
            .get(index)
            .and_then(|style| path::outline(&line, *style, LineStyle::Solid, 2.0)),
        LineChoice::Pattern => LineStyle::ALL
            .get(index)
            .and_then(|style| draw::outline(&line, *style, 2.0)),
    };
    if let Some(path) = outline {
        painter.fill_path(&path, color);
    }
}

/// 【线条】【选择面板绘制】将 panel 按 kind 绘入 painter；内容缓存、透明度及窄界面布局由共用组件管理，无返回值
pub fn draw_popover(painter: &mut Painter<'_>, panel: &mut ChoicePopover, kind: LineChoice) {
    draw_choice_popover(painter, panel, |painter, index, rect, color| {
        preview(painter, kind, index, rect, color)
    });
}
