//! 【录制导出】【进程内转换】录制结束后在同一工作线程完成封装或 GIF 导出，无媒体子进程

use super::{
    Command,
    backend::{self, Shared},
    output::Output,
};
use std::sync::mpsc::Receiver;

/// 【录制导出】【成品生成】按 config 导出 output，commands 提供取消，status 显示阶段，成功后原子发布
pub(super) fn finish(
    config: &crate::config::recording::Recording,
    output: &Output,
    commands: &Receiver<Command>,
    status: &Shared,
) -> Result<(), String> {
    if config.format == "mp4" {
        return output.publish(&output.video);
    }
    let destination = output.directory.join(format!("export.{}", config.format));
    let media = shot_media::Media::load()?;
    media
        .export(
            &output.video,
            &destination,
            (config.format == "gif").then_some(config.gif_fps),
            || matches!(commands.try_recv(), Ok(Command::Stop)),
        )
        .map_err(|error| {
            format!("导出失败，原始视频保留 / Export failed; video retained: {error}")
        })?;
    let elapsed = status.lock().unwrap().elapsed_ms;
    backend::publish(status, "saving", "", elapsed);
    output.publish(&destination)
}
