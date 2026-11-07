//! 【图片上传】【命令执行】私有 PNG 与原版图片占位符，有限管道输出，取消后回收所属进程组

use super::{Error, ErrorKind, MAX_RESPONSE, Output, Result};
use crate::{
    config::upload::Upload,
    features::external_actions::{desktop_exec, launch::shell_quote},
};
use std::{io::Write, path::Path, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, sync::oneshot};

use crate::utils::process_group::Group;

/// 【图片上传】【图片参数】按 template、image 返回一次性引用命令，只有原版三种图片占位符参与替换；缺失时追加路径
fn command(template: &str, image: &Path) -> Result<String> {
    let file = image
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::Configuration, "Invalid upload image path"))?;
    let url = desktop_exec::file_url(image)
        .map_err(|_| Error::new(ErrorKind::Configuration, "Invalid upload image path"))?;
    let values = [
        ("{image}", file),
        ("{imagePath}", file),
        ("{imageUrl}", url.as_str()),
    ];
    let mut remaining = template;
    let mut result = String::new();
    let mut replaced = false;
    while let Some((offset, field, value)) = values
        .iter()
        .filter_map(|(field, value)| remaining.find(field).map(|offset| (offset, *field, *value)))
        .min_by_key(|item| item.0)
    {
        result.push_str(&remaining[..offset]);
        result.push_str(&shell_quote(value));
        remaining = &remaining[offset + field.len()..];
        replaced = true;
    }
    result.push_str(remaining);
    if !replaced {
        result.push(' ');
        result.push_str(&shell_quote(file));
    }
    Ok(result)
}

/// 【图片上传】【有限管道】读取 reader 的最多 limit 字节，返回输出或超限错误，不显示标准错误中的凭据
async fn read_limited(reader: impl tokio::io::AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Error::new(ErrorKind::Protocol, "Cannot read upload command output"))?;
    if bytes.len() > limit {
        return Err(Error::new(
            ErrorKind::Protocol,
            "Upload command output exceeds limit",
        ));
    }
    Ok(bytes)
}

/// 【图片上传】【单次命令】按 config、png 和 cancellation 执行上传，返回 URL；成功、取消、超时均删除临时文件并回收子进程
pub(super) async fn run(
    config: &Upload,
    png: &[u8],
    mut cancellation: oneshot::Receiver<()>,
) -> Result<Output> {
    if !matches!(
        cancellation.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ) {
        return Err(Error::new(ErrorKind::Canceled, "Upload canceled"));
    }
    let mut file = tempfile::Builder::new()
        .prefix("mark-shot-upload-")
        .suffix(".png")
        .tempfile()
        .map_err(|_| Error::new(ErrorKind::StartFailed, "Cannot create upload image"))?;
    file.write_all(png)
        .and_then(|_| file.flush())
        .map_err(|_| Error::new(ErrorKind::StartFailed, "Cannot write upload image"))?;
    let mut builder = tokio::process::Command::new(&config.command_shell);
    builder
        .arg("-c")
        .arg(command(&config.command, file.path())?)
        .env_clear()
        .envs(&config.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    crate::utils::process_group::configure(&mut builder);
    let mut child = builder
        .spawn()
        .map_err(|_| Error::new(ErrorKind::StartFailed, "Upload command failed to start"))?;
    let mut group = Group::new(&child)
        .map_err(|_| Error::new(ErrorKind::StartFailed, "Cannot track upload command"))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    // 1. 【图片上传】【任务边界】管道、退出、取消和超时共同等待，任一输出超限立即终止任务
    let completed = tokio::select! {
        biased;
        _ = &mut cancellation => Err(Error::new(ErrorKind::Canceled, "Upload canceled")),
        _ = tokio::time::sleep(Duration::from_millis(config.timeout_ms)) => Err(Error::new(ErrorKind::Timeout, "Upload timed out")),
        result = async { tokio::try_join!(async { child.wait().await.map_err(|_| Error::new(ErrorKind::Command, "Cannot wait for upload command")) }, read_limited(stdout, MAX_RESPONSE), read_limited(stderr, 512 * 1024)) } => result,
    };
    // 2. 【图片上传】【明确回收】先终止所属组，再确认直接子进程退出，最后由文件句柄删除私有 PNG
    group.terminate();
    child
        .wait()
        .await
        .map_err(|_| Error::new(ErrorKind::Command, "Cannot reap upload command"))?;
    let (status, bytes, _) = completed?;
    if !status.success() {
        return Err(Error::new(ErrorKind::Command, "Upload failed"));
    }
    super::request::output(&bytes, "", "", true)
}
