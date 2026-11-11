//! 【录制导出】【停止后转换】实时采集结束后才执行 GIF 量化或无损重封装

use super::{
    Command,
    backend::{self, Shared},
    output::Output,
};
use std::{
    io::Read,
    process::{Child, Stdio},
    sync::{Arc, Mutex, mpsc::Receiver},
    time::Duration,
};

/// 【录制导出】【有界日志】按 child 持续排空错误管道，只保存最近 16 KiB，返回日志和读取线程
pub(crate) fn stderr(child: &mut Child) -> (Arc<Mutex<Vec<u8>>>, std::thread::JoinHandle<()>) {
    let mut pipe = child.stderr.take().expect("recording stderr must be piped");
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let output = bytes.clone();
    let thread = std::thread::spawn(move || {
        let mut chunk = [0u8; 2048];
        while let Ok(count) = pipe.read(&mut chunk) {
            if count == 0 {
                break;
            }
            let mut output = output.lock().unwrap_or_else(|error| error.into_inner());
            output.extend_from_slice(&chunk[..count]);
            let remove = output.len().saturating_sub(16384);
            output.drain(..remove);
        }
    });
    (bytes, thread)
}

/// 【录制导出】【成品生成】按 config 将 output 工作视频发布为成品，commands 允许取消转换，status 显示阶段
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
    let mut process = std::process::Command::new(backend::component::executable("ffmpeg")?);
    process
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-n", "-i"])
        .arg(&output.video);
    if config.format == "gif" {
        process.args(["-filter_complex", &format!("fps={},split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=sierra2_4a", config.gif_fps), "-an", "-loop", "0"]);
    } else {
        process.args(["-map", "0", "-c", "copy"]);
    }
    process
        .arg(&destination)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    backend::component::quiet(&mut process);
    let mut child = process.spawn().map_err(|error| error.to_string())?;
    let (errors, reader) = stderr(&mut child);
    let result = loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(exit) => {
                break if exit.success() {
                    Ok(())
                } else {
                    Err(format!("导出失败 / Export failed ({exit})"))
                };
            }
            None => {
                if matches!(
                    commands.recv_timeout(Duration::from_millis(100)),
                    Ok(Command::Stop)
                ) {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(
                        "已取消导出，原始视频保留 / Export cancelled; video retained".into(),
                    );
                }
            }
        }
    };
    let _ = reader.join();
    result.map_err(|error| {
        format!(
            "{error}\n{}",
            String::from_utf8_lossy(&errors.lock().unwrap())
        )
    })?;
    let elapsed = status.lock().unwrap().elapsed_ms;
    backend::publish(status, "saving", "", elapsed);
    output.publish(&destination)
}
