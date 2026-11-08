use tiny_skia::Rect;

use crate::ocr::models::{MODELS, ModelStatus};
use crate::renderer::Painter;
use crate::renderer::text::TextStyle;
use crate::theme::{Rgba, color, font, radius, stroke};
use crate::ui::icons;
use crate::ui::model_popover::{
    ModelPopover, ModelPopoverElement, ModelRow, button_geom, cfg, note_font_size, row_geom,
};
use crate::ui::panel::UiPanel;

const RECOMMENDED: &str = "Recommended for your system";

pub fn draw_model_popover(p: &mut Painter, popover: &mut ModelPopover) {
    let Some(rect) = popover.rect() else {
        return;
    };

    let mut cache = popover.pixmap.take();
    p.panel(
        &mut cache,
        (rect.left(), rect.top()),
        popover.size,
        popover.opacity,
        true,
        |p| draw_content(p, popover),
    );
    popover.pixmap = cache;
}

fn draw_content(p: &mut Painter, popover: &ModelPopover) {
    let cfg = cfg();
    let (w, h) = popover.size;
    if let Some(bg) = Rect::from_xywh(0.0, 0.0, w, h) {
        p.fill_rounded(bg, radius::panel(), cfg.background.get());
    }

    let title_x = cfg.padding + cfg.row_padding;
    let title_w = cfg.width - title_x * 2.0;
    let chinese =
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    let title = if chinese {
        "OCR · 引擎与资源"
    } else {
        "OCR · Engine and resources"
    };
    let parts: &[(&str, Rgba, f32)] = &[(title, emphasis(), font::label())];
    let mut x = title_x;
    for &(text, text_color, size) in parts {
        if let Some(rect) = Rect::from_xywh(x, cfg.padding, (title_x + title_w - x).max(1.0), 20.0)
        {
            p.text(rect, text, TextStyle::new(size, text_color.color()));
        }
        // title_gap: between the title's dot and its highlighted part
        x += p.measure(text, size) + cfg.title_gap;
    }
    if let Some(rect) = Rect::from_xywh(title_x, cfg.padding + 22.0, title_w, 18.0) {
        p.text(
            rect,
            if popover.setup.message.is_empty() {
                if chinese {
                    "自行选择下载资源；不会自动下载"
                } else {
                    "Choose downloads manually"
                }
            } else {
                &popover.setup.message
            },
            TextStyle::new(note_font_size(), color::muted().color()),
        );
    }

    for resources in [true, false] {
        if !resources && !popover.setup.show_models() {
            continue;
        }
        let area = super::layout::heading(popover, (0.0, 0.0), resources);
        let expanded = if resources {
            popover.resources_expanded
        } else {
            popover.languages_expanded
        };
        let title = if resources {
            if chinese {
                if popover.setup.runtime == ModelStatus::Installed {
                    "引擎与资源 · 已安装"
                } else {
                    "引擎与资源 · 安装管理"
                }
            } else {
                "Engine and resources"
            }
        } else if chinese {
            "其他语言模型"
        } else {
            "Other language models"
        };
        p.text(
            Rect::from_xywh(
                area.left() + 10.0,
                area.top(),
                area.width() - 76.0,
                area.height(),
            )
            .unwrap(),
            title,
            TextStyle::new(font::small(), color::foreground()),
        );
        let action = match (chinese, expanded) {
            (true, true) => "收起",
            (true, false) => "展开",
            (false, true) => "Less",
            (false, false) => "More",
        };
        p.text(
            Rect::from_xywh(area.right() - 64.0, area.top(), 54.0, area.height()).unwrap(),
            action,
            TextStyle::new(font::small(), emphasis()).center(),
        );
    }
    if popover.resources_expanded {
        super::setup::draw(p, popover);
    }
    if !popover.setup.show_models() {
        return;
    }
    for (idx, row) in popover.rows.iter().enumerate() {
        draw_row(p, popover, idx, *row, popover.hovered);
    }
}

