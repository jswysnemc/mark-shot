//! 【文件选择】【绘制】共用主题的目录、单行字段与覆盖确认

use super::{
    layout::Control,
    state::{Field, Picker},
};
use crate::ui::fonts::UiFonts;
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{color, font},
    ui::controls,
};
use cosmic_text::SwashCache;
use tiny_skia::{Pixmap, Rect};

/// 【文件选择】【文本区域】返回 x、y、width、height 的矩形；布局保证尺寸有效
fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width, height).unwrap()
}

/// 【文件选择】【对话框绘制】用 fonts、cache 把 picker 绘制到 canvas；scale 为物理缓冲比例
pub fn draw(
    picker: &mut Picker,
    fonts: &mut UiFonts,
    cache: &mut SwashCache,
    canvas: &mut Pixmap,
    scale: f32,
) {
    let rows = picker.clamp_scroll();
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(fonts, cache);
        let panel = picker.layout.panel;
        painter.fill_rounded(panel, 6.0, color::panel());
        painter.panel_border(
            panel.x(),
            panel.y(),
            panel.width(),
            panel.height(),
            6.0,
            1.0,
        );
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.y() + 7.0,
                panel.width() - 24.0,
                26.0,
            ),
            "Save Screenshot / PNG",
            TextStyle::new(font::label(), color::foreground()),
        );
        for &(control, bounds) in &picker.layout.buttons {
            let label = match control {
                Control::Home => "Home",
                Control::Up => "Up",
                Control::Hidden => "Hidden",
                Control::NewFolder => "New folder",
                Control::Cancel => "Cancel",
                Control::Save if picker.creating_folder => "Create",
                Control::Save => "Save",
                _ => "",
            };
            controls::button(
                &mut painter,
                bounds,
                label,
                control == Control::Hidden && picker.hidden,
                false,
                !picker.busy
                    && (!matches!(control, Control::Save | Control::NewFolder)
                        || picker.pending.is_none()),
            );
        }
        for field in [picker.layout.path, picker.layout.filename] {
            painter.fill_rounded(field, 4.0, color::field());
        }
        painter.fill_rounded(picker.layout.list, 4.0, color::field());
        for (row, item) in picker
            .listing
            .iter()
            .skip(picker.scroll)
            .take(rows)
            .enumerate()
        {
            let bounds = rect(
                picker.layout.list.x() + 4.0,
                picker.layout.list.y() + row as f32 * 28.0,
                picker.layout.list.width() - 8.0,
                26.0,
            );
            let selected = picker.selected == Some(picker.scroll + row);
            if selected {
                painter.fill_rounded(bounds, 3.0, color::active());
            }
            let foreground = if selected {
                color::on_active()
            } else {
                color::foreground()
            };
            painter.text(
                rect(bounds.x() + 4.0, bounds.y(), 36.0, 26.0),
                if item.directory { "DIR" } else { "PNG" },
                TextStyle::new(
                    font::small(),
                    if selected { foreground } else { color::muted() },
                ),
            );
            let label = controls::fit_label(
                &mut painter,
                &item.name.to_string_lossy(),
                font::label(),
                bounds.width() - 50.0,
            );
            painter.text(
                rect(bounds.x() + 46.0, bounds.y(), bounds.width() - 50.0, 26.0),
                &label,
                TextStyle::new(font::label(), foreground),
            );
        }
        if picker.listing.is_empty() {
            painter.text(
                picker.layout.list,
                if picker.pending.is_some() {
                    "Loading folder..."
                } else {
                    "No PNG files in this folder"
                },
                TextStyle::new(font::small(), color::muted()).center(),
            );
        }
        let status = picker
            .status
            .as_deref()
            .unwrap_or("Ctrl+L: folder / Alt+Up: parent / PNG only");
        let label = controls::fit_label(&mut painter, status, font::small(), panel.width() - 24.0);
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.bottom() - 110.0,
                panel.width() - 24.0,
                22.0,
            ),
            &label,
            TextStyle::new(font::small(), color::muted()),
        );
        painter.text(
            rect(panel.x() + 12.0, panel.bottom() - 80.0, 40.0, 28.0),
            if picker.creating_folder {
                "Folder"
            } else {
                "Name"
            },
            TextStyle::new(font::small(), color::muted()),
        );
    }
    picker.path.draw(
        fonts,
        cache,
        canvas,
        picker.layout.path_text,
        scale,
        picker.focus == Some(Field::Path) && picker.overwrite.is_none() && !picker.busy,
    );
    picker.filename.draw(
        fonts,
        cache,
        canvas,
        picker.layout.filename_text,
        scale,
        picker.focus == Some(Field::Filename) && picker.overwrite.is_none() && !picker.busy,
    );
    if let Some((target, _)) = &picker.overwrite {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(fonts, cache);
        let panel = picker.layout.confirm;
        painter.fill_rounded(panel, 6.0, color::panel());
        painter.panel_border(
            panel.x(),
            panel.y(),
            panel.width(),
            panel.height(),
            6.0,
            1.0,
        );
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.y() + 10.0,
                panel.width() - 24.0,
                24.0,
            ),
            "Replace existing file?",
            TextStyle::new(font::label(), color::foreground()),
        );
        let name = controls::fit_label(
            &mut painter,
            &target.file_name().unwrap_or_default().to_string_lossy(),
            font::small(),
            panel.width() - 24.0,
        );
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.y() + 42.0,
                panel.width() - 24.0,
                24.0,
            ),
            &name,
            TextStyle::new(font::small(), color::muted()),
        );
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.y() + 68.0,
                panel.width() - 24.0,
                24.0,
            ),
            "The existing file will be replaced.",
            TextStyle::new(font::small(), color::muted()),
        );
        for &(control, bounds) in &picker.layout.confirm_buttons {
            controls::button(
                &mut painter,
                bounds,
                if control == Control::ConfirmCancel {
                    "Cancel"
                } else {
                    "Replace"
                },
                (control == Control::ConfirmReplace) == picker.confirm_replace,
                false,
                true,
            );
        }
    }
}
