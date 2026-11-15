//! 【录屏工具】【参数选项】定义小工具栏可调整的录制参数，不修改全局偏好

use crate::config::recording::Recording;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    Format,
    Fps,
    Countdown,
    Audio,
    Quality,
    Encoder,
    Cursor,
}

impl Parameter {
    /// 【录屏工具】【参数名称】按 cn 返回当前参数的中英文名称
    pub(crate) fn label(self, cn: bool) -> &'static str {
        match (self, cn) {
            (Self::Format, true) => "格式",
            (Self::Format, false) => "Format",
            (Self::Fps, true) => "帧率",
            (Self::Fps, false) => "FPS",
            (Self::Countdown, true) => "倒计时",
            (Self::Countdown, false) => "Delay",
            (Self::Audio, true) => "声音",
            (Self::Audio, false) => "Audio",
            (Self::Quality, true) => "画质",
            (Self::Quality, false) => "Quality",
            (Self::Encoder, true) => "编码器",
            (Self::Encoder, false) => "Encoder",
            (Self::Cursor, true) => "光标",
            (Self::Cursor, false) => "Cursor",
        }
    }

    /// 【录屏工具】【选项文本】按 config 与 cn 返回当前参数的全部选项，保留自定义帧率和秒数
    pub(crate) fn choices(self, config: &Recording, cn: bool) -> Vec<String> {
        let labels: &[&str] = match (self, cn) {
            (Self::Format, _) => &["MP4", "MKV", "GIF"],
            (Self::Audio, true) => &["关闭", "系统声音", "麦克风"],
            (Self::Audio, false) => &["Off", "System", "Microphone"],
            (Self::Quality, true) => &["均衡", "高画质", "小文件"],
            (Self::Quality, false) => &["Balanced", "High", "Smaller file"],
            (Self::Encoder, true) => &["自动", "硬件", "软件"],
            (Self::Encoder, false) => &["Auto", "Hardware", "Software"],
            (Self::Cursor, true) => &["关闭", "显示"],
            (Self::Cursor, false) => &["Off", "Show"],
            (Self::Fps | Self::Countdown, _) => {
                return self
                    .numbers(config)
                    .into_iter()
                    .map(|value| match self {
                        Self::Countdown if value == 0 => if cn { "关闭" } else { "Off" }.into(),
                        Self::Countdown if cn => format!("{value} 秒"),
                        Self::Countdown => format!("{value}s"),
                        _ => value.to_string(),
                    })
                    .collect();
            }
        };
        labels.iter().map(|label| (*label).into()).collect()
    }

    /// 【录屏工具】【当前选项】按 config 返回选中序号，所有合法配置都在选项列表中
    pub(crate) fn selected(self, config: &Recording) -> usize {
        let (values, current): (&[&str], &str) = match self {
            Self::Format => (&["mp4", "mkv", "gif"], &config.format),
            Self::Audio => (&["off", "system", "microphone"], &config.audio),
            Self::Quality => (&["balanced", "high", "efficient"], &config.quality),
            Self::Encoder => (&["auto", "hardware", "software"], &config.encoder),
            Self::Cursor => return usize::from(config.cursor),
            Self::Fps | Self::Countdown => {
                let current = if self == Self::Fps {
                    config.frame_rate()
                } else {
                    config.countdown
                };
                return self
                    .numbers(config)
                    .iter()
                    .position(|value| *value == current)
                    .unwrap_or(0);
            }
        };
        values
            .iter()
            .position(|value| *value == current)
            .unwrap_or(0)
    }

    /// 【录屏工具】【选项写入】按 index 更新 config 当前参数；非法序号或不可用参数不修改，无返回值
    pub(crate) fn choose(self, index: usize, config: &mut Recording) {
        if !self.enabled(config) {
            return;
        }
        match self {
            Self::Cursor => {
                if index < 2 {
                    config.cursor = index == 1;
                }
            }
            Self::Fps | Self::Countdown => {
                if let Some(value) = self.numbers(config).get(index) {
                    if self == Self::Countdown {
                        config.countdown = *value;
                    } else if config.format == "gif" {
                        config.gif_fps = *value;
                    } else {
                        config.fps = *value;
                    }
                }
            }
            _ => {
                let (values, target): (&[&str], &mut String) = match self {
                    Self::Format => (&["mp4", "mkv", "gif"], &mut config.format),
                    Self::Audio => (&["off", "system", "microphone"], &mut config.audio),
                    Self::Quality => (&["balanced", "high", "efficient"], &mut config.quality),
                    Self::Encoder => (&["auto", "hardware", "software"], &mut config.encoder),
                    _ => unreachable!(),
                };
                if let Some(value) = values.get(index) {
                    *target = (*value).into();
                }
            }
        }
    }

    /// 【录屏工具】【参数可用】按 config 返回是否接受输入，GIF 不提供声音和视频编码器
    pub(crate) fn enabled(self, config: &Recording) -> bool {
        config.format != "gif" || !matches!(self, Self::Audio | Self::Encoder)
    }

    /// 【录屏工具】【数值微调】按 delta 调整 config 帧率或倒计时，返回是否改变；其他参数返回 false
    pub(crate) fn step(self, delta: i32, config: &mut Recording) -> bool {
        let (target, min, max) = match self {
            Self::Fps if config.format == "gif" => (&mut config.gif_fps, 1, 30),
            Self::Fps => (&mut config.fps, 1, 60),
            Self::Countdown => (&mut config.countdown, 0, 10),
            _ => return false,
        };
        let next = (*target as i32 + delta).clamp(min, max) as u32;
        let changed = *target != next;
        *target = next;
        changed
    }

    /// 【录屏工具】【数值列表】按 config 返回紧凑预设并插入当前值，用于显示和提交
    fn numbers(self, config: &Recording) -> Vec<u32> {
        let (mut values, current) = match self {
            Self::Fps if config.format == "gif" => (vec![8, 12, 15, 24, 30], config.gif_fps),
            Self::Fps => (vec![15, 24, 30, 60], config.fps),
            Self::Countdown => (vec![0, 3, 5, 10], config.countdown),
            _ => return Vec::new(),
        };
        if !values.contains(&current) {
            values.push(current);
            values.sort_unstable();
        }
        values
    }
}
