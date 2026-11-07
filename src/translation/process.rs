//! 【翻译任务】【命令隔离】使用私有临时输入与独立进程组，取消时回收命令及后代

use super::{Error, ErrorKind, Output, Provider, Result, Segment, request::Request};
use serde_json::Value;
use std::{io::Write, path::Path, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt;
use tokio::sync::oneshot;

use crate::utils::process_group::Group;

/// 【翻译任务】【参数引用】按 value 返回单引号安全 shell 参数，不把用户正文直接插入命令
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// 【翻译任务】【命令占位符】按 template、input、target 和 config 返回参数引用结果，只解析原始模板，没有占位符时追加输入路径
fn command(template: &str, input: &str, target: &str, config: &str) -> String {
    let values = [
        ("{input}", quote(input)),
        ("{inputPath}", quote(input)),
        ("{targetLanguage}", quote(target)),
        ("{config}", quote(config)),
    ];
    let mut result = String::new();
    let mut remaining = template;
    let mut replaced = false;
    // 1. 【翻译任务】【模板边界】替换内容中的花括号保持字面量，不能成为下一轮占位符或破坏 shell 参数引用
    while let Some(index) = remaining.find('{') {
        result.push_str(&remaining[..index]);
        remaining = &remaining[index..];
        if let Some((placeholder, value)) = values
            .iter()
            .find(|(placeholder, _)| remaining.starts_with(placeholder))
        {
            result.push_str(value);
            remaining = &remaining[placeholder.len()..];
            replaced = true;
        } else {
            result.push('{');
            remaining = &remaining[1..];
        }
    }
    result.push_str(remaining);
    if !replaced {
        result.push(' ');
        result.push_str(&quote(input));
    }
    result
}

/// 【翻译任务】【外部结果】按 provider 和 bytes 解析原版 tokens 输出，返回译文；成功但没有有效文本同样失败
fn output(provider: Provider, bytes: &[u8]) -> Result<Output> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| {
        Error::new(
            ErrorKind::Protocol,
            "translation command response is not valid JSON",
        )
    })?;
    let segments = root
        .get("tokens")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, token)| {
            token
                .get("text")
                .and_then(Value::as_str)
                .map(|text| Segment {
                    id: index as i32,
                    text: text.trim().to_owned(),
                })
        })
        .collect();
    let result = Output { provider, segments };
    if result.text().is_empty() {
        let detail = root
            .pointer("/errors/0")
            .and_then(Value::as_str)
            .unwrap_or("no translation result");
        return Err(Error::new(ErrorKind::Protocol, detail));
    }
    Ok(result)
}

/// 【翻译任务】【限量管道】按 reader 和 limit 读取有限输出，返回字节或超限错误，不等待已经无法继续读取的子进程
async fn read_limited(reader: impl tokio::io::AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| {
            Error::new(
                ErrorKind::Protocol,
                "cannot read translation command output",
            )
        })?;
    if bytes.len() > limit {
        return Err(Error::new(
            ErrorKind::Protocol,
            "translation command output exceeds limit",
        ));
    }
    Ok(bytes)
}

/// 【翻译任务】【外部执行】按服务、命令、原文、目标、配置路径、cancellation 和 timeout 返回有限结果，取消或超时后先终止进程组再回收直接子进程
#[cfg(test)]
pub(crate) async fn run(
    provider: Provider,
    template: &str,
    segments: &[Segment],
    target: &str,
    config: &Path,
    cancellation: oneshot::Receiver<()>,
    timeout: Duration,
) -> Result<Output> {
    run_request(
        provider,
        template,
        &Request::Text(segments.to_vec()),
        target,
        config,
        cancellation,
        timeout,
    )
    .await
}

/// 【翻译任务】【快照执行】按 provider、template、request、target、config、cancellation 和 timeout 执行一次真实请求，返回保留段落身份的结果
pub(crate) async fn run_request(
    provider: Provider,
    template: &str,
    request: &Request,
    target: &str,
    config: &Path,
    mut cancellation: oneshot::Receiver<()>,
    timeout: Duration,
) -> Result<Output> {
    if !matches!(
        cancellation.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ) {
        return Err(Error::new(ErrorKind::Canceled, "Translation canceled"));
    }
    // 1. 【翻译任务】【私有输入】只把确认原文快照写入权限为 0600 的临时文件，整个未来值持有删除责任
    let input = request.input(target);
    let mut file = tempfile::Builder::new()
        .prefix("mark-shot-result-translate-")
        .suffix(".json")
        .tempfile()
        .map_err(|_| {
            Error::new(
                ErrorKind::StartFailed,
                "cannot create translation input file",
            )
        })?;
    file.write_all(input.to_string().as_bytes())
        .and_then(|_| file.flush())
        .map_err(|_| {
            Error::new(
                ErrorKind::StartFailed,
                "cannot write translation input file",
            )
        })?;
    let mut builder = if provider == Provider::Helper {
        let mut builder = tokio::process::Command::new("mark-shot-translate");
        builder
            .arg("--input")
            .arg(file.path())
            .arg("--target-language")
            .arg(target)
            .arg("--config")
            .arg(config);
        builder
    } else {
        let mut builder = tokio::process::Command::new("/bin/sh");
        builder.arg("-c").arg(command(
            template,
            &file.path().to_string_lossy(),
            target,
            &config.to_string_lossy(),
        ));
        builder
    };
    crate::utils::process_group::configure(&mut builder);
    builder
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = builder.spawn().map_err(|_| {
        Error::new(
            ErrorKind::StartFailed,
            "translation provider failed to start",
        )
    })?;
    let mut group = Group::new(&child)
        .map_err(|_| Error::new(ErrorKind::StartFailed, "cannot track translation provider"))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    // 2. 【翻译任务】【有限输出】同时读取两个管道和等待退出，取消及超时不直接丢弃整个进程未来值
    let completed = tokio::select! {
        biased;
        _ = &mut cancellation => Err(Error::new(ErrorKind::Canceled, "Translation canceled")),
        _ = tokio::time::sleep(timeout) => Err(Error::new(ErrorKind::Timeout, "timed out")),
        result = async {
            tokio::try_join!(
                async { child.wait().await.map_err(|_| Error::new(ErrorKind::StartFailed, "cannot wait for translation provider")) },
                read_limited(stdout, 4 * 1024 * 1024), read_limited(stderr, 512 * 1024))
        } => result,
    };
    // 3. 【翻译任务】【明确回收】失败时也等待本次直接子进程退出，窗口线程不参与等待，输入文件在此后自动删除
    group.terminate();
    let reaped = child
        .wait()
        .await
        .map_err(|_| Error::new(ErrorKind::StartFailed, "cannot reap translation provider"));
    let (status, bytes, errors) = completed?;
    reaped?;
    if !status.success() {
        return Err(Error::new(
            ErrorKind::Api,
            String::from_utf8_lossy(&errors[..errors.len().min(500)])
                .trim()
                .to_owned(),
        ));
    }
    if matches!(request, Request::Image { .. }) {
        let root = serde_json::from_slice(&bytes).map_err(|_| {
            Error::new(
                ErrorKind::Protocol,
                "translation command response is not valid JSON",
            )
        })?;
        request.image_output(provider, &root).unwrap()
    } else {
        output(provider, &bytes)
    }
}

#[cfg(test)]
mod tests;
