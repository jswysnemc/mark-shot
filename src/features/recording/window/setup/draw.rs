//! 【录制设置】【界面绘制】标题、分段行、开关、提示与操作区分层绘制，不读取视频数据

use super::{
    super::summary,
    Setup,
    layout::{self, Control, PAD, rect},
    options::{self, Row},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{Rgba, window::Palette},
    ui::{controls, icons},
};

/// 提示区单行高度
const NOTICE_LINE: f32 = 14.0;

/// 一行分段选项的绘制数据：行、各段文字、选中段、是否可用
type RowView = (Row, Vec<String>, usize, bool);

/// 【录制设置】【内容绘制】按 state、canvas 与 scale 绘制完整准备窗口，无返回值
pub(super) fn draw(state: &mut Setup, canvas: &mut tiny_skia::Pixmap, scale: f32) {
    let cn = state.chinese();
    let text = |a: &'static str, b: &'static str| if cn { a } else { b };
    let size = state.size;
    // 1. 【录制设置】【显示数据】先从选项模型取出全部文字，绘制阶段不再访问配置
    let rows: Vec<RowView> = Row::ALL
        .into_iter()
        .map(|row| {
            (
                row,
                options::choices(row, &state.config, &state.target, cn),
                options::selected(row, &state.config, &state.target),
                options::enabled(row, &state.config),
            )
        })
        .collect();
    let labels = Labels {
        title: text("屏幕录制", "Screen recording"),
        cursor: text("鼠标指针", "Cursor"),
        settings: text("更多设置", "More settings"),
        start: text("开始录制", "Start recording"),
        copy: text("复制详情", "Copy details"),
    };
    let failed = state.failure.is_some();
    let notice: Vec<String> = match &state.failure {
        Some(failure) => failure.lines().map(str::to_owned).collect(),
        None => vec![
            summary::target(&state.target, cn),
            text(
                "首次录制可能出现系统授权窗口，请选择对应的显示器",
                "The system may ask for permission; choose the matching display",
            )
            .into(),
        ],
    };
    // 2. 【录制设置】【缺失字形】只准备本帧实际绘制的文字
    let mut glyphs = format!(
        "{}{}{}{}{}",
        labels.title, labels.cursor, labels.settings, labels.start, labels.copy
    );
    for (row, choices, ..) in &rows {
        glyphs.push_str(row.label(cn));
        choices.iter().for_each(|choice| glyphs.push_str(choice));
    }
    notice.iter().for_each(|line| glyphs.push_str(line));
    let hover = layout::hit(state, state.pointer);
    let items = layout::controls(state);
    let cursor = state.config.cursor;
    let palette = Palette::current();
    canvas.fill(palette.window.color());
    let mut shared = state.fonts.borrow_mut();
    let fonts = &mut *shared;
    fonts.system.ensure_text(&glyphs);
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_icons(&state.icons)
        .with_text(&mut fonts.system, &mut fonts.cache);
    header(&mut painter, size, labels.title, hover, palette);
    // 3. 【录制设置】【分段选项】每行一个连续的单选组，选中项使用强调色
    for (index, (row, choices, selected, enabled)) in rows.iter().enumerate() {
        painter.text(
            layout::label(index),
            row.label(cn),
            TextStyle::new(11.0, palette.muted),
        );
        let choices: Vec<&str> = choices.iter().map(String::as_str).collect();
        controls::segmented(
            &mut painter,
            layout::field(size, index),
            &choices,
            Some(*selected),
            match hover {
                Some(Control::Choice(hovered, segment)) if hovered == *row => Some(segment),
                _ => None,
            },
            *enabled,
        );
    }
    // 4. 【录制设置】【光标开关】布尔选项使用开关，整行都是点击目标
    painter.text(
        layout::label(rows.len()),
        labels.cursor,
        TextStyle::new(11.0, palette.muted),
    );
    controls::switch(
        &mut painter,
        layout::field(size, rows.len()),
        cursor,
        hover == Some(Control::Cursor),
    );
    // 5. 【录制设置】【提示与错误】失败详情折行后按可见高度显示，滚轮浏览其余行，完整内容可复制
    let area = layout::notice(size);
    let width = area.width() - if failed { 84.0 } else { 0.0 };
    let lines = wrap(&mut painter, &notice, 10.0, width);
    let visible = (area.height() / NOTICE_LINE).max(1.0) as usize;
    state.notice_scroll = state.notice_scroll.min(lines.len().saturating_sub(visible));
    for (index, line) in lines
        .iter()
        .skip(state.notice_scroll)
        .take(visible)
        .enumerate()
    {
        painter.text(
            rect(
                area.left(),
                area.top() + index as f32 * NOTICE_LINE,
                width,
                NOTICE_LINE,
            ),
            line,
            TextStyle::new(
                10.0,
                if failed {
                    palette.error
                } else if index == 0 && state.notice_scroll == 0 {
                    palette.text
                } else {
                    palette.muted
                },
            ),
        );
    }
    footer(&mut painter, size, &items, hover, &labels, palette);
    crate::ui::window_frame::draw(painter.canvas(), scale);
}

