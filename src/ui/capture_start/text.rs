//! 【截图起始】【界面文案】入口、录制状态与消息条的中英文文字集中维护，不包含绘制逻辑

use super::Control;
use crate::features::recording::Status;

/// 【截图起始】【控件名称】按 control、paused 和 cn 返回本地化按钮文字
pub(super) fn label(control: Control, paused: bool, cn: bool) -> &'static str {
    match (control, cn) {
        (Control::Region, true) => "选区",
        (Control::Region, false) => "Region",
        (Control::Fullscreen, true) => "全屏",
        (Control::Fullscreen, false) => "Screen",
        (Control::Video, true) => "录屏",
        (Control::Video, false) => "Video",
        (Control::Gif, true) => "录 GIF",
        (Control::Gif, false) => "GIF",
        (Control::Pause, true) if paused => "继续",
        (Control::Pause, true) => "暂停",
        (Control::Pause, false) if paused => "Resume",
        (Control::Pause, false) => "Pause",
        (Control::Stop, true) => "停止并保存",
        (Control::Stop, false) => "Stop & save",
        (Control::Background, _) => "",
    }
}

/// 【截图录制】【阶段按钮】按 control、status 和 cn 返回当前可执行动作，倒计时与启动阶段取消录制而不是保存
pub(super) fn control_label(control: Control, status: &Status, cn: bool) -> &'static str {
    if control == Control::Stop && matches!(status.phase.as_str(), "countdown" | "starting") {
        if cn { "取消录制" } else { "Cancel" }
    } else {
        label(control, status.phase == "paused", cn)
    }
}

/// 【截图起始】【录制类型】按 status 的输出路径和 cn 返回 GIF 或录屏名称
fn kind(status: &Status, cn: bool) -> &'static str {
    if status.format == "gif"
        || status
            .output
            .as_ref()
            .is_some_and(|path| path.ends_with(".gif"))
    {
        "GIF"
    } else if cn {
        "录屏"
    } else {
        "Video"
    }
}

/// 【截图起始】【状态文字】按 status 和 cn 返回录制类型、阶段及时长，不显示内部诊断内容
pub(super) fn status(status: &Status, cn: bool) -> String {
    let kind = kind(status, cn);
    // 1. 【截图起始】【倒计时文字】倒计时阶段没有已录时长，显示距离开始的剩余秒数
    if status.phase == "countdown" {
        let seconds = status.detail.trim();
        return if cn {
            format!("{kind} · {seconds} 秒后开始")
        } else {
            format!("{kind} · starts in {seconds}s")
        };
    }
    let phase = match (status.phase.as_str(), cn) {
        ("paused", true) => "已暂停",
        ("paused", false) => "Paused",
        ("starting", true) => "准备中",
        ("starting", false) => "Starting",
        ("saving", true) => "保存中",
        ("saving", false) => "Saving",
        (_, true) => "录制中",
        (_, false) => "REC",
    };
    let seconds = status.elapsed_ms / 1000;
    format!("{kind} · {phase} {:02}:{:02}", seconds / 60, seconds % 60)
}

/// 【截图起始】【消息文字】按 status、fallback 和 cn 返回消息条内容
///
/// 参数：status 最近一次录制状态、fallback 查询失败时的提示、cn 是否使用中文；
/// 返回已保存的文件名、失败提示或录制窗口占用提示
pub(super) fn message(status: &Status, fallback: &str, cn: bool) -> String {
    match status.phase.as_str() {
        "finished" => {
            let name = status
                .output
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if cn {
                format!("录制已保存 {name}")
            } else {
                format!("Recording saved {name}")
            }
        }
        "failed" if cn => "录制失败，请在录制窗口查看原因".into(),
        "failed" => "Recording failed. See the recording window".into(),
        _ if status.window_open && cn => "录制窗口已打开，请在原窗口操作".into(),
        _ if status.window_open => "Recording window is open".into(),
        _ => fallback.into(),
    }
}
