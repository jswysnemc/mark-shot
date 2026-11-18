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

struct Source {
    output: usize,
    info: StreamInfo,
    stream: PipewireStream,
}

pub(super) struct PortalCapture {
    session: Option<Session<Screencast>>,
    sources: Vec<Source>,
    restore_token: Option<String>,
    first_frame_after: Option<tokio::time::Instant>,
}

struct Authorized<S> {
    session: S,
    streams: Vec<StreamInfo>,
    restore_token: Option<String>,
}

#[async_trait]
trait Authorizer: Send + Sync {
    type Session: Send;

    /// 【Portal捕获】【执行授权】按 token 请求桌面门户选择来源并启动会话，返回会话与流描述
    async fn start(
        &self,
        token: Option<&str>,
    ) -> std::result::Result<Authorized<Self::Session>, ashpd::Error>;

    /// 【Portal捕获】【丢弃会话】关闭不再使用的门户会话 handle，无返回值
    async fn discard(&self, handle: Self::Session);

    /// 【Portal捕获】【清除令牌】删除本地失效的恢复令牌，无参数、无返回值
    fn forget_token(&self);

    /// 【Portal捕获】【刷新输出】按 selected 重新读取合成器当前输出描述，返回对齐后的输出列表
    fn refresh_outputs(&self, selected: &[Output]) -> Option<Vec<Output>>;
}

struct LiveAuthorizer<'a> {
    proxy: &'a Screencast,
    connection: Option<&'a wayland_client::Connection>,
    include_cursor: bool,
}

#[async_trait]
impl Authorizer for LiveAuthorizer<'_> {
    type Session = Session<Screencast>;

    /// 【Portal捕获】【门户启动】按 token 调用真实 Screencast 门户并采集返回流描述，返回已启动会话
    async fn start(
        &self,
        token: Option<&str>,
    ) -> std::result::Result<Authorized<Self::Session>, ashpd::Error> {
        let (session, response) = start(self.proxy, token, self.include_cursor).await?;
        let streams = response
            .streams()
            .iter()
            .map(|stream| StreamInfo {
                node_id: stream.pipe_wire_node_id(),
                size: stream.size(),
                position: stream.position(),
            })
            .collect();
        Ok(Authorized {
            session,
            streams,
            restore_token: response.restore_token().map(str::to_owned),
        })
    }

    /// 【Portal捕获】【真实回收】关闭被放弃的真实门户会话 handle，失败时记录错误日志，无返回值
    async fn discard(&self, handle: Self::Session) {
        if let Err(error) = handle.close().await {
            log::error!("【Portal捕获】【旧授权关闭】{error}");
        }
    }

    /// 【Portal捕获】【真实清令牌】清理状态目录与旧目录中的恢复令牌文件，无参数、无返回值
    fn forget_token(&self) {
        token::forget_token();
    }

    /// 【Portal捕获】【真实输出刷新】通过 Wayland 连接拉取最新输出几何并与 selected 对齐，返回更新列表
    fn refresh_outputs(&self, selected: &[Output]) -> Option<Vec<Output>> {
        refresh_outputs(self.connection?, selected)
    }
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

/// 【Portal捕获】【授权协商】按 authorizer、saved 令牌与 outputs 完成授权与位置核对，返回有效会话、匹配流及新令牌
async fn negotiate<A: Authorizer>(
    authorizer: &A,
    saved: Option<&str>,
    outputs: &[Output],
) -> Result<(A::Session, Vec<(usize, StreamInfo)>, Option<String>)> {
    // 1. 【Portal捕获】【初始授权】优先复用本地恢复令牌，门户明确拒绝参数时清除令牌并转入交互式选择
    let mut used_saved = saved.is_some();
    let mut authorized = match authorizer.start(saved).await {
        Err(ashpd::Error::Portal(ashpd::PortalError::InvalidArgument(error))) if used_saved => {
            log::info!("【Portal捕获】【恢复令牌】旧授权不可用: {error}");
            authorizer.forget_token();
            used_saved = false;
            authorizer.start(None).await?
        }
        result => result?,
    };
    let mut resolved = outputs.to_vec();
    let try_reconcile = |streams: Vec<StreamInfo>, current: &mut Vec<Output>| {
        match reconcile(streams.clone(), current) {
            Ok(matched) => Ok(matched),
            Err(error) => {
                // 2. 【Portal捕获】【热插拔同步】外接屏刚接入时授权弹窗期间布局才稳定，先按最新输出位置重试精确核对
                if let Some(refreshed) = authorizer.refresh_outputs(current) {
                    *current = refreshed;
                    return reconcile(streams, current);
                }
                Err(error)
            }
        }
    };
    match try_reconcile(authorized.streams.clone(), &mut resolved) {
        Ok(matched) => Ok((authorized.session, matched, authorized.restore_token)),
        Err(error) if used_saved => {
            // 3. 【Portal捕获】【令牌失效重试】显示器数量或目标变化后旧令牌仅返回历史子集，销毁旧会话并重新请求完整授权
            log::info!("【Portal捕获】【恢复令牌】已保存授权未覆盖当前输出，重新请求授权: {error}");
            authorizer.discard(authorized.session).await;
            authorizer.forget_token();
            authorized = authorizer.start(None).await?;
            match try_reconcile(authorized.streams, &mut resolved) {
                Ok(matched) => Ok((authorized.session, matched, authorized.restore_token)),
                Err(retry_error) => {
                    authorizer.discard(authorized.session).await;
                    Err(retry_error)
                }
            }
        }
        Err(error) => {
            authorizer.discard(authorized.session).await;
            Err(error)
        }
    }
}