/// 准备窗口的固定文案
struct Labels {
    title: &'static str,
    cursor: &'static str,
    settings: &'static str,
    start: &'static str,
    copy: &'static str,
}

/// 【录制设置】【标题栏】用 painter 按 size 绘制图标、title 与关闭按钮，hover 决定关闭按钮高亮，无返回值
fn header(
    painter: &mut Painter<'_>,
    size: (u32, u32),
    title: &str,
    hover: Option<Control>,
    palette: Palette,
) {
    painter.icon(icons::VIDEO, 18.0, PAD, 13.0, palette.accent);
    painter.text(
        rect(PAD + 26.0, 6.0, size.0 as f32 - PAD * 2.0 - 70.0, 32.0),
        title,
        TextStyle::new(13.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
    );
    let close = rect(size.0 as f32 - 40.0, 8.0, 28.0, 28.0);
    if hover == Some(Control::Close) {
        painter.fill_rounded(close, 6.0, palette.alternate);
    }
    painter.icon(
        icons::CLOSE,
        16.0,
        close.left() + 6.0,
        close.top() + 6.0,
        if hover == Some(Control::Close) {
            palette.text
        } else {
            palette.muted
        },
    );
}

/// 【录制设置】【操作区】用 painter 按 size 与 items 绘制分隔线、次要入口、主操作与错误复制入口，无返回值
fn footer(
    painter: &mut Painter<'_>,
    size: (u32, u32),
    items: &[(Control, tiny_skia::Rect)],
    hover: Option<Control>,
    labels: &Labels,
    palette: Palette,
) {
    painter.fill_rounded(
        rect(PAD, layout::footer(size), size.0 as f32 - PAD * 2.0, 1.0),
        0.5,
        palette.border,
    );
    for (control, bounds) in items.iter().copied() {
        let hovered = hover == Some(control);
        let (label, style) = match control {
            // 1. 【录制设置】【主操作】开始录制是唯一的实心强调按钮，其余入口保持低视觉权重
            Control::Start => (
                labels.start,
                controls::ButtonStyle {
                    fill: if hovered {
                        shade(palette.accent, 0.92)
                    } else {
                        palette.accent
                    },
                    foreground: palette.on_accent,
                    border: palette.accent,
                    font_size: 12.0,
                    weight: cosmic_text::Weight::SEMIBOLD,
                },
            ),
            Control::Settings => (
                labels.settings,
                controls::ButtonStyle {
                    fill: if hovered {
                        palette.alternate
                    } else {
                        palette.window
                    },
                    foreground: palette.text,
                    border: if hovered {
                        palette.alternate
                    } else {
                        palette.border
                    },
                    font_size: 11.0,
                    weight: cosmic_text::Weight::NORMAL,
                },
            ),
            Control::Copy => (
                labels.copy,
                controls::ButtonStyle {
                    fill: if hovered {
                        palette.alternate
                    } else {
                        palette.window
                    },
                    foreground: palette.text,
                    border: palette.border,
                    font_size: 10.0,
                    weight: cosmic_text::Weight::NORMAL,
                },
            ),
            _ => continue,
        };
        controls::icon_button(painter, bounds, label, None, style);
    }
}

/// 【录制设置】【颜色加深】按 factor 缩放 color 的明度，返回悬停使用的颜色，透明度不变
fn shade(color: Rgba, factor: f32) -> Rgba {
    let scale = |value: u8| (f32::from(value) * factor).round().clamp(0.0, 255.0) as u8;
    Rgba(scale(color.0), scale(color.1), scale(color.2), color.3)
}

/// 【录制设置】【文字折行】用 painter 把 lines 按 size 和 width 折成不超宽的行，返回显示行；按字符边界切分
fn wrap(painter: &mut Painter<'_>, lines: &[String], size: f32, width: f32) -> Vec<String> {
    let mut wrapped = Vec::new();
    for line in lines {
        let mut current = String::new();
        for character in line.chars() {
            current.push(character);
            if painter.measure(&current, size) > width && current.chars().count() > 1 {
                current.pop();
                wrapped.push(std::mem::take(&mut current));
                current.push(character);
            }
        }
        wrapped.push(current);
    }
    wrapped
}
