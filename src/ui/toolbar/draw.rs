//! 【截图工具栏】【面板绘制】复用原版图标、分组细线、握柄和按钮角色

use super::{Orientation, Toolbar, ToolbarButton, ToolbarItem, icons};
use crate::{renderer::Painter, theme::Rgba, types::Finish, ui::panel::UiPanel};
use tiny_skia::{Rect, Stroke};

/// 【截图工具栏】【缓存绘制】将 toolbar 绘入 p；只在外观或缓冲尺寸变化时重建缓存
pub fn draw_toolbar(p: &mut Painter, toolbar: &mut Toolbar) {
    let Some(rect) = toolbar.rect() else { return };
    let mut cache = toolbar.toolbar_pixmap.take();
    let look = toolbar.look();
    p.panel(
        &mut cache,
        (rect.left(), rect.top()),
        toolbar.size,
        toolbar.opacity,
        toolbar.drawn_look.as_ref() != Some(&look),
        |p| content(p, toolbar),
    );
    toolbar.toolbar_pixmap = cache;
    toolbar.drawn_look = Some(look);
}

/// 【截图工具栏】【圆角描边】按 rect、radius 和 color 绘制细边，不添加尖角装饰
fn border(p: &mut Painter, rect: Rect, radius: f32, color: Rgba) {
    if let Some(path) =
        crate::renderer::paths::rounded_rect_path(&rect, radius, true, true, true, true)
    {
        p.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.0,
                ..Stroke::default()
            },
        );
    }
}

/// 【截图工具栏】【内容绘制】将 toolbar 的本地按钮、分组和握柄绘入 p
fn content(p: &mut Painter, toolbar: &Toolbar) {
    let cfg = &crate::config::get().toolbar;
    let rect = Rect::from_xywh(0.5, 0.5, toolbar.size.0 - 1.0, toolbar.size.1 - 1.0).unwrap();
    p.fill_rounded(rect, 11.0, cfg.background.get());
    border(p, rect, 11.0, crate::theme::color::border());
    let grip = toolbar.grip();
    for row in 0..3 {
        for column in 0..2 {
            let (dx, dy) = (column as f32 * 4.0 - 2.0, row as f32 * 4.0 - 4.0);
            let (dx, dy) = if toolbar.orientation == Orientation::Vertical {
                (dy, dx)
            } else {
                (dx, dy)
            };
            p.fill_circle(
                grip.left() + grip.width() / 2.0 + dx,
                grip.top() + grip.height() / 2.0 + dy,
                1.25,
                Rgba(148, 163, 184, 150),
            );
        }
    }
    for (index, item) in toolbar.items.iter().enumerate() {
        let Some(cell) = toolbar.cell(index) else {
            continue;
        };
        let ToolbarItem::Button(button) = item else {
            let separator = if toolbar.orientation == Orientation::Vertical {
                Rect::from_xywh(
                    cell.left() + (cell.width() - 16.0) / 2.0,
                    cell.top(),
                    16.0,
                    1.0,
                )
            } else {
                Rect::from_xywh(
                    cell.left(),
                    cell.top() + (cell.height() - 16.0) / 2.0,
                    1.0,
                    16.0,
                )
            };
            if let Some(separator) = separator {
                p.fill_rounded(separator, 0.0, icons::SEPARATOR);
            }
            continue;
        };
        let disabled = toolbar.disabled.contains(&index);
        let hovered = toolbar.hovered == Some(index) && !disabled;
        let selected = toolbar.selected == Some(index);
        let save = *button == ToolbarButton::SaveAs;
        let copy = *button == ToolbarButton::Finish(Finish::Copy);
        let cancel = *button == ToolbarButton::Cancel;
        let fill = if save {
            Some(if hovered {
                Rgba(16, 185, 129, 255)
            } else {
                Rgba(5, 150, 105, 255)
            })
        } else if hovered && cancel {
            Some(Rgba(239, 68, 68, 70))
        } else if hovered && copy {
            Some(Rgba(45, 212, 191, 64))
        } else if hovered {
            Some(icons::HOVER_FILL)
        } else if selected {
            Some(icons::ACTIVE_FILL)
        } else if copy {
            Some(Rgba(45, 212, 191, 34))
        } else {
            None
        };
        if let Some(fill) = fill {
            if save {
                p.fill_rounded_gradient(
                    cell,
                    6.0,
                    if hovered {
                        Rgba(52, 211, 153, 255)
                    } else {
                        Rgba(16, 185, 129, 255)
                    }
                    .into(),
                    fill.into(),
                );
            } else {
                p.fill_rounded(cell, 6.0, fill);
            }
            let edge = if save && hovered {
                Rgba(110, 231, 183, 255)
            } else if save {
                Rgba(52, 211, 153, 255)
            } else if hovered && cancel {
                Rgba(252, 165, 165, 140)
            } else if hovered && copy {
                Rgba(94, 234, 212, 150)
            } else if hovered {
                Rgba(94, 234, 212, 255)
            } else if selected {
                icons::ACTIVE_BORDER
            } else if copy {
                Rgba(45, 212, 191, 96)
            } else {
                fill
            };
            border(p, cell, 6.0, edge);
        }
        let ink = if disabled {
            cfg.icon.get().with_alpha(90)
        } else if save {
            Rgba(255, 255, 255, 255)
        } else if hovered && cancel {
            Rgba(252, 165, 165, 255)
        } else if hovered && !copy {
            icons::HOVER_INK
        } else {
            cfg.icon.get()
        };
        let size = if toolbar.secondary { 20.0 } else { 22.0 };
        let icon_rect = Rect::from_xywh(
            cell.left() + (cell.width() - size) / 2.0,
            cell.top() + (cell.height() - size) / 2.0,
            size,
            size,
        )
        .unwrap();
        if matches!(button, ToolbarButton::Tool(crate::tools::Tool::Marker)) {
            crate::features::marker::draw_icon(
                p,
                toolbar.marker_shape,
                icon_rect,
                ink.color(),
                toolbar.marker_filled,
            );
        } else {
            p.icon(
                icons::icon(*button),
                size,
                icon_rect.left(),
                icon_rect.top(),
                ink,
            );
        }
    }
}
