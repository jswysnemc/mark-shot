//! 【门户录制】【授权绑定】自己持有授权会话与持续 PipeWire，不把来源选择交给外部录制器

use super::{
    Frame, Source, Step,
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
            // 1. 【门户录制】【独立连接】每次录制持有自己的总线连接，结束后不会遗留绑定旧运行时的全局连接
            let connection = zbus::Connection::session()
                .await
                .map_err(|e| e.to_string())?;
            let portal = Screencast::with_connection(connection)
                .await
                .map_err(|e| e.to_string())?;
            let session = Arc::new(
                portal
                    .create_session(Default::default())
                    .await
                    .map_err(|e| e.to_string())?,
            );
            let watched = session.clone();
            let ended = closed.clone();
            let monitor = tokio::spawn(async move {
                use futures::StreamExt;
                // 1. 【门户录制】【关闭监听】仅在确实收到桌面门户 Closed 信号时标记会话结束，避免订阅失败误报
                if let Ok(mut events) = watched.receive_closed().await
                    && events.next().await.is_some()
                {
                    ended.store(true, Ordering::Release);
                }
            });
            let result = cancellable(commands, async {
                portal
                    .select_sources(
                        &session,
                        SelectSourcesOptions::default()
                            .set_sources(Some(SourceType::Monitor.into()))
                            .set_multiple(false)
                            .set_cursor_mode(if cursor {
                                CursorMode::Embedded
                            } else {
                                CursorMode::Hidden
                            }),
                    )
                    .await
                    .map_err(|e| e.to_string())?
                    .response()
                    .map_err(|e| e.to_string())?;
                let response = portal
                    .start(&session, None, Default::default())
                    .await
                    .map_err(|e| e.to_string())?
                    .response()
                    .map_err(|e| e.to_string())?;
                if response.streams().len() != 1 {
                    return Err("请选择一个录制来源 / Select one recording source".into());
                }
                let source = &response.streams()[0];
                verify_source(
                    target,
                    &output,
                    source.position(),
                    source.size(),
                    expected_size,
                )?;
                let fd = portal
                    .open_pipe_wire_remote(&session, Default::default())
                    .await
                    .map_err(|e| e.to_string())?;
                Stream::new(
                    Input::Video {
                        node: source.pipe_wire_node_id(),
                        remote: fd,
                    },
                    60,
                )
            })
            .await;
            match result {
                Ok(stream) => Ok((session, stream, monitor)),
                Err(error) => {
                    monitor.abort();
                    let _ = monitor.await;
                    let _ = session.close().await;
                    Err(error)
                }
            }
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

/// 【门户录制】【来源校验】按 target、output、source_position、source_size 与 expected_physical 核对授权来源，兼容未提供坐标的门户
fn verify_source(
    target: &Target,
    output: &Output,
    source_position: Option<(i32, i32)>,
    source_size: Option<(i32, i32)>,
    expected_physical: Option<(u32, u32)>,
) -> Result<(), String> {
    if target.display.is_empty() && target.crop.is_none() {
        return Ok(());
    }
    // 1. 【门户录制】【位置校验】门户提供坐标时必须与目标显示器一致；KDE Plasma 6.3 等门户不返回 position 时放行给尺寸校验
    let expected_position = output.info.logical_position.unwrap_or(output.info.location);
    if let Some(actual_position) = source_position {
        if actual_position != expected_position {
            return Err("授权来源与选区显示器不一致，请重新选择 / Authorized source does not match the selected display".into());
        }
    } else {
        log::info!("【门户录制】【位置缺省】桌面门户未返回来源坐标，按尺寸继续核对授权来源");
    }
    // 2. 【门户录制】【尺寸校验】同时兼容逻辑尺寸、分数缩放 1px 舍入与物理像素尺寸，防止跨显示器误选
    let logical = output.logical_size();
    let physical = expected_physical.map(|(w, h)| (w as i32, h as i32));
    if let Some(actual_size) = source_size
        && (logical.is_some() || physical.is_some())
    {
        let matches_logical = logical.is_some_and(|(lw, lh)| {
            (actual_size.0 - lw).abs() <= 1 && (actual_size.1 - lh).abs() <= 1
        });
        let matches_physical = physical.is_some_and(|expected| actual_size == expected);
        if !matches_logical && !matches_physical {
            return Err(
                "授权来源尺寸与显示器不一致 / Authorized source geometry does not match".into(),
            );
        }
    }
    Ok(())
}

/// 【门户录制】【结束判定】按 has_frame 决定桌面关闭授权后的行为，已采集视频帧时优雅结束以完成封包保存
fn closed_step(has_frame: bool) -> Result<Step, String> {
    if has_frame {
        log::info!("【门户录制】【授权结束】桌面端已关闭屏幕共享授权，结束当前录制并保存");
        Ok(Step::Ended)
    } else {
        Err("录制授权已关闭 / Recording authorization closed".into())
    }
}

impl Source for Capture {
    /// 【门户录制】【持续等待】无参数，持续流可在编码期限前等待新画面
    fn asynchronous(&self) -> bool {
        true
    }
    /// 【门户录制】【连续取帧】按 timeout 等待持续流的新帧，桌面关闭共享且已有帧时返回结束以触发正常保存
    fn next(&mut self, timeout: Duration) -> Result<Step, String> {
        // 1. 【门户录制】【总线推进】持续驱动授权连接，撤销或关闭能及时进入会话状态
        self.runtime
            .block_on(async { tokio::task::yield_now().await });
        if self.closed.load(Ordering::Acquire) {
            return closed_step(!self.pixels.is_empty());
        }
        let size = match self
            .stream
            .as_ref()
            .unwrap()
            .video(&mut self.pixels, timeout)
        {
            Ok(size) => size,
            Err(error) => {
                // 2. 【门户录制】【并发关闭】桌面停止共享时 PipeWire 流断开可能略早于 Closed 信号，复查授权状态后优雅收尾
                self.runtime
                    .block_on(async { tokio::task::yield_now().await });
                if self.closed.load(Ordering::Acquire) {
                    return closed_step(!self.pixels.is_empty());
                }
                return Err(error);
            }
        };
        if let Some(size) = size {
            if self.expected_size.is_some_and(|expected| expected != size) {
                return Err("授权流像素尺寸与选区坐标不一致，请使用完整来源 / Portal pixel size does not match the selected region".into());
            }
            self.size = size;
            return Ok(Step::Frame);
        }
        Ok(Step::Wait)
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
    /// 【门户录制】【暂停控制】按 paused 停用持续流，桌面已关闭授权时直接成功以免阻断封装保存
    fn pause(&mut self, paused: bool) -> Result<(), String> {
        if self.closed.load(Ordering::Acquire) {
            return Ok(());
        }
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

    /// 【门户录制回归】【优雅收尾】无参数，已有帧后桌面关闭授权返回正常结束，零帧时保留授权关闭错误
    #[test]
    fn closed_authorization_finishes_when_frames_exist_and_errors_when_empty() {
        assert_eq!(super::closed_step(true), Ok(super::Step::Ended));
        assert!(
            super::closed_step(false)
                .unwrap_err()
                .contains("Recording authorization closed")
        );
    }

    /// 【门户录制回归】【来源校验】无参数，验证 Plasma 6.3 省略坐标时可通过尺寸核对，错屏坐标或错尺寸仍被拒绝
    #[test]
    fn verify_source_accepts_omitted_position_and_rejects_mismatched_display() {
        let mut info =
            crate::types::linux_output::Info::x11(1, "DP-1".into(), (0, 0), (2560, 1440));
        info.logical_size = Some((1707, 960));
        let output = crate::types::Output {
            wl_output: None,
            info,
        };
        let target = crate::features::recording::Target {
            display: "DP-1".into(),
            crop: Some([100, 100, 800, 600]),
            format: None,
            config: None,
        };
        // 1. KDE Plasma 6.3 省略 position，返回逻辑尺寸时应通过校验
        assert!(
            super::verify_source(
                &target,
                &output,
                None,
                Some((1707, 960)),
                Some((2560, 1440))
            )
            .is_ok()
        );
        // 2. 分数缩放 1px 舍入差异或直接返回物理像素尺寸时均应通过
        assert!(
            super::verify_source(
                &target,
                &output,
                None,
                Some((1706, 960)),
                Some((2560, 1440))
            )
            .is_ok()
        );
        assert!(
            super::verify_source(
                &target,
                &output,
                Some((0, 0)),
                Some((2560, 1440)),
                Some((2560, 1440))
            )
            .is_ok()
        );
        // 3. 明确返回不同位置或不同尺寸时必须拒绝
        assert!(
            super::verify_source(
                &target,
                &output,
                Some((1707, 0)),
                Some((1707, 960)),
                Some((2560, 1440))
            )
            .is_err()
        );
        assert!(
            super::verify_source(
                &target,
                &output,
                None,
                Some((1920, 1080)),
                Some((2560, 1440))
            )
            .is_err()
        );
    }
}
