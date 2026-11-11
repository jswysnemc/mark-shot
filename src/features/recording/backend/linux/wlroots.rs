//! 【Wayland录制】【直接采集】支持 wlroots 协议的桌面使用随包 wf-recorder，GPU 或软件路径共用控制时钟

use crate::{config::recording::Recording, features::recording::Target};
use std::{path::Path, process::Command};

/// 【Wayland录制】【协议候选】无参数，按桌面声明选择直接捕获候选；其他桌面使用通用 Portal
pub(super) fn preferred() -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    ["niri", "sway", "hyprland", "wayfire", "river", "labwc"]
        .iter()
        .any(|name| desktop.contains(name))
}

/// 【Wayland录制】【命令构造】按 config、target 和 path 返回支持暂停与限帧的随包组件命令
pub(super) fn build(config: &Recording, target: &Target, path: &Path) -> Result<Command, String> {
    let executable = super::super::component::executable("wf-recorder")?;
    if executable
        .parent()
        .and_then(Path::file_name)
        .is_none_or(|name| name != "recording")
    {
        return Err("直接 Wayland 录制需要完整构建包 / Direct Wayland recording requires the bundled component".into());
    }
    // 1. 【Wayland录制】【显示器解析】多显示器时也明确指定默认来源，避免组件要求用户在终端选择
    let session =
        crate::backend::session::Session::connect_to_env().map_err(|error| error.to_string())?;
    let overlay = crate::backend::initialize_overlay(session).map_err(|error| error.to_string())?;
    let (index, output) = overlay
        .discovered_outputs()
        .iter()
        .enumerate()
        .find(|(index, output)| {
            target.display.is_empty()
                || crate::backend::output_selection::name(output, *index) == target.display
        })
        .ok_or("录制显示器已移除 / Display removed")?;
    let name = crate::backend::output_selection::name(output, index);
    let mut command = Command::new(executable);
    command
        .args([
            "--no-damage",
            "-r",
            &config.frame_rate().to_string(),
            "-m",
            "mp4",
            "-f",
        ])
        .arg(path);
    command.args(["-o", &name]);
    if let Some([x, y, width, height]) = target.crop {
        // 2. 【Wayland录制】【全局区域】wf-recorder 接受全局逻辑几何，将物理区域还原到目标输出
        let (left, top) = output.info.logical_position.unwrap_or(output.info.location);
        let scale = output.scale();
        command.args([
            "-g",
            &format!(
                "{},{} {}x{}",
                left + (x as f32 / scale).round() as i32,
                top + (y as f32 / scale).round() as i32,
                (width as f32 / scale).round() as u32,
                (height as f32 / scale).round() as u32
            ),
        ]);
    }
    if !config.cursor {
        command.arg("--no-cursor");
    }
    if config.encoder == "hardware" {
        let render = std::fs::read_dir("/dev/dri")
            .map_err(|error| error.to_string())?
            .flatten()
            .find(|entry| entry.file_name().to_string_lossy().starts_with("renderD"))
            .ok_or("没有可用的 VAAPI 设备 / VAAPI device unavailable")?
            .path();
        command.args(["-c", "h264_vaapi", "-d"]).arg(render);
        command.args([
            "-p",
            match config.quality.as_str() {
                "high" => "qp=18",
                "efficient" => "qp=28",
                _ => "qp=23",
            },
        ]);
    } else {
        command.args([
            "-c",
            "libx264",
            "-p",
            "preset=superfast",
            "-p",
            "threads=2",
            "-p",
            match config.quality.as_str() {
                "high" => "crf=18",
                "efficient" => "crf=28",
                _ => "crf=23",
            },
        ]);
    }
    if config.format != "gif" && config.audio != "off" {
        command.args(["--audio-backend", "pulse", "--audio-codec", "aac"]);
        command.arg(if config.audio == "system" {
            "--audio=@DEFAULT_MONITOR@"
        } else {
            "--audio=@DEFAULT_SOURCE@"
        });
    }
    Ok(command)
}
