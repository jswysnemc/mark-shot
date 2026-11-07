//! 【画布导航】【倍率气泡】原版指针右下十四像素、六像素圆角与浅深色提示配色

use crate::{
    editor::EditorState,
    renderer::{Painter, text::TextStyle},
    theme::{Mode, Rgba},
};
use cosmic_text::{FontSystem, Style, Weight};
use tiny_skia::{Rect, Stroke};

const FONT_SIZE: f32 = 15.0;

/// 【画布导航】【气泡布局】按text、point、size和fonts返回窗口内气泡，保留八像素边距
pub(crate) fn rect(
    text: &str,
    point: (f64, f64),
    size: (f32, f32),
    fonts: &mut FontSystem,
) -> Rect {
    let (_, width, _) = crate::renderer::text::shape_single_line(
        fonts,
        text,
        FONT_SIZE,
        Weight::SEMIBOLD,
        Style::Normal,
    );
    let width = width.ceil() + 20.0;
    let height = 30.0;
    let left = (point.0 as f32 + 14.0).clamp(8.0, (size.0 - width - 8.0).max(8.0));
    let top = (point.1 as f32 + 14.0).clamp(8.0, (size.1 - height - 8.0).max(8.0));
    Rect::from_xywh(left, top, width, height).unwrap()
}

/// 【画布导航】【气泡绘制】按state、monitor和dirty合成固定屏幕尺寸提示，不进入原图导出，无返回值
pub(crate) fn draw(state: &mut EditorState, monitor: usize, dirty: Option<Rect>) {
    let Some(preview) = state
        .navigation
        .preview
        .as_ref()
        .filter(|preview| preview.monitor == monitor)
    else {
        return;
    };
    let palette = crate::theme::toolbar::Palette::current();
    let (fill, border, text) = if Mode::current() == Mode::Dark {
        (
            Rgba(17, 24, 39, 235),
            Rgba(148, 163, 184, 80),
            Rgba(229, 231, 235, 255),
        )
    } else {
        (
            palette.panel_fill,
            palette.panel_border,
            palette.strong_text,
        )
    };
    let rect = preview.rect;
    let point = preview.point;
    let sample = preview.sample;
    let bounds = preview.damage_bounds();
    let mut painter = Painter::new(&mut state.canvas[monitor])
        .with_scale(state.placements[monitor].scale)
        .with_text(
            state.navigation.preview_fonts.as_mut().unwrap(),
            &mut state.text.swash_cache,
        );
    painter.clipped(dirty, |painter| {
        crate::ui::annotation_selection::control_raster::draw(painter, bounds, |painter| {
            if let Some(sample) = sample {
                super::size_preview::draw(painter, rect, point, &preview.text, sample);
                return;
            }
            painter.fill_rounded(rect, 6.0, fill);
            if let Some(path) =
                crate::renderer::rounded_rect_path(&rect, 6.0, true, true, true, true)
            {
                painter.stroke_path(
                    &path,
                    border,
                    &Stroke {
                        width: 1.0,
                        ..Default::default()
                    },
                );
            }
            painter.text(
                rect,
                &preview.text,
                TextStyle::new(FONT_SIZE, text.color())
                    .center()
                    .weight(Weight::SEMIBOLD),
            );
        });
    });
}
