//! 【设置窗口】【窗口外壳】统一紧凑顶栏、图标导航、页面标题与真实保存状态

use super::{
    catalog::Page,
    layout::{self, rect},
    sections,
    window::{Control, Preferences},
};
use crate::{
    renderer::{Icons, Painter, text::TextStyle},
    theme::window::Palette,
    ui::{controls, icons},
};
use tiny_skia::Pixmap;

/// 【设置窗口】【图标初始化】无参数，返回导航和窗口所用 SVG 缓存，只在创建窗口时解析
pub(super) fn icons() -> Icons {
    Page::ALL
        .iter()
        .copied()
        .map(sections::icon)
        .chain([
            icons::CLOSE,
            icons::MAGNIFIER,
            icons::CHECK,
            sections::PANEL,
            icons::APPLICATION,
        ])
        .filter_map(|svg| {
            usvg::Tree::from_str(svg, &usvg::Options::default())
                .ok()
                .map(|tree| (svg, tree))
        })
        .collect()
}

/// 【设置窗口】【外壳绘制】把 state 的导航与状态按 scale 绘入 canvas，内容页单独绘制
pub(super) fn draw(state: &mut Preferences, canvas: &mut Pixmap, scale: f32) {
    let palette = Palette::current();
    let chinese = state.chinese();
    let sidebar = layout::sidebar(state);
    let search = layout::search_rect(state);
    let close = layout::close_rect(state);
    let hover = state.hit(state.pointer);
    let navigation: Vec<_> = Page::ALL
        .into_iter()
        .enumerate()
        .map(|(index, page)| (page, layout::nav_rect(state, index)))
        .collect();
    let title = if state.search.text().is_empty() {
        state.page.title(chinese)
    } else if chinese {
        "搜索结果"
    } else {
        "Search results"
    };
    let description = if state.search.text().is_empty() {
        sections::description(state.page, chinese)
    } else if chinese {
        "跨分类查找；点击右侧控件修改设置"
    } else {
        "Find settings across categories and edit their controls"
    };
    let path = state.path.to_string_lossy().into_owned();
    let path_bounds = layout::path_rect(state);
    let status = if state.status.is_empty() {
        if chinese {
            "修改后自动保存 · Enter 提交 · Esc 取消编辑"
        } else {
            "Changes save on leaving a field · Enter saves · Esc cancels"
        }
    } else {
        &state.status
    }
    .to_owned();
    state.fonts.ensure_text(&format!(
        "{title} {description} {status} {path} {} 设置 搜索设置 逐项保存到本机 配置路径 已保存",
        Page::ALL
            .iter()
            .map(|page| page.title(chinese))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    canvas.fill(palette.window.color());
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_icons(&state.icons)
            .with_text(&mut state.fonts, &mut state.cache);
        painter.fill_rounded(rect(0.0, 0.0, state.size.0 as f32, 48.0), 0.0, palette.base);
        painter.fill_rounded(
            rect(0.0, 48.0, sidebar, state.size.1 as f32 - 80.0),
            0.0,
            palette.window,
        );
        painter.fill_rounded(
            rect(0.0, 47.0, state.size.0 as f32, 1.0),
            0.0,
            palette.alternate,
        );
        painter.fill_rounded(
            rect(sidebar, 48.0, 1.0, state.size.1 as f32 - 80.0),
            0.0,
            palette.alternate,
        );
        if let Some(logo) = state.icons.get(icons::APPLICATION) {
            resvg::render(
                logo,
                tiny_skia::Transform::from_scale(30.0 * scale / 128.0, 30.0 * scale / 128.0)
                    .post_translate(11.0 * scale, 9.0 * scale),
                &mut painter.canvas().as_mut(),
            );
        }
        painter.text(
            rect(46.0, 9.0, sidebar - 50.0, 30.0),
            "Mark Shot",
            TextStyle::new(12.0, palette.text).weight(cosmic_text::Weight::BOLD),
        );
        for (page, bounds) in navigation {
            let active = state.page == page && state.search.text().is_empty();
            if active || hover == Some(Control::Page(page)) {
                painter.fill_rounded(
                    bounds,
                    7.0,
                    if active {
                        palette.pressed
                    } else {
                        palette.alternate
                    },
                );
            }
            let color = if active {
                palette.accent
            } else {
                palette.more_icon
            };
            painter.icon(
                sections::icon(page),
                16.0,
                bounds.left() + 10.0,
                bounds.top() + 8.0,
                color,
            );
            painter.text(
                rect(
                    bounds.left() + 36.0,
                    bounds.top(),
                    bounds.width() - 40.0,
                    bounds.height(),
                ),
                page.title(chinese),
                TextStyle::new(12.0, color).weight(if active {
                    cosmic_text::Weight::SEMIBOLD
                } else {
                    cosmic_text::Weight::NORMAL
                }),
            );
        }
        if state.size.1 >= 460 {
            painter.fill_rounded(
                rect(0.0, state.size.1 as f32 - 70.0, sidebar, 1.0),
                0.0,
                palette.alternate,
            );
            painter.fill_rounded(
                rect(14.0, state.size.1 as f32 - 53.0, 6.0, 6.0),
                3.0,
                palette.accent,
            );
            painter.text(
                rect(26.0, state.size.1 as f32 - 63.0, sidebar - 36.0, 26.0),
                if chinese {
                    "逐项保存到本机"
                } else {
                    "Saved locally"
                },
                TextStyle::new(10.0, palette.muted),
            );
        }
        controls::frame(
            &mut painter,
            search,
            palette.window,
            if state.search_focus {
                palette.accent
            } else {
                palette.border
            },
            7.0,
        );
        painter.icon(
            icons::MAGNIFIER,
            14.0,
            search.left() + 10.0,
            search.top() + 8.0,
            palette.muted,
        );
        if state.search.text().is_empty() && !state.search_focus {
            painter.text(
                rect(
                    search.left() + 31.0,
                    search.top(),
                    search.width() - 94.0,
                    search.height(),
                ),
                if chinese {
                    "搜索设置…"
                } else {
                    "Search settings…"
                },
                TextStyle::new(11.0, palette.muted),
            );
        }
        if !state.search_focus && state.search.text().is_empty() {
            let badge = rect(search.right() - 52.0, search.top() + 5.0, 44.0, 20.0);
            controls::frame(&mut painter, badge, palette.base, palette.border, 4.0);
            painter.text(
                badge,
                "Ctrl+F",
                TextStyle::new(10.0, palette.muted).center(),
            );
        }
        if hover == Some(Control::Close) {
            painter.fill_rounded(close, 6.0, palette.alternate);
        }
        painter.icon(
            icons::CLOSE,
            16.0,
            close.left() + 7.0,
            close.top() + 7.0,
            palette.muted,
        );
        painter.text(
            rect(
                sidebar + 20.0,
                68.0,
                state.size.0 as f32 - sidebar - 40.0,
                24.0,
            ),
            title,
            TextStyle::new(16.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
        );
        let description = controls::fit_label(
            &mut painter,
            description,
            11.0,
            state.size.0 as f32 - sidebar - 40.0,
        );
        painter.text(
            rect(
                sidebar + 20.0,
                95.0,
                state.size.0 as f32 - sidebar - 40.0,
                20.0,
            ),
            &description,
            TextStyle::new(11.0, palette.muted),
        );
        painter.fill_rounded(
            rect(0.0, state.size.1 as f32 - 32.0, state.size.0 as f32, 32.0),
            0.0,
            palette.base,
        );
        painter.fill_rounded(
            rect(0.0, state.size.1 as f32 - 32.0, state.size.0 as f32, 1.0),
            0.0,
            palette.alternate,
        );
        let path = controls::fit_label(
            &mut painter,
            &format!("{}: {path}", if chinese { "配置路径" } else { "Config" }),
            10.0,
            path_bounds.width(),
        );
        painter.text(
            path_bounds,
            &path,
            TextStyle::new(
                10.0,
                if hover == Some(Control::ConfigPath) {
                    palette.accent
                } else {
                    palette.muted
                },
            ),
        );
        let bounds = rect(
            path_bounds.right() + 14.0,
            state.size.1 as f32 - 29.0,
            state.size.0 as f32 - path_bounds.right() - 30.0,
            26.0,
        );
        let status = controls::fit_label(&mut painter, &status, 10.0, bounds.width());
        painter.text(bounds, &status, TextStyle::new(10.0, palette.muted));
    }
    if state.search_focus || !state.search.text().is_empty() {
        state.search.draw(
            &mut state.fonts,
            &mut state.cache,
            canvas,
            rect(
                search.left() + 31.0,
                search.top() + 3.0,
                search.width() - 42.0,
                24.0,
            ),
            scale,
            state.search_focus,
        );
    }
}
