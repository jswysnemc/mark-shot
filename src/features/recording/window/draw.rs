//! 【录制窗口】【状态绘制】统一按钮、细边框和可滚动错误正文

use super::{
    Recorder,
    layout::{self, Control, rect},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};

/// 【录制窗口】【内容绘制】按 state、canvas、scale 绘制状态与选项，不读取视频数据
pub(super) fn draw(state: &mut Recorder, canvas: &mut tiny_skia::Pixmap, scale: f32) {
    let cn = state.chinese();
    let text = |a: &str, b: &str| if cn { a.to_owned() } else { b.to_owned() };
    let phase = match state.status.phase.as_str() {
        "countdown" => text("即将开始", "Starting in"),
        "starting" => text("等待捕获授权与编码器", "Waiting for capture and encoder"),
        "recording" => text("正在录制", "Recording"),
        "paused" => text("已暂停", "Paused"),
        "saving" => text("正在保存", "Saving"),
        "finished" => text("已保存", "Saved"),
        "failed" => text("录制失败", "Recording failed"),
        "cancelled" => text("已取消", "Cancelled"),
        _ => text("准备录制", "Ready"),
    };
    let seconds = state.status.elapsed_ms / 1000;
    let heading = format!("{phase}  {:02}:{:02}", seconds / 60, seconds % 60);
    let detail = if state.status.detail.is_empty() {
        if let Some([x, y, width, height]) = state.target.crop {
            format!(
                "{} · {width}×{height} +{x}+{y}\n{}",
                state.target.display,
                text(
                    "录制所选区域；如出现系统授权窗口，请选择对应显示器",
                    "Records this region; choose the matching display if prompted"
                )
            )
        } else {
            text(
                "录制默认显示器；使用“选择区域”指定范围。部分桌面会显示授权窗口",
                "Records the default display; select a region to choose a source. Some desktops request permission",
            )
        }
    } else {
        state.status.detail.clone()
    };
    let items = layout::controls(state);
    let labels = [
        text("范围", "Source"),
        text("格式", "Format"),
        text("音频", "Audio"),
        text("质量", "Quality"),
        text("帧率", "Frame rate"),
        text("选项", "Options"),
    ];
    state.fonts.ensure_text(&format!(
        "{heading} {detail} {} {}",
        labels.join(" "),
        items
            .iter()
            .map(|(_, _, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    ));
    let palette = Palette::current();
    let active = state.active();
    let hover = layout::hit(state, state.pointer);
    canvas.fill(palette.window.color());
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_icons(&state.icons)
        .with_text(&mut state.fonts, &mut state.cache);
    painter.icon(crate::ui::icons::VIDEO, 20.0, 16.0, 14.0, palette.accent);
    painter.text(
        rect(46.0, 8.0, state.size.0 as f32 - 96.0, 32.0),
        &heading,
        TextStyle::new(13.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
    );
    for (label, y) in labels.iter().zip([58.0, 98.0, 132.0, 166.0, 202.0, 242.0]) {
        painter.text(
            rect(18.0, y, 96.0, 30.0),
            label,
            TextStyle::new(11.0, palette.muted),
        );
    }
    painter.text(
        rect(154.0, 202.0, 54.0, 30.0),
        &state.config.frame_rate().to_string(),
        TextStyle::new(12.0, palette.text).center(),
    );
    for (control, area, label) in items {
        if control == Control::Close {
            painter.icon(
                crate::ui::icons::CLOSE,
                16.0,
                area.left() + 6.0,
                area.top() + 6.0,
                palette.muted,
            );
            continue;
        }
        let selected = match control {
            Control::Format(index) => state.config.format == ["mp4", "mkv", "gif"][index],
            Control::Audio(index) => state.config.audio == ["off", "system", "microphone"][index],
            Control::Quality(index) => {
                state.config.quality == ["balanced", "high", "efficient"][index]
            }
            Control::Cursor => state.config.cursor,
            Control::Start => true,
            _ => false,
        };
        let enabled = match control {
            Control::Audio(_) => !active && state.config.format != "gif",
            Control::Pause => ["paused", "recording"].contains(&state.status.phase.as_str()),
            Control::Stop | Control::Copy => true,
            _ => !active,
        };
        controls::button(
            &mut painter,
            area,
            &label,
            selected,
            hover == Some(control),
            enabled,
        );
    }
    // 1. 【录制窗口】【有界错误显示】换行后按可见高度显示，完整内容可复制，滚轮浏览其他行
    let maximum = ((state.size.1 as f32 - 340.0) / 16.0).max(1.0) as usize;
    let columns = ((state.size.0 as f32 - 32.0) / 10.0).max(1.0) as usize;
    let lines: Vec<String> = detail
        .lines()
        .flat_map(|line| {
            line.chars()
                .collect::<Vec<_>>()
                .chunks(columns)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<_>>()
        })
        .collect();
    state.error_scroll = state.error_scroll.min(lines.len().saturating_sub(maximum));
    for (index, line) in lines
        .iter()
        .skip(state.error_scroll)
        .take(maximum)
        .enumerate()
    {
        let line = controls::fit_label(&mut painter, line, 10.0, state.size.0 as f32 - 32.0);
        painter.text(
            rect(
                16.0,
                286.0 + index as f32 * 16.0,
                state.size.0 as f32 - 32.0,
                16.0,
            ),
            &line,
            TextStyle::new(10.0, palette.muted),
        );
    }
    crate::ui::window_frame::draw(painter.canvas(), scale);
}
