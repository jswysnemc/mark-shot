//! 【Portal捕获】【授权会话】一次授权绑定实际输出，PipeWire流复用到用户结束捕获

use super::token;
use crate::{
    backend::{
        capture_session::{CaptureSession, Result},
        wayland::capture::stream::PipewireStream,
    },
    types::{CaptureResult, MonitorFrame, Output, StreamInfo},
};
use ashpd::desktop::{
    PersistMode, Session,
    screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType, Streams},
};
use async_trait::async_trait;
use std::os::fd::AsFd;

struct Source {
    output: usize,
    info: StreamInfo,
    stream: PipewireStream,
}

pub(super) struct PortalCapture {
    session: Option<Session<Screencast>>,
    sources: Vec<Source>,
    restore_token: Option<String>,
}

/// 【Portal捕获】【授权请求】按 proxy、token 与 include_cursor 创建会话，返回授权结果，失败先关闭已经创建的会话
async fn start(
    proxy: &Screencast,
    token: Option<&str>,
    include_cursor: bool,
) -> std::result::Result<(Session<Screencast>, Streams), ashpd::Error> {
    let session = proxy.create_session(Default::default()).await?;
    let result = async {
        proxy
            .select_sources(
                &session,
                SelectSourcesOptions::default()
                    .set_cursor_mode(if include_cursor {
                        CursorMode::Embedded
                    } else {
                        CursorMode::Hidden
                    })
                    .set_sources(Some(SourceType::Monitor.into()))
                    .set_multiple(true)
                    .set_restore_token(token)
                    .set_persist_mode(PersistMode::ExplicitlyRevoked),
            )
            .await?
            .response()?;
        proxy
            .start(&session, None, Default::default())
            .await?
            .response()
    }
    .await;
    match result {
        Ok(response) => Ok((session, response)),
        Err(error) => {
            if let Err(close_error) = session.close().await {
                log::error!("【Portal捕获】【授权失败清理】{close_error}");
            }
            Err(error)
        }
    }
}

impl PortalCapture {
    /// 【Portal捕获】【持续建立】按 outputs 与 include_cursor 建立一次授权及每个输出的唯一流，返回固定目标会话
    pub(super) async fn open(outputs: &[Output], include_cursor: bool) -> Result<Self> {
        if outputs.is_empty() {
            return Err("continuous portal capture needs an output".into());
        }
        let proxy = Screencast::new().await?;
        let saved = token::read_token();
        let (session, response) = match start(&proxy, saved.as_deref(), include_cursor).await {
            Err(ashpd::Error::Portal(ashpd::PortalError::InvalidArgument(error)))
                if saved.is_some() =>
            {
                log::info!("【Portal捕获】【恢复令牌】旧授权不可用: {error}");
                token::forget_token();
                start(&proxy, None, include_cursor).await?
            }
            result => result?,
        };
        let mut capture = Self {
            session: Some(session),
            sources: Vec::new(),
            restore_token: response.restore_token().map(str::to_owned),
        };
        let setup: Result<()> = async {
            let streams = response
                .streams()
                .iter()
                .map(|stream| StreamInfo {
                    node_id: stream.pipe_wire_node_id(),
                    size: stream.size(),
                    position: stream.position(),
                })
                .collect();
            let streams = reconcile(streams, outputs)?;
            let fd = proxy
                .open_pipe_wire_remote(capture.session.as_ref().unwrap(), Default::default())
                .await?;
            for (output, info) in streams {
                let stream = PipewireStream::new(info.node_id, fd.as_fd())?;
                capture.sources.push(Source {
                    output,
                    info,
                    stream,
                });
            }
            Ok(())
        }
        .await;
        if let Err(error) = setup {
            let _ = capture.close().await;
            return Err(error);
        }
        log::info!(
            "【Portal捕获】【持续建立】已绑定{}个输出，一次授权持续复用",
            capture.sources.len()
        );
        Ok(capture)
    }
}

#[async_trait]
impl CaptureSession for PortalCapture {
    /// 【Portal捕获】【下一帧】从既有流读取各输出，返回同一授权内的新帧，已关闭会话不重新申请权限
    async fn next_frame(&mut self) -> Result<CaptureResult> {
        if self.session.is_none() {
            return Err("the portal capture session is closed".into());
        }
        let mut frames = Vec::with_capacity(self.sources.len());
        for source in &mut self.sources {
            let frame = source.stream.next_frame().await?;
            frames.push(MonitorFrame {
                output: source.output,
                pixels: frame.pixels,
                pw_width: frame.width,
                pw_height: frame.height,
                pw_stride: frame.stride,
                info: StreamInfo {
                    node_id: source.info.node_id,
                    size: source.info.size,
                    position: source.info.position,
                },
            });
        }
        // 1. 【Portal捕获】【成功授权】只有真正收到原始像素后才保存恢复令牌
        if let Some(token) = self.restore_token.take() {
            token::write_token(&token);
        }
        Ok(CaptureResult { frames })
    }

    /// 【Portal捕获】【会话关闭】先回收全部视频线程再关闭授权会话，重复调用不再发送Close，无参数
    async fn close(&mut self) -> Result<()> {
        self.sources.clear();
        self.restore_token = None;
        if let Some(session) = self.session.take() {
            session.close().await?;
        }
        Ok(())
    }
}

impl Drop for PortalCapture {
    /// 【Portal捕获】【异常回收】调用方中途取消时回收视频线程并提交授权关闭，不创建新的授权会话，无参数
    fn drop(&mut self) {
        self.sources.clear();
        if let Some(session) = self.session.take()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            runtime.spawn(async move {
                if let Err(error) = session.close().await {
                    log::error!("【Portal捕获】【异常关闭】{error}");
                }
            });
        }
    }
}

/// 【Portal捕获】【输出核对】按 streams 与 outputs 精确匹配已授权位置，返回输出索引；没有位置仅在唯一来源与唯一目标时接受
fn reconcile(streams: Vec<StreamInfo>, outputs: &[Output]) -> Result<Vec<(usize, StreamInfo)>> {
    let positions = outputs
        .iter()
        .map(|output| output.info.logical_position.unwrap_or(output.info.location))
        .collect::<Vec<_>>();
    let mut matched = match_sources(streams, &positions)?;
    // 1. 【Portal捕获】【逻辑尺寸】Portal视频尺寸可以是物理像素，裁切比例必须来自原生输出逻辑尺寸
    for (index, info) in &mut matched {
        info.size = outputs[*index].info.logical_size.or(info.size);
        info.position = Some(positions[*index]);
    }
    Ok(matched)
}

/// 【Portal捕获】【唯一绑定】用 streams 与 positions 匹配已授权来源，返回无重复且完整的目标列表
fn match_sources(
    streams: Vec<StreamInfo>,
    positions: &[(i32, i32)],
) -> Result<Vec<(usize, StreamInfo)>> {
    let only_one = streams.len() == 1 && positions.len() == 1;
    let mut used = vec![false; positions.len()];
    let mut matched = Vec::new();
    for stream in streams {
        let index = positions
            .iter()
            .enumerate()
            .find(|(index, output)| {
                !used[*index]
                    && match stream.position {
                        Some(position) => **output == position,
                        None => only_one,
                    }
            })
            .map(|(index, _)| index);
        if let Some(index) = index {
            used[index] = true;
            matched.push((index, stream));
        }
    }
    if matched.is_empty() || matched.len() != positions.len() {
        return Err("portal streams do not uniquely cover the selected outputs".into());
    }
    matched.sort_by_key(|(index, _)| *index);
    Ok(matched)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
