//! 【门户录制】【授权绑定】自己持有授权会话与持续 PipeWire，不把来源选择交给外部录制器

use super::{
    Frame, Source,
    pipewire::{Input, Stream},
};
use crate::{features::recording::Target, types::Output};
use ashpd::desktop::{
    Session,
    screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

pub(super) struct Capture {
    stream: Option<Stream>,
    session: Option<Arc<Session<Screencast>>>,
    closed: Arc<AtomicBool>,
    monitor: Option<tokio::task::JoinHandle<()>>,
    runtime: tokio::runtime::Runtime,
    pixels: Vec<u8>,
    size: (u32, u32),
    expected_size: Option<(u32, u32)>,
}

impl Capture {
    /// 【门户录制】【真实来源】按 target、output、cursor 请求单一来源，指定目标必须与返回位置匹配
    pub fn new(
        target: &Target,
        output: Output,
        cursor: bool,
        commands: &std::sync::mpsc::Receiver<crate::features::recording::Command>,
    ) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let expected_size = if target.crop.is_some() {
            use wayland_client::protocol::wl_output::Transform as T;
            let mode = output
                .info
                .modes
                .iter()
                .find(|mode| mode.current)
                .ok_or("Portal crop requires a known display mode")?;
            let (width, height) = mode.dimensions;
            let rotated = matches!(
                output.info.transform,
                T::_90 | T::_270 | T::Flipped90 | T::Flipped270
            );
            Some(if rotated {
                (height as u32, width as u32)
            } else {
                (width as u32, height as u32)
            })
        } else {
            None
        };
        let closed = Arc::new(AtomicBool::new(false));
        let (session, stream, monitor) = runtime.block_on(async {
            let portal = Screencast::new().await.map_err(|e| e.to_string())?;
            let session = Arc::new(portal.create_session(Default::default()).await.map_err(|e| e.to_string())?);
            let watched = session.clone();
            let ended = closed.clone();
            let monitor = tokio::spawn(async move {
                use futures::StreamExt;
                if let Ok(mut events) = watched.receive_closed().await { events.next().await; }
                ended.store(true, Ordering::Release);
            });
            let result = cancellable(commands, async {
                portal.select_sources(&session, SelectSourcesOptions::default().set_sources(Some(SourceType::Monitor.into()))
                    .set_multiple(false).set_cursor_mode(if cursor { CursorMode::Embedded } else { CursorMode::Hidden }))
                    .await.map_err(|e| e.to_string())?.response().map_err(|e| e.to_string())?;
                let response = portal.start(&session, None, Default::default()).await.map_err(|e| e.to_string())?.response().map_err(|e| e.to_string())?;
                if response.streams().len() != 1 { return Err("请选择一个录制来源 / Select one recording source".into()); }
                let source = &response.streams()[0];
                if !target.display.is_empty() || target.crop.is_some() {
                    let position = output.info.logical_position.unwrap_or(output.info.location);
                    if source.position() != Some(position) {
                        return Err("授权来源与选区显示器不一致，请重新选择 / Authorized source does not match the selected display".into());
                    }
                    if source.size() != output.info.logical_size {
                        return Err("授权来源尺寸与显示器不一致 / Authorized source geometry does not match".into());
                    }
                }
                let fd = portal.open_pipe_wire_remote(&session, Default::default()).await.map_err(|e| e.to_string())?;
                Stream::new(Input::Video { node: source.pipe_wire_node_id(), remote: fd }, 60)
            }).await;
            match result { Ok(stream) => Ok((session, stream, monitor)), Err(error) => { monitor.abort(); let _ = monitor.await; let _ = session.close().await; Err(error) } }
        })?;
        Ok(Self {
            stream: Some(stream),
            session: Some(session),
            closed,
            monitor: Some(monitor),
            runtime,
            pixels: Vec::new(),
            size: (0, 0),
            expected_size,
        })
    }
}

/// 【门户录制】【授权取消】按 commands 等待 operation，停止或控制端关闭在一个检查周期内结束等待
async fn cancellable<T>(
    commands: &std::sync::mpsc::Receiver<crate::features::recording::Command>,
    operation: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tokio::pin!(operation);
    loop {
        tokio::select! {
            result = &mut operation => return result,
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                use std::sync::mpsc::TryRecvError;
                match commands.try_recv() {
                    Ok(crate::features::recording::Command::Stop) | Err(TryRecvError::Disconnected) => return Err("已取消录制授权 / Recording authorization cancelled".into()),
                    _ => {}
                }
            }
        }
    }
}

impl Source for Capture {
    /// 【门户录制】【持续等待】无参数，持续流可在编码期限前等待新画面
    fn asynchronous(&self) -> bool {
        true
    }
    /// 【门户录制】【连续取帧】按 timeout 等待持续流的新帧，不在每次读取后停流
    fn next(&mut self, timeout: Duration) -> Result<bool, String> {
        // 1. 【门户录制】【总线推进】持续驱动授权连接，撤销或关闭能及时进入会话状态
        self.runtime
            .block_on(async { tokio::task::yield_now().await });
        if self.closed.load(Ordering::Acquire) {
            return Err("录制授权已关闭 / Recording authorization closed".into());
        }
        let size = self
            .stream
            .as_ref()
            .unwrap()
            .video(&mut self.pixels, timeout)?;
        if let Some(size) = size {
            if self.expected_size.is_some_and(|expected| expected != size) {
                return Err("授权流像素尺寸与选区坐标不一致，请使用完整来源 / Portal pixel size does not match the selected region".into());
            }
            self.size = size;
        }
        Ok(size.is_some())
    }
    /// 【门户录制】【当前画面】无参数，借用最近完成的画面，静态流没有更新时保持原内容
    fn frame(&self) -> Option<Frame<'_>> {
        if self.pixels.is_empty() {
            return None;
        }
        Some(Frame {
            gpu: None,
            pixels: &self.pixels,
            size: self.size,
            stride: self.size.0 as usize * 4,
        })
    }
    /// 【门户录制】【暂停控制】按 paused 停用持续流，恢复时清理旧缓存
    fn pause(&mut self, paused: bool) -> Result<(), String> {
        self.stream.as_ref().unwrap().pause(paused)
    }
    /// 【门户录制】【路径诊断】无参数，返回当前实现的数据交付类型
    fn name(&self) -> &'static str {
        "Portal / PipeWire shared memory"
    }
}

impl Drop for Capture {
    /// 【门户录制】【会话释放】无参数，先退出视频线程，再关闭授权会话与运行时
    fn drop(&mut self) {
        self.stream.take();
        self.runtime.block_on(async {
            if let Some(monitor) = self.monitor.take() {
                monitor.abort();
                let _ = monitor.await;
            }
            if let Some(session) = self.session.take() {
                let _ = session.close().await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    /// 【门户录制回归】【停止响应】无参数，授权永不返回时停止仍能结束等待，不需要真实桌面弹窗
    #[test]
    fn pending_authorization_is_cancellable() {
        let (send, receive) = std::sync::mpsc::channel();
        send.send(crate::features::recording::Command::Stop)
            .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let result = runtime.block_on(super::cancellable::<()>(&receive, std::future::pending()));
        assert!(result.unwrap_err().contains("cancelled"));
    }
}
