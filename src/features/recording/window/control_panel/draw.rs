//! 【录制控制面板】【内容绘制】共用主题、按钮和字库显示状态、时长与隐藏后的控制入口

use super::{
    Panel,
    layout::{self, Control},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use tiny_skia::{Pixmap, Rect};

/// 【录制控制面板】【界面绘制】按 panel、canvas 与 scale 绘制紧凑面板，文本按可用宽度截断，无返回值
pub(super) fn draw(panel: &mut Panel, canvas: &mut Pixmap, scale: f32) {
    let chinese = panel.chinese();
    let toggle = match (panel.model.status.phase.as_str(), chinese) {
        ("paused", true) => "继续",
        ("paused", false) => "Resume",
        (_, true) => "暂停",
        (_, false) => "Pause",
    };
    let stop = if chinese { "停止" } else { "Stop" };
    let hide = if chinese { "隐藏" } else { "Hide" };
    let title = panel.model.title(chinese);
    let hint = if chinese {
        "隐藏后录制继续进行"
    } else {
        "Recording continues when controls are hidden"
    };
    let entry = if chinese {
        "再次进入截图界面或通过托盘控制录制"
    } else {
        "Open capture or use the tray to pause, resume or stop"
    };
    let feedback = panel.model.error.as_deref().unwrap_or(hint);
    let palette = Palette::current();
    canvas.fill(palette.window.color());
    let mut shared = panel.fonts.borrow_mut();
    let fonts = &mut *shared;
    fonts
        .system
        .ensure_text(&format!("{title}{toggle}{stop}{hide}{feedback}{entry}"));
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut fonts.system, &mut fonts.cache);
    // 1. 【录制控制面板】【会话状态】状态名称和有效录制时长排在操作左侧
    painter.fill_circle(
        17.0,
        20.0,
        3.0,
        if panel.model.status.phase == "paused" {
            palette.muted.color()
        } else {
            palette.accent.color()
        },
    );
    painter.text(
        Rect::from_xywh(27.0, 8.0, 105.0, 20.0).unwrap(),
        title,
        TextStyle::new(11.0, palette.text),
    );
    let elapsed = controls::fit_label(&mut painter, &panel.model.elapsed(), 11.0, 105.0);
    painter.text(
        Rect::from_xywh(27.0, 26.0, 105.0, 17.0).unwrap(),
        &elapsed,
        TextStyle::new(11.0, palette.muted),
    );
    // 2. 【录制控制面板】【统一按钮】使用共用按钮表达可用状态，命令确认前禁止重复提交
    for (control, label) in [
        (Control::Toggle, toggle),
        (Control::Stop, stop),
        (Control::Hide, hide),
    ] {
        controls::button(
            &mut painter,
            control.bounds(),
            label,
            control == Control::Toggle && panel.model.status.phase == "paused",
            layout::hit(panel.pointer) == Some(control),
            panel.model.enabled(control),
        );
    }
    // 3. 【录制控制面板】【控制入口】面板内保留隐藏说明，桌面通知关闭时仍可阅读控制入口
    let width = layout::SIZE.0 as f32 - 24.0;
    let feedback = controls::fit_label(&mut painter, feedback, 10.0, width);
    painter.text(
        Rect::from_xywh(12.0, 49.0, width, 16.0).unwrap(),
        &feedback,
        TextStyle::new(
            10.0,
            if panel.model.error.is_some() {
                palette.error
            } else {
                palette.muted
            },
        ),
    );
    let entry = controls::fit_label(&mut painter, entry, 10.0, width);
    painter.text(
        Rect::from_xywh(12.0, 65.0, width, 16.0).unwrap(),
        &entry,
        TextStyle::new(10.0, palette.muted),
    );
    crate::ui::window_frame::draw(painter.canvas(), scale);
}
