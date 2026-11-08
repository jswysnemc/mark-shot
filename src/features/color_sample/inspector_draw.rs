//! 【取色面板】【格式展示】紧凑预览、历史颜色和八种格式行，仅绘制界面，不进入截图导出

use super::{formats::Format, inspector::Hit};
use crate::{
    editor::EditorState,
    renderer::{Painter, text::TextStyle},
    ui::controls,
};
use tiny_skia::Rect;

/// 【取色面板】【布局矩形】按 left、top、width、height 返回有效逻辑矩形
fn area(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(left, top, width.max(1.0), height.max(1.0)).unwrap()
}

/// 【取色面板】【屏幕适配】按 size 和 anchor 放置面板，限制在当前显示器内，矮窗口使用紧凑行距
pub fn panel_rect(size: (i32, i32), anchor: (f64, f64)) -> Rect {
    let width = 426.0_f32.min((size.0 - 16).max(1) as f32);
    let height = 398.0_f32.min((size.1 - 16).max(1) as f32);
    let left = if anchor.0 as f32 + 20.0 + width < size.0 as f32 {
        anchor.0 as f32 + 20.0
    } else {
        anchor.0 as f32 - width - 20.0
    };
    area(
        left.clamp(8.0, (size.0 as f32 - width - 8.0).max(8.0)),
        (anchor.1 as f32 + 20.0).clamp(8.0, (size.1 as f32 - height - 8.0).max(8.0)),
        width,
        height,
    )
}

/// 【取色面板】【格式行】按 panel 和 index 返回格式行，八行均可见
fn row_rect(panel: Rect, index: usize) -> Rect {
    let height = (panel.height() - 132.0) / 8.0;
    area(
        panel.left() + 10.0,
        panel.top() + 124.0 + index as f32 * height,
        panel.width() - 20.0,
        height - 3.0,
    )
}

/// 【取色面板】【复制按钮】按 panel、index 返回对应行的复制按钮
pub fn copy_rect(panel: Rect, index: usize) -> Rect {
    let row = row_rect(panel, index);
    area(
        row.right() - 47.0,
        row.top() + 2.0,
        45.0,
        row.height() - 4.0,
    )
}

/// 【取色面板】【历史色块】按 panel、index 返回八个等宽色块中的一个
pub fn swatch_rect(panel: Rect, index: usize) -> Rect {
    let width = (panel.width() - 20.0) / 8.0;
    area(
        panel.left() + 10.0 + index as f32 * width,
        panel.top() + 93.0,
        width - 5.0,
        23.0,
    )
}

/// 【取色面板】【返回按钮】按 panel 返回继续取色按钮，不删除已有标注
pub fn close_rect(panel: Rect) -> Rect {
    area(panel.right() - 95.0, panel.top() + 8.0, 85.0, 24.0)
}

/// 【取色面板】【画面合成】按 monitor、dirty 将 state 面板绘到局部画布，文字字库仅按需准备
pub fn draw(state: &mut EditorState, monitor: usize, dirty: Option<Rect>) {
    let Some((owner, panel)) = state.color_inspector.panel else {
        return;
    };
    if owner != monitor || state.ui_hidden {
        return;
    }
    let inspector = &mut state.color_inspector;
    let Some(color) = inspector.live else {
        return;
    };
    let palette = crate::theme::window::Palette::current();
    let chinese =
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    let fonts = inspector.fonts.as_mut().unwrap();
    crate::fonts::ui_fallback::ensure_text(fonts, "颜色格式 继续取色 复制");
    let mut painter = Painter::new(&mut state.canvas[monitor])
        .with_scale(state.placements[monitor].scale)
        .with_text(fonts, &mut state.text.swash_cache);
    painter.clipped(dirty, |painter| {
        controls::frame(painter, panel, palette.window, palette.border, 10.0);
        painter.text(
            area(panel.left() + 12.0, panel.top() + 8.0, 160.0, 24.0),
            if chinese {
                "颜色格式"
            } else {
                "Color formats"
            },
            TextStyle::new(13.0, palette.text),
        );
        controls::button(
            painter,
            close_rect(panel),
            if chinese {
                "继续取色"
            } else {
                "Sample again"
            },
            false,
            inspector.hover == Some(Hit::Close),
            true,
        );
        let pixel = color.to_color_u8();
        let fill = crate::theme::Rgba(pixel.red(), pixel.green(), pixel.blue(), pixel.alpha());
        let ink = if color.red() * 0.299 + color.green() * 0.587 + color.blue() * 0.114 > 0.55 {
            tiny_skia::Color::BLACK
        } else {
            tiny_skia::Color::WHITE
        };
        let preview = area(
            panel.left() + 10.0,
            panel.top() + 39.0,
            panel.width() - 20.0,
            46.0,
        );
        controls::frame(painter, preview, fill, palette.border, 7.0);
        painter.text(
            preview,
            &Format::Hex.text(color),
            TextStyle::new(18.0, ink).center(),
        );
        for index in 0..8 {
            let fill = inspector
                .recent
                .get(index)
                .map_or(palette.alternate, |color| {
                    let color = color.to_color_u8();
                    crate::theme::Rgba(color.red(), color.green(), color.blue(), color.alpha())
                });
            controls::frame(
                painter,
                swatch_rect(panel, index),
                fill,
                if inspector.hover == Some(Hit::Recent(index)) {
                    palette.accent
                } else {
                    palette.border
                },
                5.0,
            );
        }
        for (index, format) in Format::ALL.into_iter().enumerate() {
            let row = row_rect(panel, index);
            painter.fill_rounded(row, 5.0, palette.alternate);
            painter.text(
                area(row.left() + 7.0, row.top(), 64.0, row.height()),
                format.label(),
                TextStyle::new(11.0, palette.muted),
            );
            let text = format.text(color);
            let width = row.width() - 120.0;
            let text_size =
                if painter.measure_styled(&text, TextStyle::new(12.0, palette.text)) > width {
                    10.0
                } else {
                    12.0
                };
            painter.text(
                area(row.left() + 70.0, row.top(), width, row.height()),
                &text,
                TextStyle::new(text_size, palette.text),
            );
            controls::button(
                painter,
                copy_rect(panel, index),
                if chinese { "复制" } else { "Copy" },
                false,
                inspector.hover == Some(Hit::Copy(format)),
                true,
            );
        }
    });
}
