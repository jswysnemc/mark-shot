//! 【多选区截图】【屏幕预览】用冻结图和区域并集绘制遮罩；操作提示仅出现在窗口内

use crate::{
    editor::EditorState,
    renderer::{FrozenImage, Painter},
    theme::window::Palette,
    types::DamageRect,
    ui::language::Language,
};
use tiny_skia::{Color, FillRule, PathBuilder, Rect, Stroke};

/// 【多选区截图】【局部合成】按 monitor 和本地 dirty 恢复冻结像素、预览与提示，返回实际设备像素损坏范围
pub(in crate::app) fn draw(
    state: &mut EditorState,
    monitor: usize,
    dirty: Option<Rect>,
) -> Option<DamageRect> {
    let session = state.multi_capture.as_ref()?;
    let placement = &state.placements[monitor];
    let scale = placement.scale;
    let offset = placement.offset();
    let translation = tiny_skia::Transform::from_translate(offset.0, offset.1);
    let viewport = placement.rect()?.transform(translation)?;
    let canvas = &mut state.canvas[monitor];
    let damage = dirty
        .and_then(|rect| crate::renderer::scale_rect(&rect, scale))
        .and_then(|rect| crate::renderer::rect_bounds(&rect, canvas.width(), canvas.height()));
    let image = FrozenImage::desktop(
        std::slice::from_ref(&state.captures[monitor]),
        std::slice::from_ref(placement),
    );
    let cn = Language::current() == Language::Chinese;
    let labels = super::bar::Bar::labels(cn);
    let palette = Palette::current();
    let bar = super::bar::Bar::new(viewport)?;
    let show_hint = dirty.is_none_or(|rect| {
        rect.transform(translation)
            .and_then(|rect| rect.intersect(&bar.bounds))
            .is_some()
    });
    if cn && show_hint {
        crate::fonts::ui_fallback::ensure_text(&mut state.text.font_system, labels.0);
        crate::fonts::ui_fallback::ensure_text(&mut state.text.font_system, labels.1);
    }
    Painter::new(canvas)
        .with_scale(scale)
        .with_origin(offset)
        .with_text(&mut state.text.font_system, &mut state.text.swash_cache)
        .clipped(dirty, |painter| {
            // 1. 【多选区截图】【背景恢复】只恢复本轮损坏区域，不从上帧叠加过遮罩的像素取样
            let transform = painter.transform();
            painter.canvas().fill(Color::BLACK);
            image.draw_transformed(painter.canvas(), transform);
            let mut shade = PathBuilder::new();
            shade.push_rect(viewport);
            for rect in session.regions.visible() {
                if let Some(visible) = viewport.intersect(rect) {
                    shade.push_rect(visible);
                }
            }
            if let Some(path) = shade.finish() {
                painter.fill_path_rule(
                    &path,
                    crate::config::get().selection.dim,
                    FillRule::EvenOdd,
                );
            }
            // 2. 【多选区截图】【窗口控件】草稿边线和说明最后绘制，不参与选区几何或导出
            if let Some(rect) = session.regions.marquee() {
                painter.stroke_path(
                    &PathBuilder::from_rect(rect),
                    palette.accent,
                    &Stroke {
                        width: 1.5,
                        ..Default::default()
                    },
                );
            }
            if show_hint {
                bar.draw(
                    painter,
                    labels,
                    session.ready(),
                    session.pressed_on == Some(monitor),
                    palette,
                );
            }
        });
    damage
}
