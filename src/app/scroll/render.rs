//! 【滚动截图】【内容提交】仅在内容或输入变化时绘制，不使用周期重绘计时器

use super::ScrollRun;
use crate::{backend::ScreenOverlay, editor::EditorState, renderer::Painter, scroll::panel};

/// 【滚动截图】【布局同步】按 state 的目标输出和 run 原图尺寸重算布局，返回可见状态，无像素分配
pub(super) fn layout(state: &EditorState, run: &mut ScrollRun) -> bool {
    let placement = &state.placements[run.session.monitor_idx];
    let (width, height) = run.session.stitcher.dimensions();
    run.layout = panel::constrained_layout(
        run.session.region,
        (placement.size.0 as f32, placement.size.1 as f32),
        (width > 0 && height > 0).then_some((width as u32, height as u32)),
        run.panel_limits,
    );
    let hovered = panel::hit_test(&run.layout, run.pointer);
    if hovered != run.hovered {
        run.hovered = hovered;
        run.dirty = true;
    }
    run.layout.fits_without_overlap || run.session.paused
}

/// 【滚动截图】【帧提交】将 state 与 run 绘制到 overlay，隐藏面板的捕获期间使用空输入区，无返回值
pub(super) fn paint(
    state: &mut EditorState,
    run: &mut ScrollRun,
    overlay: &mut dyn ScreenOverlay,
) -> crate::AppResult {
    if overlay.scroll_controls_hidden() && (!run.session.paused || run.needs_clean_frame) {
        run.dirty = false;
        return Ok(());
    }
    run.panel_limits = overlay.scroll_panel_limits(run.session.monitor_idx);
    let visible = layout(state, run) || overlay.is_scroll_window();
    overlay.configure_scroll_panel(
        run.layout.rect,
        run.session.paused && !run.needs_clean_frame,
    )?;
    let idx = run.session.monitor_idx;
    let bounds = run
        .layout
        .rect
        .round_out()
        .ok_or("invalid scroll panel bounds")?;
    let size = crate::types::px_size(
        (bounds.width(), bounds.height()),
        state.placements[idx].scale,
    );
    let canvas = state
        .canvas
        .get_mut(idx)
        .ok_or("the scroll capture canvas is gone")?;
    if (canvas.width(), canvas.height()) != size {
        *canvas = tiny_skia::Pixmap::new(size.0, size.1).ok_or("cannot allocate scroll panel")?;
    }
    canvas.data_mut().fill(0);
    if visible {
        if crate::ui::language::Language::current() == crate::ui::language::Language::Chinese {
            crate::fonts::ui_fallback::ensure_text(
                &mut state.text.font_system,
                "等待滚动正在捕获已暂停捕获错误未找到重叠区域区域已调整",
            );
        }
        let mut painter = Painter::new(canvas)
            .with_scale(state.placements[idx].scale)
            .with_origin((bounds.x() as f32, bounds.y() as f32))
            .with_icons(&state.icons_cache)
            .with_text(&mut state.text.font_system, &mut state.text.swash_cache);
        panel::draw(
            &mut painter,
            &mut run.session,
            &run.layout,
            run.hovered,
            run.pressed,
            &mut run.preview,
        );
    }
    let input = visible.then(|| {
        let rect = run.layout.rect;
        (
            rect.left().floor() as i32,
            rect.top().floor() as i32,
            rect.width().ceil() as i32,
            rect.height().ceil() as i32,
        )
    });
    overlay.set_input_rect(idx, input);
    // 1. 【滚动截图】【局部交付】控制像素与原图区域分别交给后端，边线不再依赖整屏画布
    overlay.stage_scroll_panel(
        idx,
        &crate::types::scroll_panel::ScrollPanelFrame {
            pixels: canvas.data(),
            size,
            bounds,
            region: run.session.region,
            scale: state.placements[idx].scale,
            border: crate::theme::toolbar::Palette::current()
                .active_border
                .color(),
        },
    )?;
    overlay.flush()?;
    run.dirty = false;
    Ok(())
}
