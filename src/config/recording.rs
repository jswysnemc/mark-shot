//! 【录制配置】【数据模型】仅保存用户选项，设备与会话资源留在录制后端

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Recording {
    pub format: String,
    pub quality: String,
    pub encoder: String,
    pub fps: u32,
    pub gif_fps: u32,
    pub countdown: u32,
    pub audio: String,
    pub cursor: bool,
    pub directory: String,
}

impl Default for Recording {
    /// 【录制配置】【默认值】无参数，返回兼容视频与按需音频配置
    fn default() -> Self {
        Self {
            format: "mp4".into(),
            quality: "balanced".into(),
            encoder: "auto".into(),
            fps: 30,
            gif_fps: 12,
            countdown: 3,
            audio: "off".into(),
            cursor: true,
            directory: String::new(),
        }
    }
}

impl Recording {
    /// 【录制配置】【启动校验】无参数，返回非法配置原因；不静默更改用户值
    pub fn validate(&self) -> Result<(), String> {
        for (value, choices, field) in [
            (self.format.as_str(), &["mp4", "mkv", "gif"][..], "format"),
            (
                self.quality.as_str(),
                &["balanced", "high", "efficient"][..],
                "quality",
            ),
            (
                self.encoder.as_str(),
                &["auto", "hardware", "software"][..],
                "encoder",
            ),
            (
                self.audio.as_str(),
                &["off", "system", "microphone"][..],
                "audio",
            ),
        ] {
            if !choices.contains(&value) {
                return Err(format!(
                    "recording.{field}: invalid option / 无效选项: {value}"
                ));
            }
        }
        if !(1..=60).contains(&self.fps) || !(1..=30).contains(&self.gif_fps) || self.countdown > 10
        {
            return Err(
                "录制帧率应为 1–60，GIF 为 1–30，倒计时为 0–10 秒 / Invalid FPS or countdown"
                    .into(),
            );
        }
        Ok(())
    }

    /// 【录制配置】【目标帧率】无参数，返回当前输出模式需要的最大采集帧率
    pub fn frame_rate(&self) -> u32 {
        if self.format == "gif" {
            self.gif_fps
        } else {
            self.fps
        }
    }
}
