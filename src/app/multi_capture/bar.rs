//! 【多选区截图】【操作栏】说明与编辑按钮共用窗口边界；空间不足时优先保留按钮

use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use tiny_skia::Rect;

pub(super) struct Bar {
    pub bounds: Rect,
    pub edit: Rect,
    hint: Option<Rect>,
}

impl Bar {
    /// 【多选区截图】【操作栏布局】按 viewport 返回底部说明与按钮的同坐标系边界，空视口返回空值
    pub fn new(viewport: Rect) -> Option<Self> {
        let width = viewport.width().min(510.0);
        let height = viewport.height().min(32.0);
        let bounds = Rect::from_xywh(
            viewport.left() + (viewport.width() - width) * 0.5,
            (viewport.bottom() - height - 8.0).max(viewport.top()),
            width,
            height,
        )?;
        let pad = 3.0_f32.min(width.min(height) * 0.2);
        let button_width = (width - pad * 2.0).min(108.0);
        let edit = Rect::from_xywh(
            bounds.right() - pad - button_width,
            bounds.top() + pad,
            button_width,
            height - 2.0 * pad,
        )?;
        let hint = Rect::from_ltrb(
            bounds.left() + 6.0,
            bounds.top(),
            edit.left() - 6.0,
            bounds.bottom(),
        )
        .filter(|rect| rect.width() >= 24.0);
        Some(Self { bounds, edit, hint })
    }

    /// 【多选区截图】【操作栏文案】按 cn 返回与实际手势一致的说明和编辑按钮文字
    pub fn labels(cn: bool) -> (&'static str, &'static str) {
        if cn {
            (
                "左键添加　右键扣除　Ctrl+Z 撤销　Esc 返回",
                "进入编辑 (Enter)",
            )
        } else {
            (
                "Left drag: add   Right drag: subtract   Ctrl+Z: undo   Esc: back",
                "Edit (Enter)",
            )
        }
    }

    /// 【多选区截图】【操作栏绘制】按 labels、enabled 和 pressed 将面板与按钮绘入 painter，仅影响屏幕预览
    pub fn draw(
        &self,
        painter: &mut Painter<'_>,
        labels: (&str, &str),
        enabled: bool,
        pressed: bool,
        palette: Palette,
    ) {
        controls::frame(painter, self.bounds, palette.base, palette.border, 6.0);
        if let Some(rect) = self.hint {
            let text = controls::fit_label(painter, labels.0, 11.0, rect.width());
            painter.text(rect, &text, TextStyle::new(11.0, palette.text));
        }
        let label =
            controls::fit_label(painter, labels.1, 11.0, (self.edit.width() - 4.0).max(1.0));
        controls::icon_button(
            painter,
            self.edit,
            &label,
            None,
            controls::ButtonStyle {
                fill: if !enabled {
                    palette.base
                } else if pressed {
                    palette.pressed
                } else {
                    palette.accent
                },
                foreground: if !enabled {
                    palette.muted
                } else if pressed {
                    palette.accent
                } else {
                    palette.on_accent
                },
                border: if enabled {
                    palette.accent
                } else {
                    palette.border
                },
                font_size: 11.0,
                weight: cosmic_text::Weight::NORMAL,
            },
        );
    }
}
