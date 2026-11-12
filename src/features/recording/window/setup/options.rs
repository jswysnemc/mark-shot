//! 【录制设置】【选项模型】每行的取值、标签与写入规则集中定义，绘制与输入共用同一来源

use crate::{config::recording::Recording, features::recording::Target};

/// 准备窗口的一行配置
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Row {
    Source,
    Format,
    Audio,
    Quality,
    Fps,
    Countdown,
}

/// 点击选项后的交接结果
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Outcome {
    /// 仅更新本窗口显示
    Redraw,
    /// 需要关闭窗口并重新框选区域
    SelectRegion,
}

impl Row {
    /// 【录制设置】【全部行】无参数，返回从上到下的分段行顺序，光标选项单独处理
    pub(super) const ALL: [Self; 6] = [
        Self::Source,
        Self::Format,
        Self::Audio,
        Self::Quality,
        Self::Fps,
        Self::Countdown,
    ];

    /// 【录制设置】【行名称】按 cn 返回左侧标签
    pub(super) fn label(self, cn: bool) -> &'static str {
        match (self, cn) {
            (Self::Source, true) => "范围",
            (Self::Source, false) => "Source",
            (Self::Format, true) => "格式",
            (Self::Format, false) => "Format",
            (Self::Audio, true) => "声音",
            (Self::Audio, false) => "Audio",
            (Self::Quality, true) => "画质",
            (Self::Quality, false) => "Quality",
            (Self::Fps, true) => "帧率",
            (Self::Fps, false) => "Frame rate",
            (Self::Countdown, true) => "倒计时",
            (Self::Countdown, false) => "Countdown",
        }
    }
}

/// 【录制设置】【选项文本】按 row、config、target 和 cn 返回该行全部选项，选中的区域带尺寸
pub(super) fn choices(row: Row, config: &Recording, target: &Target, cn: bool) -> Vec<String> {
    match row {
        Row::Source => {
            let region = match target.crop {
                Some([_, _, width, height]) if cn => format!("区域 {width}×{height}"),
                Some([_, _, width, height]) => format!("Region {width}×{height}"),
                None if cn => "选择区域…".into(),
                None => "Select region…".into(),
            };
            vec![
                region,
                if cn {
                    "整个显示器"
                } else {
                    "Whole display"
                }
                .into(),
            ]
        }
        Row::Format => vec!["MP4".into(), "MKV".into(), "GIF".into()],
        Row::Audio => if cn {
            ["关闭", "系统声音", "麦克风"]
        } else {
            ["Off", "System", "Microphone"]
        }
        .iter()
        .map(|label| (*label).into())
        .collect(),
        Row::Quality => if cn {
            ["均衡", "高画质", "小文件"]
        } else {
            ["Balanced", "High", "Smaller file"]
        }
        .iter()
        .map(|label| (*label).into())
        .collect(),
        Row::Fps => frame_rates(config)
            .into_iter()
            .map(|value| format!("{value}"))
            .collect(),
        Row::Countdown => countdowns(config.countdown)
            .into_iter()
            .map(|value| countdown_label(value, cn))
            .collect(),
    }
}

/// 【录制设置】【选项数量】按 row 和 config 返回该行的段数，布局命中时无需生成文字
pub(super) fn count(row: Row, config: &Recording) -> usize {
    match row {
        Row::Source => 2,
        Row::Format | Row::Audio | Row::Quality => 3,
        Row::Fps => frame_rates(config).len(),
        Row::Countdown => countdowns(config.countdown).len(),
    }
}

/// 【录制设置】【选中段】按 row、config 和 target 返回当前选中序号，取值始终包含在选项内
pub(super) fn selected(row: Row, config: &Recording, target: &Target) -> usize {
    match row {
        Row::Source => usize::from(target.crop.is_none()),
        Row::Format => ["mp4", "mkv", "gif"]
            .iter()
            .position(|value| *value == config.format)
            .unwrap_or(0),
        Row::Audio => ["off", "system", "microphone"]
            .iter()
            .position(|value| *value == config.audio)
            .unwrap_or(0),
        Row::Quality => ["balanced", "high", "efficient"]
            .iter()
            .position(|value| *value == config.quality)
            .unwrap_or(0),
        Row::Fps => frame_rates(config)
            .iter()
            .position(|value| *value == config.frame_rate())
            .unwrap_or(0),
        Row::Countdown => countdowns(config.countdown)
            .iter()
            .position(|value| *value == config.countdown)
            .unwrap_or(0),
    }
}

