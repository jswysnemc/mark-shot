//! 【图片窗口】【窗口外观】紧凑标题栏与窗口边框独立于图片缩放和标注导出

use crate::{editor::EditorState, renderer::Painter, theme::window::Palette};
use tiny_skia::Rect;

pub const TITLE_HEIGHT: f32 = 32.0;
pub const INSET: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Move,
    Minimize,
    Maximize,
    Close,
    Resize(crate::backend::window::WindowEdge),
}

#[derive(Default)]
pub(crate) struct State {
    pub enabled: bool,
    pub hovered: Option<Hit>,
    pub pressed: Option<Hit>,
    pub action: Option<crate::types::image_window::Action>,
}

/// 【图片窗口】【按钮边界】按 size 和 index 返回从右至左的关闭、最大化、最小化按钮
pub(crate) fn button(size: (u32, u32), index: usize) -> Rect {
    Rect::from_xywh(
        size.0 as f32 - 6.0 - 32.0 * (index + 1) as f32,
        4.0,
        30.0,
        24.0,
    )
    .unwrap()
}

/// 【图片窗口】【窗口命中】按 size 和 point 返回边框或标题栏动作，画布位置返回空值
pub(crate) fn hit(size: (u32, u32), point: (f64, f64)) -> Option<Hit> {
    use crate::backend::window::WindowEdge::*;
    let left = point.0 < 4.0;
    let right = point.0 >= f64::from(size.0) - 4.0;
    let top = point.1 < 4.0;
    let bottom = point.1 >= f64::from(size.1) - 4.0;
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
    for (index, control) in [Hit::Close, Hit::Maximize, Hit::Minimize]
        .into_iter()
        .enumerate()
    {
        if crate::ui::selection_size::state::contains(button(size, index), point) {
            return Some(control);
        }
    }
    (point.1 < f64::from(TITLE_HEIGHT)).then_some(Hit::Move)
}

/// 【图片窗口】【边框绘制】按 state 和 monitor 绘制标题按钮及细边框，不修改原图和导出缓冲
pub(crate) fn draw(state: &mut EditorState, monitor: usize) {
    if !state.image_window.enabled || monitor != 0 {
        return;
    }
    let size = (
        state.placements[0].size.0 as u32,
        state.placements[0].size.1 as u32,
    );
    let palette = Palette::current();
    let scale = state.placements[0].scale;
    let mut painter = Painter::new(&mut state.canvas[0])
        .with_scale(scale)
        .with_text(&mut state.text.font_system, &mut state.text.swash_cache);
    painter.fill_rounded(
        Rect::from_xywh(0.0, 0.0, size.0 as f32, TITLE_HEIGHT).unwrap(),
        0.0,
        palette.base,
    );
    painter.text(
        Rect::from_xywh(12.0, 4.0, (size.0 as f32 - 120.0).max(1.0), 24.0).unwrap(),
        "Mark Shot",
        crate::renderer::text::TextStyle::new(12.0, palette.text),
    );
    for (index, control) in [Hit::Close, Hit::Maximize, Hit::Minimize]
        .into_iter()
        .enumerate()
    {
        let area = button(size, index);
        if state.image_window.hovered == Some(control) {
            painter.fill_rounded(area, 4.0, palette.alternate);
        }
        let cx = area.left() + area.width() / 2.0;
        let cy = area.top() + area.height() / 2.0;
        let mut path = tiny_skia::PathBuilder::new();
        match control {
            Hit::Close => {
                path.move_to(cx - 4.0, cy - 4.0);
                path.line_to(cx + 4.0, cy + 4.0);
                path.move_to(cx + 4.0, cy - 4.0);
                path.line_to(cx - 4.0, cy + 4.0);
            }
            Hit::Maximize => path.push_rect(Rect::from_xywh(cx - 4.0, cy - 4.0, 8.0, 8.0).unwrap()),
            _ => {
                path.move_to(cx - 4.0, cy + 2.0);
                path.line_to(cx + 4.0, cy + 2.0);
            }
        }
        painter.stroke_path(
            &path.finish().unwrap(),
            palette.text,
            &tiny_skia::Stroke {
                width: 1.2,
                ..Default::default()
            },
        );
    }
    crate::ui::window_frame::draw(&mut state.canvas[0], scale);
}