impl PortalCapture {
    /// 【Portal捕获】【持续建立】按 outputs、connection 与 include_cursor 建立唯一流会话，返回固定目标捕获会话
    pub(super) async fn open(
        outputs: &[Output],
        connection: Option<&wayland_client::Connection>,
        include_cursor: bool,
    ) -> Result<Self> {
        if outputs.is_empty() {
            return Err("continuous portal capture needs an output".into());
        }
        // 1. 【Portal捕获】【独立总线】授权会话使用当前运行时创建的连接，避免其他临时门户操作影响截图
        let bus = zbus::Connection::session().await?;
        let proxy = Screencast::with_connection(bus).await?;
        let saved = token::read_token();
        let authorizer = LiveAuthorizer {
            proxy: &proxy,
            connection,
            include_cursor,
        };
        let (session, streams, restore_token) =
            negotiate(&authorizer, saved.as_deref(), outputs).await?;
        let mut capture = Self {
            session: Some(session),
            sources: Vec::new(),
            restore_token,
            first_frame_after: Some(
                tokio::time::Instant::now() + std::time::Duration::from_millis(350),
            ),
        };
        let setup: Result<()> = async {
            for (output, info) in streams {
                // 1. 【Portal捕获】【独立连接】每个原生客户端需要独立套接字，复制描述符不能隔离PipeWire协议状态
                let fd = proxy
                    .open_pipe_wire_remote(
                        capture
                            .session
                            .as_ref()
                            .ok_or("portal authorization is closed")?,
                        Default::default(),
                    )
                    .await?;
                let stream = PipewireStream::new(info.node_id, fd)?;
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
        // 1. 【Portal捕获】【授权过渡】Start响应先于GNOME选择窗口关闭动画，首次激活流前留出动画结束时间
        if let Some(deadline) = self.first_frame_after {
            tokio::time::sleep_until(deadline).await;
            self.first_frame_after = None;
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
        // 2. 【Portal捕获】【成功授权】只有真正收到原始像素后才保存恢复令牌
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

/// 【Portal捕获】【输出刷新】按 connection 与 selected 重新读取合成器当前输出描述，全部命中时返回最新列表
fn refresh_outputs(
    connection: &wayland_client::Connection,
    selected: &[Output],
) -> Option<Vec<Output>> {
    let runtime = crate::backend::wayland::overlay::state::OverlayRunTime::new(connection).ok()?;
    merge_refreshed_outputs(selected, &runtime.state.outputs)
}

/// 【Portal捕获】【身份对齐】按 selected 的协议编号或输出名称从 latest 更新几何信息，缺失任一目标时返回空值
fn merge_refreshed_outputs(selected: &[Output], latest: &[Output]) -> Option<Vec<Output>> {
    selected
        .iter()
        .map(|target| {
            let updated = latest.iter().find(|candidate| {
                candidate.info.id == target.info.id
                    || (target.info.name.is_some() && candidate.info.name == target.info.name)
            })?;
            Some(Output {
                wl_output: target.wl_output.clone(),
                info: updated.info.clone(),
            })
        })
        .collect()
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
        info.size = outputs[*index].logical_size().or(info.size);
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
