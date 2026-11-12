//! 【截图起始】【顶部栏绘制】截图入口、录制状态与控制按钮共用一行，消息条只在没有活动录制时显示

use super::{
    Control, State,
    layout::{self, rect},
    text,
};
use crate::{
    features::recording::Format,
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::{controls, icons},
};

/// 【截图起始】【顶部栏】用 painter 按 start 绘制入口、录制状态与控制按钮，无返回值
///
/// 参数：painter 目标画布、start 起始界面状态、monitor 当前输出序号、
/// size 输出逻辑尺寸、cn 是否使用中文、palette 当前窗口配色
pub(super) fn draw(
    painter: &mut Painter<'_>,
    start: &State,
    monitor: usize,
    size: (i32, i32),
    cn: bool,
    palette: Palette,
) {
    let active = start.active();
    let paused = start.status.phase == "paused";
    let top = layout::top(size, active);
    controls::frame(painter, top, palette.base, palette.border, 8.0);
    // 1. 【截图起始】【录制状态】活动录制在栏内显示状态点、类型、阶段和时长
    if active {
        let area = layout::status(size);
        painter.fill_rounded(
            rect(
                area.left() - 7.0,
                top.top() + 10.0,
                1.0,
                top.height() - 20.0,
            ),
            0.5,
            palette.border,
        );
        painter.fill_circle(
            area.left() + 4.0,
            top.top() + top.height() / 2.0,
            4.0,
            match start.status.phase.as_str() {
                "recording" => palette.error.color(),
                "paused" => palette.muted.color(),
                _ => palette.accent.color(),
            },
        );
        let bounds = rect(
            area.left() + 14.0,
            area.top(),
            area.width() - 14.0,
            area.height(),
        );
        let label = text::status(&start.status, cn);
        let label = controls::fit_label(painter, &label, 11.0, bounds.width());
        painter.text(bounds, &label, TextStyle::new(11.0, palette.text));
    }
    // 2. 【截图起始】【入口与控制】同一循环绘制全部按钮，可用状态与输入处理使用相同判断
    let selected = match start.recording {
        Some(Format::Video) => Control::Video,
        Some(Format::Gif) => Control::Gif,
        None => Control::Region,
    };
    for (control, bounds) in layout::controls(size, active) {
        let enabled = match control {
            Control::Video | Control::Gif => start.can_record(),
            Control::Pause => start.can_pause(),
            Control::Stop => start.can_stop(),
            _ => true,
        };
        let chosen = control == selected;
        let hover = start.hovered == Some((monitor, control));
        let icon = match control {
            Control::Pause if paused => Some(icons::SCROLL_RESUME),
            Control::Pause => Some(icons::SCROLL_PAUSE),
            Control::Stop => Some(icons::SAVE),
            _ => None,
        };
        controls::icon_button(
            painter,
            bounds,
            text::label(control, paused, cn),
            icon,
            controls::ButtonStyle {
                fill: if chosen {
                    palette.pressed
                } else if hover && enabled {
                    palette.alternate
                } else {
                    palette.base
                },
                foreground: if !enabled {
                    palette.muted
                } else if chosen {
                    palette.accent
                } else {
                    palette.text
                },
                border: if chosen { palette.accent } else { palette.base },
                font_size: 11.0,
                weight: cosmic_text::Weight::NORMAL,
            },
        );
    }
    // 3. 【截图起始】【消息条】录制窗口占用、保存结果和查询失败显示在入口下方，不拦截选区
    if start.message_visible() {
        let bar = layout::message(size);
        controls::frame(painter, bar, palette.base, palette.border, 6.0);
        let bounds = rect(
            bar.left() + 10.0,
            bar.top(),
            bar.width() - 20.0,
            bar.height(),
        );
        let label = text::message(&start.status, &start.message, cn);
        let label = controls::fit_label(painter, &label, 10.0, bounds.width());
        painter.text(
            bounds,
            &label,
            TextStyle::new(
                10.0,
                if start.status.phase == "failed" {
                    palette.error
                } else {
                    palette.text
                },
            ),
        );
    }
}

/// 【截图起始】【字形文本】按 start 和 cn 返回顶部栏本帧会绘制的全部文字，供调用方预先准备缺失字形
pub(super) fn glyphs(start: &State, cn: bool) -> String {
    let paused = start.status.phase == "paused";
    let mut value = String::new();
    for control in [
        Control::Region,
        Control::Fullscreen,
        Control::Video,
        Control::Gif,
        Control::Pause,
        Control::Stop,
    ] {
        value.push_str(text::label(control, paused, cn));
    }
    value.push_str(&text::status(&start.status, cn));
    value.push_str(&text::message(&start.status, &start.message, cn));
    value
}
