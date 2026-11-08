//! 【翻译设置】【原生绘制】服务卡片、分组参数与原生编辑器沿用真实配置和异步目录

use super::super::{widgets, window::Preferences};
use super::{
    fields::{Field, SERVICES},
    layout::{Hit, Layout, rect},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use tiny_skia::Pixmap;

/// 【翻译设置】【页面绘制】将 state 按 scale 绘入 canvas；密钥默认遮蔽，滚动内容不覆盖导航和状态栏
pub(in crate::features::preferences) fn draw(
    state: &mut Preferences,
    canvas: &mut Pixmap,
    scale: f32,
) {
    let layout = Layout::new(state);
    let chinese = state.chinese();
    let palette = Palette::current();
    let hover = layout.hit(state.pointer);
    let expanded = state.dropdown.as_ref().map(|menu| menu.target);
    let values = state
        .translation
        .fields()
        .into_iter()
        .map(|field| {
            let value = state.translation.value(field);
            let text = if field.secret() && !value.as_str().unwrap_or_default().is_empty() {
                "••••••••".into()
            } else {
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string())
            };
            let text = if field == Field::Target {
                let language = if chinese {
                    crate::translation::language::UiLanguage::Chinese
                } else {
                    crate::translation::language::UiLanguage::English
                };
                let canonical = crate::translation::language::canonical(&text);
                let stable = crate::translation::language::OPTIONS
                    .iter()
                    .find(|option| {
                        canonical.is_some()
                            && crate::translation::language::canonical(option) == canonical
                    })
                    .copied()
                    .unwrap_or(&text);
                crate::translation::language::display_for(stable, language).to_owned()
            } else {
                text
            };
            (field, text)
        })
        .collect::<Vec<_>>();
    let labels = layout
        .rows
        .iter()
        .map(|row| {
            let (title, help) = row.field.caption(chinese);
            format!("{title} {help}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let service_labels = SERVICES
        .iter()
        .map(|service| {
            let (title, detail) = service.caption(chinese);
            format!("{title} {detail}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let panel_labels = layout
        .panels
        .iter()
        .map(|(_, title)| *title)
        .collect::<Vec<_>>()
        .join(" ");
    state.fonts.ensure_text(&format!("{labels} {service_labels} {panel_labels} 查询模型 取消查询 选择模型 保存 显示 隐藏 使用默认值 {}", values.iter().map(|(_, text)| text.as_str()).collect::<Vec<_>>().join(" ")));
    let mut editor_rect = None;
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_icons(&state.icons)
            .with_text(&mut state.fonts, &mut state.cache);
        painter.clipped(Some(layout.viewport), |painter| {
            for (bounds, title) in &layout.panels {
                widgets::panel(painter, *bounds, title);
            }
            for row in &layout.rows {
                let (title, help) = row.field.caption(chinese);
                let title = controls::fit_label(painter, title, 12.0, row.label.width());
                let help = controls::fit_label(painter, help, 10.0, row.help.width());
                painter.text(row.label, &title, TextStyle::new(12.0, palette.text));
                painter.text(row.help, &help, TextStyle::new(10.0, palette.muted));
            }
            for (hit, bounds) in &layout.controls {
                let hovered = hover == Some(*hit) || state.translation.focus == Some(*hit);
                let mut enabled = true;
                let label: String = match *hit {
                    Hit::Enable => {
                        if state.translation.root["provider"] == state.translation.service().id {
                            if chinese {
                                "当前已启用"
                            } else {
                                "Currently enabled"
                            }
                        } else {
                            if chinese {
                                "启用此服务"
                            } else {
                                "Enable service"
                            }
                        }
                        .into()
                    }
                    Hit::Auto => if state.translation.root["provider"] == "auto" {
                        if chinese {
                            "当前：自动选择"
                        } else {
                            "Current: automatic"
                        }
                    } else {
                        if chinese {
                            "使用自动选择"
                        } else {
                            "Use automatic selection"
                        }
                    }
                    .into(),
                    Hit::Field(field) => {
                        let masked = state
                            .translation
                            .editing
                            .as_ref()
                            .map(|(_, editor)| editor.masked_text())
                            .unwrap_or_default();
                        if state
                            .translation
                            .editing
                            .as_ref()
                            .is_some_and(|(current, _)| *current == field)
                        {
                            widgets::field(
                                painter,
                                *bounds,
                                if field.secret() && !state.translation.reveal {
                                    &masked
                                } else {
                                    ""
                                },
                                true,
                                false,
                            );
                            if !field.secret() || state.translation.reveal {
                                editor_rect = Some(rect(
                                    bounds.left() + 8.0,
                                    bounds.top() + 3.0,
                                    bounds.width() - 16.0,
                                    bounds.height() - 6.0,
                                ));
                            }
                        } else {
                            let text = values
                                .iter()
                                .find(|(current, _)| *current == field)
                                .map(|(_, text)| text.as_str())
                                .unwrap_or_default();
                            if matches!(field, Field::Filter | Field::Disable) {
                                widgets::switch(painter, *bounds, text == "true", hovered);
                            } else if matches!(field, Field::Provider | Field::Target) {
                                widgets::select(
                                    painter,
                                    *bounds,
                                    text,
                                    hovered,
                                    expanded
                                        == Some(super::super::dropdown::Target::Translation(field)),
                                );
                            } else {
                                widgets::field(
                                    painter,
                                    *bounds,
                                    if text.is_empty() {
                                        if chinese {
                                            "使用默认值"
                                        } else {
                                            "Use default"
                                        }
                                    } else {
                                        text
                                    },
                                    hovered,
                                    false,
                                );
                            }
                        }
                        continue;
                    }
                    Hit::Apply => if chinese { "保存" } else { "Save" }.into(),
                    Hit::Reveal => match (chinese, state.translation.reveal) {
                        (true, true) => "隐藏",
                        (true, false) => "显示",
                        (false, true) => "Hide",
                        (false, false) => "Show",
                    }
                    .into(),
                    Hit::Query => match (chinese, state.translation.task.is_some()) {
                        (true, true) => "取消查询",
                        (true, false) => "查询模型",
                        (false, true) => "Cancel query",
                        (false, false) => "Fetch models",
                    }
                    .into(),
                    Hit::Models => {
                        enabled = !state.translation.models.is_empty();
                        if enabled {
                            widgets::select(
                                painter,
                                *bounds,
                                &format!(
                                    "{} ({})",
                                    if chinese {
                                        "选择模型"
                                    } else {
                                        "Select model"
                                    },
                                    state.translation.models.len()
                                ),
                                hovered,
                                expanded
                                    == Some(super::super::dropdown::Target::Translation(
                                        Field::Model,
                                    )),
                            );
                            continue;
                        }
                        format!(
                            "{} ({})",
                            if chinese {
                                "选择模型"
                            } else {
                                "Select model"
                            },
                            state.translation.models.len()
                        )
                    }
                };
                widgets::button(painter, *bounds, &label, false, hovered, enabled);
            }
        });
        if let Some(bar) = &layout.scrollbar {
            painter.fill_rounded(bar.track, 4.0, palette.alternate);
            painter.fill_rounded(bar.handle, 3.0, palette.muted);
        }
    }
    if let Some(rect) = editor_rect
        && rect.top() >= layout.viewport.top()
        && rect.bottom() <= layout.viewport.bottom()
        && let Some((_, editor)) = &mut state.translation.editing
    {
        editor.draw(
            &mut state.fonts,
            &mut state.cache,
            canvas,
            rect,
            scale,
            !state.search_focus,
        );
    }
}