/// 【录制设置】【选项写入】按 row、index 更新 config 与 target，返回是否需要重新框选区域
pub(super) fn choose(
    row: Row,
    index: usize,
    config: &mut Recording,
    target: &mut Target,
) -> Outcome {
    match row {
        // 1. 【录制设置】【范围切换】选择区域始终重新框选，避免用旧区域表达新的意图
        Row::Source => match index {
            0 => Outcome::SelectRegion,
            _ => {
                *target = Target {
                    format: target.format,
                    ..Default::default()
                };
                Outcome::Redraw
            }
        },
        Row::Format => {
            if let Some(value) = ["mp4", "mkv", "gif"].get(index) {
                config.format = (*value).into();
                target.format = None;
            }
            Outcome::Redraw
        }
        Row::Audio => {
            if let Some(value) = ["off", "system", "microphone"].get(index) {
                config.audio = (*value).into();
            }
            Outcome::Redraw
        }
        Row::Quality => {
            if let Some(value) = ["balanced", "high", "efficient"].get(index) {
                config.quality = (*value).into();
            }
            Outcome::Redraw
        }
        Row::Fps => {
            if let Some(value) = frame_rates(config).get(index) {
                if config.format == "gif" {
                    config.gif_fps = *value;
                } else {
                    config.fps = *value;
                }
            }
            Outcome::Redraw
        }
        Row::Countdown => {
            if let Some(value) = countdowns(config.countdown).get(index) {
                config.countdown = *value;
            }
            Outcome::Redraw
        }
    }
}

/// 【录制设置】【行可用】按 row 和 config 返回该行是否接受输入，GIF 不提供音轨选择
pub(super) fn enabled(row: Row, config: &Recording) -> bool {
    !matches!(row, Row::Audio) || config.format != "gif"
}

/// 【录制设置】【帧率微调】按 delta 调整当前格式的帧率，返回是否发生变化；取值保持合法范围
pub(super) fn step_frame_rate(delta: i32, config: &mut Recording) -> bool {
    let limit = if config.format == "gif" { 30 } else { 60 };
    let target = if config.format == "gif" {
        &mut config.gif_fps
    } else {
        &mut config.fps
    };
    let next = (*target as i32 + delta).clamp(1, limit) as u32;
    if next == *target {
        return false;
    }
    *target = next;
    true
}

/// 【录制设置】【帧率选项】按 config 返回预设帧率，当前自定义值按序插入以免选项丢失
fn frame_rates(config: &Recording) -> Vec<u32> {
    let mut values: Vec<u32> = if config.format == "gif" {
        vec![8, 12, 15, 24]
    } else {
        vec![15, 24, 30, 60]
    };
    let current = config.frame_rate();
    if !values.contains(&current) {
        values.push(current);
        values.sort_unstable();
    }
    values
}

/// 【录制设置】【倒计时选项】按 countdown 返回预设秒数，当前值不是预设时按序插入
fn countdowns(countdown: u32) -> Vec<u32> {
    let mut values = vec![0, 3, 5, 10];
    if !values.contains(&countdown) {
        values.push(countdown);
        values.sort_unstable();
    }
    values
}

/// 【录制设置】【倒计时文本】按 value 和 cn 返回选项文字，零表示不等待
fn countdown_label(value: u32, cn: bool) -> String {
    match (value, cn) {
        (0, true) => "关闭".into(),
        (0, false) => "Off".into(),
        (value, true) => format!("{value} 秒"),
        (value, false) => format!("{value}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::recording::Format;

    /// 【录制设置测试】【选项往返】选中序号与写入后的取值一致，自定义帧率不会从选项中消失
    #[test]
    fn choices_round_trip_through_selection() {
        let mut config = Recording {
            fps: 50,
            ..Default::default()
        };
        let mut target = Target::default();
        for row in Row::ALL {
            let count = choices(row, &config, &target, true).len();
            assert_eq!(count, super::count(row, &config), "{row:?}");
            let index = selected(row, &config, &target);
            assert!(index < count, "{row:?}");
            assert_eq!(
                choose(row, index, &mut config, &mut target),
                Outcome::Redraw
            );
            assert_eq!(selected(row, &config, &target), index, "{row:?}");
        }
        assert_eq!(selected(Row::Fps, &config, &Target::default()), 3);
        assert!(choices(Row::Fps, &config, &target, true).contains(&"50".into()));
    }

    /// 【录制设置测试】【范围切换】重新选择区域交由外部流程，整个显示器清空区域保留格式偏好
    #[test]
    fn source_row_clears_the_region_only_for_the_whole_display() {
        let mut config = Recording::default();
        let mut target = Target {
            display: "DP-1".into(),
            crop: Some([1, 2, 3, 4]),
            format: Some(Format::Gif),
        };
        assert_eq!(
            choose(Row::Source, 0, &mut config, &mut target),
            Outcome::SelectRegion
        );
        assert!(target.crop.is_some());
        assert_eq!(
            choose(Row::Source, 1, &mut config, &mut target),
            Outcome::Redraw
        );
        assert!(target.crop.is_none() && target.display.is_empty());
        assert_eq!(target.format, Some(Format::Gif));
    }

    /// 【录制设置测试】【帧率边界】滚轮调整保持 1–60 与 GIF 1–30 的合法范围
    #[test]
    fn frame_rate_stepping_stays_in_range() {
        let mut config = Recording {
            fps: 60,
            gif_fps: 30,
            ..Default::default()
        };
        assert!(!step_frame_rate(1, &mut config));
        config.format = "gif".into();
        assert!(!step_frame_rate(1, &mut config));
        config.gif_fps = 1;
        assert!(!step_frame_rate(-1, &mut config));
        assert!(step_frame_rate(1, &mut config));
        assert_eq!(config.gif_fps, 2);
    }
}
