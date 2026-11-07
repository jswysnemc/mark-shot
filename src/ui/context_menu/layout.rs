//! 【上下文菜单】【原版布局】按字体度量排列菜单行与快捷键列，菜单越界时先翻转再贴边

use super::Row;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use tiny_skia::Rect;

pub struct Text {
    pub label: Buffer,
    pub shortcut: Buffer,
    pub shortcut_width: f32,
}

pub struct Layout {
    pub size: (f32, f32),
    pub rows: Vec<Rect>,
    pub text: Vec<Text>,
    pub line_height: f32,
    pub arrow_column: bool,
}

/// 【上下文菜单】【文字排版】按 fonts、family、height 排版 text，返回缓冲及舍入宽度，与原版整数文字度量一致
fn shape(fonts: &mut FontSystem, family: &str, height: f32, text: &str) -> (Buffer, f32) {
    let mut buffer = Buffer::new(fonts, Metrics::new(16.0, height));
    buffer.set_size(None, None);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::Name(family)),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max)
        .round();
    (buffer, width)
}

impl Layout {
    /// 【上下文菜单】【行布局】按 rows 和 fonts 返回单列布局，只定向加载菜单字体，不扫描完整字库
    pub fn new(rows: &[Row], fonts: &mut FontSystem) -> Self {
        Self::with_submenus(rows, fonts, &[])
    }

    /// 【上下文菜单】【子菜单布局】按 rows、fonts 和 submenu_rows 排列文字、快捷键及箭头列，返回统一布局
    pub fn with_submenus(rows: &[Row], fonts: &mut FontSystem, submenu_rows: &[usize]) -> Self {
        let family = crate::fonts::text_faces::family(fonts, "DejaVu LGC Sans", false, false);
        let line_height = crate::fonts::text_faces::line_height(fonts, &family, 16.0, false, false);
        let mut label_width: f32 = 0.0;
        let mut shortcut_width: f32 = 0.0;
        let text = rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let (label, width) = shape(fonts, &family, line_height, row.label);
                let (shortcut, key_width) = shape(
                    fonts,
                    &family,
                    line_height,
                    if submenu_rows.contains(&index) {
                        ""
                    } else {
                        row.shortcut
                    },
                );
                label_width = label_width.max(width);
                shortcut_width = shortcut_width.max(key_width);
                Text {
                    label,
                    shortcut,
                    shortcut_width: key_width,
                }
            })
            .collect();
        let width = (24.0
            + label_width
            + if shortcut_width > 0.0 {
                24.0 + shortcut_width
            } else {
                0.0
            }
            + if submenu_rows.is_empty() { 0.0 } else { 16.0 })
        .clamp(168.0, 420.0);
        let mut top = 4.0;
        let rects = rows
            .iter()
            .map(|row| {
                let height = if row.separator() {
                    9.0
                } else {
                    (line_height + 10.0).max(28.0)
                };
                let rect = Rect::from_xywh(4.0, top, width - 8.0, height).unwrap();
                top += height;
                rect
            })
            .collect();
        Self {
            size: (width, top + 4.0),
            rows: rects,
            text,
            line_height,
            arrow_column: !submenu_rows.is_empty(),
        }
    }
}

/// 【上下文菜单】【屏幕避让】按 anchor、size、bounds 返回菜单左上角；尺寸超过屏幕时保持左上角可见
pub fn position(anchor: (f32, f32), size: (f32, f32), bounds: (f32, f32)) -> (f32, f32) {
    let axis = |point: f32, length: f32, limit: f32| {
        let flipped = if point + length > limit && point - length >= 0.0 {
            point - length + 1.0
        } else {
            point
        };
        flipped.clamp(0.0, (limit - length).max(0.0))
    };
    (
        axis(anchor.0, size.0, bounds.0),
        axis(anchor.1, size.1, bounds.1),
    )
}
