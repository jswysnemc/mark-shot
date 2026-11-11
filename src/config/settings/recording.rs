//! 【录制设置】【字段目录】注册原生设置窗口中的录制参数

use super::{Entry, Kind, Page};

/// 【录制设置】【字段构造】无参数，返回共用设置控件所需的字段与双语文案
pub(super) fn entries() -> Vec<Entry> {
    [
        (
            "format",
            ("输出格式", "Output format"),
            Kind::Segments(&["mp4", "mkv", "gif"]),
        ),
        (
            "quality",
            ("录制质量", "Quality"),
            Kind::Choice(&["balanced", "high", "efficient"]),
        ),
        (
            "encoder",
            ("视频编码", "Video encoding"),
            Kind::Choice(&["auto", "hardware", "software"]),
        ),
        ("fps", ("视频帧率", "Video FPS"), Kind::Text),
        ("gif_fps", ("GIF 帧率", "GIF FPS"), Kind::Text),
        (
            "countdown",
            ("开始倒计时（秒）", "Countdown (seconds)"),
            Kind::Text,
        ),
        (
            "audio",
            ("音频来源", "Audio source"),
            Kind::Choice(&["off", "system", "microphone"]),
        ),
        ("cursor", ("录制鼠标指针", "Include cursor"), Kind::Toggle),
        (
            "directory",
            ("录制保存目录", "Recording directory"),
            Kind::Text,
        ),
    ]
    .into_iter()
    .map(|(key, labels, kind)| Entry {
        page: Page::Recording,
        section: "recording",
        key,
        labels,
        kind,
    })
    .collect()
}

/// 【录制设置】【选项显示】按 key、value 和 chinese 返回有业务语义的名称，未知选项返回空值
pub(super) fn label(key: &str, value: &str, chinese: bool) -> Option<&'static str> {
    let pair = match (key, value) {
        ("quality", "balanced") => ("均衡", "Balanced"),
        ("quality", "high") => ("高质量", "High quality"),
        ("quality", "efficient") => ("较小文件", "Smaller file"),
        ("encoder", "auto") => ("自动", "Automatic"),
        ("encoder", "hardware") => ("硬件编码", "Hardware"),
        ("encoder", "software") => ("软件编码", "Software"),
        ("audio", "off") => ("不录制音频", "No audio"),
        ("audio", "system") => ("系统声音", "System audio"),
        ("audio", "microphone") => ("默认麦克风", "Default microphone"),
        _ => return None,
    };
    Some(if chinese { pair.0 } else { pair.1 })
}
