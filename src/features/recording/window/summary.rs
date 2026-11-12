//! 【录制窗口】【目标摘要】准备窗口与倒计时小组件共用的目标与格式说明，不读取配置文件

use super::super::Target;
use crate::config::recording::Recording;

/// 【录制窗口】【目标说明】按 target 和 cn 返回显示器与区域坐标，未指定区域时说明使用默认显示器
pub(super) fn target(target: &Target, cn: bool) -> String {
    match target.crop {
        Some([x, y, width, height]) if target.display.is_empty() => {
            format!("{width}×{height} +{x}+{y}")
        }
        Some([x, y, width, height]) => format!("{} · {width}×{height} +{x}+{y}", target.display),
        None if !target.display.is_empty() => target.display.clone(),
        None if cn => "默认显示器".into(),
        None => "Default display".into(),
    }
}

/// 【录制窗口】【格式说明】按 config 返回本次输出格式与帧率，GIF 使用独立帧率
pub(super) fn format(config: &Recording) -> String {
    format!(
        "{} · {} fps",
        config.format.to_uppercase(),
        config.frame_rate()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【录制窗口测试】【摘要文本】区域、指定显示器和默认显示器分别给出可读说明，GIF 使用自身帧率
    #[test]
    fn summaries_describe_the_target_and_format() {
        let mut value = Target {
            display: "eDP-2".into(),
            crop: Some([614, 403, 1040, 824]),
            format: None,
        };
        assert_eq!(target(&value, true), "eDP-2 · 1040×824 +614+403");
        value.crop = None;
        assert_eq!(target(&value, true), "eDP-2");
        value.display.clear();
        assert_eq!(target(&value, true), "默认显示器");
        assert_eq!(target(&value, false), "Default display");
        let mut config = Recording::default();
        assert_eq!(format(&config), "MP4 · 30 fps");
        config.format = "gif".into();
        assert_eq!(format(&config), "GIF · 12 fps");
    }
}