fn draw_row(
    p: &mut Painter,
    panel: &ModelPopover,
    idx: usize,
    row: ModelRow,
    hovered: Option<ModelPopoverElement>,
) {
    let (Some(rect), Some(button), Some(model)) = (
        row_geom(panel, (0.0, 0.0), idx),
        button_geom(panel, (0.0, 0.0), idx),
        MODELS.get(idx),
    ) else {
        return;
    };
    let cfg = cfg();
    let row_hovered = hovered == Some(ModelPopoverElement::Row(idx));
    let button_hovered = hovered == Some(ModelPopoverElement::Button(idx));

    let row_selected = row.active || row.recommended;
    if row_hovered || row_selected {
        p.item_border(
            rect.left(),
            rect.top(),
            rect.width(),
            rect.height(),
            radius::item(),
            stroke::border(),
            row_hovered,
            row_selected,
        );
    }

    let left = rect.left() + cfg.row_padding;
    let text_w = (button.left() - cfg.status_width - left).max(0.0);

    let name_color = if row.active {
        emphasis()
    } else {
        color::foreground()
    };
    if let Some(name_rect) =
        Rect::from_xywh(left, rect.top() + cfg.name_top, text_w, cfg.name_height)
    {
        p.text(
            name_rect,
            model.label(
                crate::ui::language::Language::current() == crate::ui::language::Language::Chinese,
            ),
            TextStyle::new(font::label(), name_color.color()),
        );
    }

    match row.status {
        ModelStatus::Queued => p.progress_bar(left, rect.top() + cfg.bar_top, text_w, 0),
        ModelStatus::Downloading(percent) => {
            p.progress_bar(left, rect.top() + cfg.bar_top, text_w, percent)
        }
        _ => {
            if let Some(note_rect) =
                Rect::from_xywh(left, rect.top() + cfg.note_top, text_w, cfg.note_height)
            {
                let (note, note_color) = if row.recommended {
                    (RECOMMENDED, emphasis())
                } else {
                    (model.note, color::muted())
                };
                p.text(
                    note_rect,
                    note,
                    TextStyle::new(note_font_size(), note_color.color()),
                );
            }
        }
    }

    let status_text = match row.status {
        ModelStatus::Missing => size_label(row.size),
        ModelStatus::Queued => "Waiting".to_string(),
        ModelStatus::Downloading(percent) => format!("{percent}%"),
        ModelStatus::Failed => "Failed".to_string(),
        ModelStatus::Installed => String::new(),
    };
    if let Some(status_rect) = Rect::from_xywh(
        button.left() - cfg.status_width,
        rect.top(),
        cfg.status_width,
        cfg.row_height,
    ) {
        p.text(
            status_rect,
            &status_text,
            TextStyle::new(note_font_size(), color::muted().color()).center(),
        );
    }

    let checked = row.active && row.status == ModelStatus::Installed;
    let icon = match row.status {
        ModelStatus::Missing => icons::DOWNLOAD,
        ModelStatus::Failed => icons::RETRY,
        ModelStatus::Queued | ModelStatus::Downloading(_) => icons::CLOSE,
        ModelStatus::Installed if checked => icons::CHECK,
        ModelStatus::Installed if row_hovered || button_hovered => icons::TRASH,
        ModelStatus::Installed => return,
    };

    if !checked && button_hovered {
        p.item_border(
            button.left(),
            button.top(),
            cfg.button_size,
            cfg.button_size,
            radius::item(),
            stroke::border(),
            true,
            false,
        );
    }
    let tint = if checked {
        emphasis()
    } else if button_hovered {
        color::hover()
    } else {
        color::foreground()
    };
    p.icon(
        icon,
        cfg.icon_size,
        button.left() + (cfg.button_size - cfg.icon_size) / 2.0,
        button.top() + (cfg.button_size - cfg.icon_size) / 2.0,
        tint.usvg(),
    );
}

fn size_label(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

/// 【识别模型】【强调文字】返回当前主题的不透明强调色，标题和选中状态不能使用半透明背景填色，无参数
fn emphasis() -> Rgba {
    crate::theme::context_menu::Palette::current().accent
}
