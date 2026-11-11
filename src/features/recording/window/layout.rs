//! 【录制窗口】【控件布局】绘制和命中共用紧凑几何

use super::Recorder;
use tiny_skia::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
    Close,
    Region,
    Display,
    Format(usize),
    Audio(usize),
    Quality(usize),
    Fps(i32),
    Countdown,
    Cursor,
    Settings,
    Start,
    Pause,
    Stop,
    Copy,
}

/// 【录制窗口】【矩形构造】按 x、y、width、height 返回有效控件矩形
pub(super) fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width.max(1.0), height.max(1.0)).unwrap()
}

/// 【录制窗口】【控件列表】按 state 当前尺寸返回控件与可见名称，避免重复布局计算
pub(super) fn controls(state: &Recorder) -> Vec<(Control, Rect, String)> {
    let cn = state.chinese();
    let text = |a: &str, b: &str| if cn { a.to_owned() } else { b.to_owned() };
    let width = state.size.0 as f32;
    let mut items = vec![
        (
            Control::Close,
            rect(width - 40.0, 8.0, 28.0, 28.0),
            String::new(),
        ),
        (
            Control::Region,
            rect(120.0, 58.0, 138.0, 30.0),
            text("选择区域", "Select region"),
        ),
        (
            Control::Display,
            rect(266.0, 58.0, width - 282.0, 30.0),
            text("整个显示器", "Whole display"),
        ),
        (Control::Fps(-1), rect(120.0, 202.0, 32.0, 30.0), "−".into()),
        (Control::Fps(1), rect(210.0, 202.0, 32.0, 30.0), "+".into()),
        (
            Control::Countdown,
            rect(266.0, 202.0, width - 282.0, 30.0),
            format!(
                "{} {}s",
                text("倒计时", "Countdown"),
                state.config.countdown
            ),
        ),
        (
            Control::Cursor,
            rect(120.0, 242.0, 138.0, 30.0),
            text(
                if state.config.cursor {
                    "包含指针"
                } else {
                    "隐藏指针"
                },
                if state.config.cursor {
                    "Include cursor"
                } else {
                    "Hide cursor"
                },
            ),
        ),
        (
            Control::Settings,
            rect(266.0, 242.0, width - 282.0, 30.0),
            text("更多设置", "More settings"),
        ),
        (
            Control::Copy,
            rect(16.0, state.size.1 as f32 - 48.0, 100.0, 30.0),
            text("复制详情", "Copy details"),
        ),
    ];
    for (row, labels) in [
        vec!["MP4", "MKV", "GIF"],
        if cn {
            vec!["无声音", "系统声音", "麦克风"]
        } else {
            vec!["No audio", "System", "Microphone"]
        },
        if cn {
            vec!["均衡", "高质量", "较小文件"]
        } else {
            vec!["Balanced", "High", "Smaller file"]
        },
    ]
    .iter()
    .enumerate()
    {
        let item_width = (width - 152.0) / 3.0;
        for (index, label) in labels.iter().enumerate() {
            let control = match row {
                0 => Control::Format(index),
                1 => Control::Audio(index),
                _ => Control::Quality(index),
            };
            items.push((
                control,
                rect(
                    120.0 + index as f32 * (item_width + 8.0),
                    98.0 + row as f32 * 34.0,
                    item_width,
                    28.0,
                ),
                (*label).into(),
            ));
        }
    }
    let bottom = state.size.1 as f32 - 48.0;
    if state.active() {
        items.push((
            Control::Pause,
            rect(width - 234.0, bottom, 104.0, 30.0),
            if state.status.phase == "paused" {
                text("继续", "Resume")
            } else {
                text("暂停", "Pause")
            },
        ));
        items.push((
            Control::Stop,
            rect(width - 120.0, bottom, 104.0, 30.0),
            text("停止并保存", "Stop & save"),
        ));
    } else {
        items.push((
            Control::Start,
            rect(width - 160.0, bottom, 144.0, 30.0),
            text("开始录制", "Start recording"),
        ));
    }
    items
}

/// 【录制窗口】【指针命中】按 state 和 point 返回控件身份，没有命中返回空值
pub(super) fn hit(state: &Recorder, point: (f64, f64)) -> Option<Control> {
    controls(state)
        .into_iter()
        .find(|(_, area, _)| {
            point.0 >= f64::from(area.left())
                && point.0 < f64::from(area.right())
                && point.1 >= f64::from(area.top())
                && point.1 < f64::from(area.bottom())
        })
        .map(|(control, _, _)| control)
}
