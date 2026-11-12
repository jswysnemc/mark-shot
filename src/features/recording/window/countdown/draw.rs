//! 【录制倒计时】【小组件绘制】数字徽标、说明与取消按钮排成一行，保持紧凑尺寸

use super::{Countdown, SIZE};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use std::time::Instant;
use tiny_skia::Rect;

/// 【录制倒计时】【内容绘制】按 state、canvas 与 scale 绘制剩余秒数、目标摘要和取消按钮，无返回值
pub(super) fn draw(state: &mut Countdown, canvas: &mut tiny_skia::Pixmap, scale: f32) {
    let cn = state.chinese();
    let seconds = state
        .schedule
        .map_or(state.duration.as_secs().max(1), |schedule| {
            schedule.seconds(Instant::now())
        })
        .to_string();
    let title = if cn {
        "即将开始录制"
    } else {
        "Recording starts soon"
    };
    let cancel = if cn { "取消" } else { "Cancel" };
    let hovered = Countdown::over_cancel(state.pointer);
    let palette = Palette::current();
    canvas.fill(palette.window.color());
    {
        let mut shared = state.fonts.borrow_mut();
        let fonts = &mut *shared;
        fonts
            .system
            .ensure_text(&format!("{title}{cancel}{}", state.summary));
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut fonts.system, &mut fonts.cache);
        // 1. 【录制倒计时】【数字徽标】强调色圆形内显示剩余秒数，字号遵循界面标题规范
        painter.fill_circle(28.0, SIZE.1 as f32 / 2.0, 16.0, palette.accent.color());
        painter.text(
            Rect::from_xywh(12.0, 12.0, 32.0, 32.0).unwrap(),
            &seconds,
            TextStyle::new(15.0, palette.on_accent)
                .weight(cosmic_text::Weight::SEMIBOLD)
                .center(),
        );
        // 2. 【录制倒计时】【说明文字】标题说明即将发生的动作，副行给出录制目标与格式
        let width = SIZE.0 as f32 - 54.0 - 80.0;
        painter.text(
            Rect::from_xywh(54.0, 9.0, width, 20.0).unwrap(),
            title,
            TextStyle::new(12.0, palette.text),
        );
        let summary = controls::fit_label(&mut painter, &state.summary, 10.0, width);
        painter.text(
            Rect::from_xywh(54.0, 29.0, width, 16.0).unwrap(),
            &summary,
            TextStyle::new(10.0, palette.muted),
        );
        // 3. 【录制倒计时】【取消按钮】倒计时期间唯一的操作，取消后回到准备窗口
        controls::icon_button(
            &mut painter,
            Countdown::cancel_bounds(),
            cancel,
            None,
            controls::ButtonStyle {
                fill: if hovered {
                    palette.alternate
                } else {
                    palette.window
                },
                foreground: palette.text,
                border: palette.border,
                font_size: 11.0,
                weight: cosmic_text::Weight::NORMAL,
            },
        );
        crate::ui::window_frame::draw(painter.canvas(), scale);
    }
    // 4. 【录制倒计时】【首帧计时】首帧内容准备完成后才开始倒数，用户总能看到完整的秒数
    state.begin(Instant::now());
}
