//! 【截图起始】【界面绘制】编排顶部栏与快捷键提示，沿用统一控件主题，快捷键仅提示且不拦截选区

use super::{bar, hints, layout};
use crate::{
    editor::EditorState, renderer::Painter, theme::window::Palette, ui::language::Language,
};
use tiny_skia::Rect;

/// 【截图起始】【面板裁切】按 bounds 与 dirty 的交集执行 draw，painter 提供目标画布，返回空值；缓冲不超过面板尺寸
fn draw_area(
    painter: &mut Painter<'_>,
    bounds: Rect,
    dirty: Option<Rect>,
    draw: impl FnOnce(&mut Painter<'_>),
) {
    match dirty {
        Some(dirty) => {
            if let Some(area) = dirty.intersect(&bounds) {
                painter.clipped(Some(area), draw);
            }
        }
        None => draw(painter),
    }
}

/// 【截图起始】【界面绘制】按 state 与 monitor 绘制顶部栏和提示面板，dirty 限制本帧恢复范围，避免透明底色叠加，无返回值
pub(crate) fn draw(state: &mut EditorState, monitor: usize, dirty: Option<Rect>) {
    if !state.capture_start.shows_bar() {
        return;
    }
    let size = state.placements[monitor].size;
    let scale = state.placements[monitor].scale;
    let cn = Language::current() == Language::Chinese;
    let palette = Palette::current();
    // 1. 【截图起始】【提示内容】先按当前绑定生成提示，再由布局决定列数与可见行数
    let groups = hints::groups(state, cn);
    let grid = (state.capture_start.visible && state.capture_start.hint_hovered != Some(monitor))
        .then(|| layout::hint_grid(size, (groups.0.len(), groups.1.len())))
        .flatten();
    // 2. 【截图起始】【缺失字形】只准备本帧实际绘制的文字，避免手工维护字符表
    if cn {
        let mut glyphs = bar::glyphs(&state.capture_start, cn);
        for hint in groups.0.iter().chain(&groups.1) {
            glyphs.push_str(&hint.key);
            glyphs.push_str(hint.title);
        }
        crate::fonts::ui_fallback::ensure_text(&mut state.text.font_system, &glyphs);
    }
    let start = &state.capture_start;
    let mut painter = Painter::new(&mut state.canvas[monitor])
        .with_scale(scale)
        .with_icons(&state.icons_cache)
        .with_text(&mut state.text.font_system, &mut state.text.swash_cache);
    // 3. 【截图起始】【局部合成】只绘制已恢复的像素，鼠标移动时保留范围外材质与文字
    let mut bar_bounds = layout::toolbar(size, start, monitor);
    if start.message_visible() {
        bar_bounds = bar_bounds
            .join(&layout::message(size))
            .unwrap_or(bar_bounds);
    }
    draw_area(&mut painter, bar_bounds, dirty, |painter| {
        bar::draw(painter, start, monitor, size, cn, palette);
    });
    // 4. 【截图起始】【低分辨率】面板按可见行数收缩，组合键列不随比例缩放被挤掉，高度不超过屏幕四分之一
    if let Some(grid) = grid {
        draw_area(&mut painter, grid.bounds, dirty, |painter| {
            hints::draw(painter, &grid, &groups, palette);
        });
    }
}
