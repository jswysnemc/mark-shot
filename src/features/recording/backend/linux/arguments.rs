//! 【Linux录制】【参数构造】避免 shell 拼接，默认 Portal 不申请 KMS 管理权限

use crate::{config::recording::Recording, features::recording::Target};
use std::{path::Path, process::Command};

/// 【Linux录制】【命令构造】按 config、target、path 返回独立录制命令，非法几何与缺少组件返回错误
pub(super) fn build(config: &Recording, target: &Target, path: &Path) -> Result<Command, String> {
    let executable = super::super::component::executable("gpu-screen-recorder")?;
    let mut process = Command::new(&executable);
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    if wayland {
        process.args(["-w", "portal", "-restore-portal-session", "no"]);
        if let Some([x, y, width, height]) = target.crop {
            if width < 2 || height < 2 {
                return Err("录制区域至少为 2×2 / Recording region is too small".into());
            }
            if executable
                .parent()
                .and_then(Path::file_name)
                .is_none_or(|name| name != "recording")
            {
                return Err("Wayland 选区录制需要完整构建包中的录制组件 / Region recording requires the bundled component".into());
            }
            process.env(
                "MARK_SHOT_RECORDING_CROP",
                format!("{x},{y},{width},{height}"),
            );
        }
    } else {
        // 1. 【Linux录制】【X11来源】随包组件固定无特权 XImage 捕获，编码仍可使用 GPU
        if executable
            .parent()
            .and_then(Path::file_name)
            .is_none_or(|name| name != "recording")
        {
            return Err(
                "X11 录制需要完整构建包中的录制组件 / X11 recording requires the bundled component"
                    .into(),
            );
        }
        process.env("MARK_SHOT_X11_UNPRIVILEGED", "1");
        process.args(["-w", "region", "-region", &super::x11::region(target)?]);
    }
    process.args([
        "-f",
        &config.frame_rate().to_string(),
        "-fm",
        "vfr",
        "-k",
        "h264",
        "-ac",
        "aac",
        "-c",
        "mp4",
        "-q",
    ]);
    process.arg(match config.quality.as_str() {
        "high" => "very_high",
        "efficient" => "medium",
        _ => "high",
    });
    process.args([
        "-encoder",
        if config.encoder == "software" {
            "cpu"
        } else {
            "gpu"
        },
        "-fallback-cpu-encoding",
        if config.encoder == "auto" {
            "yes"
        } else {
            "no"
        },
    ]);
    process.args(["-cursor", if config.cursor { "yes" } else { "no" }]);
    process.args([
        "-ffmpeg-opts",
        "movflags=+frag_keyframe+empty_moov+default_base_moof",
    ]);
    if config.format != "gif" {
        match config.audio.as_str() {
            "system" => {
                process.args(["-a", "default_output"]);
            }
            "microphone" => {
                process.args(["-a", "default_input"]);
            }
            _ => {}
        }
    }
    process.arg("-o").arg(path);
    Ok(process)
}
