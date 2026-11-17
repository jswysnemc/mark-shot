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

pub struct PortalCapture {
    session: Option<Session<Screencast>>,
    sources: Vec<Source>,
    restore_token: Option<String>,
    first_frame_after: Option<tokio::time::Instant>,
    /// 【Portal捕获】【待绑定授权】已经拿到的 Start 响应；在捕获真正开始前就授权时，流要等输出列表确定后才绑定。
    /// 它和 `session` 成对出现：有 session 有它才算授权完整，绑定后立即清空。
    pending: Option<Streams>,
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
    /// 【Portal捕获】【仅授权】按 include_cursor 走完选择来源和 Start，返回尚未绑定输出的会话。
    ///
    /// 截图入口在拿到显示器列表之前调用它：用户先开机、再插外接屏或改显示器排列时，
    /// 合成器的布局通知往往和授权窗口的关闭同时到达，先授权能保证随后读到的
    /// 位置和数量是当前布局，而不是上一版桌面。这里不绑定流，绑定必须针对最终的输出列表。
    pub(super) async fn authorize(include_cursor: bool) -> Result<Self> {
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
        log::info!("【Portal捕获】【授权完成】等待绑定输出");
        Ok(Self {
            session: Some(session),
            sources: Vec::new(),
            restore_token: response.restore_token().map(str::to_owned),
            first_frame_after: Some(
                tokio::time::Instant::now() + std::time::Duration::from_millis(350),
            ),
            pending: Some(response),
        })
    }

    /// 【Portal捕获】【绑定输出】按 outputs 把已授权流接到各输出，返回绑定结果。
    ///
    /// 绑定失败会关闭授权会话，避免留下没人使用的屏幕共享。
    pub(super) async fn attach(&mut self, outputs: &[Output]) -> Result<()> {
        let Some(response) = self.pending.take() else {
            return Ok(());
        };
        let proxy = Screencast::new().await?;
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
            for (output, info) in streams {
                // 1. 【Portal捕获】【独立连接】每个原生客户端需要独立套接字，复制描述符不能隔离PipeWire协议状态
                let fd = proxy
                    .open_pipe_wire_remote(
                        self.session
                            .as_ref()
                            .ok_or("portal authorization is closed")?,
                        Default::default(),
                    )
                    .await?;
                let stream = PipewireStream::new(info.node_id, fd)?;
                self.sources.push(Source {
                    output,
                    info,
                    stream,
                });
            }
            Ok(())
        }
        .await;
        if let Err(error) = setup {
            let _ = self.close().await;
            return Err(error);
        }
        log::info!(
            "【Portal捕获】【持续建立】已绑定{}个输出，一次授权持续复用",
            self.sources.len()
        );
        Ok(())
    }

    /// 【Portal捕获】【持续建立】按 outputs 与 include_cursor 建立一次授权及每个输出的唯一流，返回固定目标会话
    pub(super) async fn open(outputs: &[Output], include_cursor: bool) -> Result<Self> {
        if outputs.is_empty() {
            return Err("continuous portal capture needs an output".into());
        }
        let mut capture = Self::authorize(include_cursor).await?;
        capture.attach(outputs).await?;
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
        // 1. 【Portal捕获】【授权归还】尚未绑定的授权也必须丢掉，否则提前授权的会话会一直挂在合成器上
        self.pending = None;
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
        self.pending = None;
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

/// 【Portal捕获】【输出核对】按 streams 与 outputs 对齐授权来源，返回输出索引。
///
/// 除了“对不上就报错”，这里还要处理一种很常见的正常情况：用户在开机后才插上 HDMI 或改过显示器排列。
/// 那时本地 wl_output 快照是旧的（位置、数量都可能是上一版布局），而 Portal 返回的是当前真实布局。
/// 所以：
/// - 位置相等仍然是最可信的绑定；
/// - 整体对不上且数量一一对应时按数量绑定，具体规则见 match_sources；
/// - 只有确实无法一一对应时才失败，失败信息带上两边的原始位置，方便直接定位。
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

/// 【Portal捕获】【唯一绑定】用 streams 与 positions 匹配已授权来源，返回按输出排序的完整目标列表。
///
/// 匹配顺序体现可信度：位置相等是最强的对应关系，先用干净；整体对不上时才考虑数量兜底。
/// 数量兜底要求满足全部条件才生效：没有任何一条流按位置命中、剩余流与剩余输出数量一致、
/// 且所有剩余流都带有位置。没有位置的流无法提供对应依据，混进来就只能放弃猜测；
/// 只对上一版布局这种“整体平移”放行，部分命中部分落空的情况宁可报错也不猜。
fn match_sources(
    streams: Vec<StreamInfo>,
    positions: &[(i32, i32)],
) -> Result<Vec<(usize, StreamInfo)>> {
    let only_one = streams.len() == 1 && positions.len() == 1;
    let mut used = vec![false; positions.len()];
    let mut matched = Vec::new();
    let mut unresolved = Vec::new();
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
        match index {
            Some(index) => {
                used[index] = true;
                matched.push((index, stream));
            }
            None => unresolved.push(stream),
        }
    }
    // 【Portal捕获】【数量兜底】本地快照整体过期时按数量顺序补齐：用户先开机再插显示器，
    // 本地列表是旧布局、portal 返回的是新布局，两边数量一致而位置整体偏移，
    // 这时按数量补齐比直接失败更有用。部分命中部分落空说明不是单纯的整体平移，
    // 按顺序硬绑会把别的屏幕的画面当成选中的屏幕，保持失败更安全
    if matched.is_empty()
        && !unresolved.is_empty()
        && unresolved.len() == positions.len()
        && unresolved.iter().all(|stream| stream.position.is_some())
    {
        for (index, stream) in unresolved.into_iter().enumerate() {
            log::info!(
                "【Portal捕获】【数量兜底】输出位置 {:?} 与流位置 {:?} 不一致，按数量顺序绑定",
                positions[index],
                stream.position
            );
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
