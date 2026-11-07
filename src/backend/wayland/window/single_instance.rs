//! 【原生窗口】【单实例接入】辅助窗口等待控制连接，登记副本避免业务回收与事件源关闭发生竞争

use super::runtime::NativeWindow;
use crate::features::single_instance::Controller;
use smithay_client_toolkit::reexports::calloop::{
    EventSource, Interest, LoopHandle, Mode, Poll, PostAction, Readiness, RegistrationToken, Token,
    TokenFactory, generic::Generic,
};
use std::{io, os::fd::OwnedFd, sync::Arc, time::Instant};

pub(super) struct Binding {
    controller: Arc<Controller>,
    token: RegistrationToken,
    generation: u64,
}

struct ControlSource {
    controller: Arc<Controller>,
    descriptors: Vec<Generic<OwnedFd>>,
    generation: u64,
}

impl Binding {
    /// 【原生窗口】【控制登记】按 handle 和 controller 登记本次事件来源，返回自动随事件循环回收的绑定，不创建线程
    pub fn install(
        handle: &LoopHandle<'static, NativeWindow>,
        controller: Arc<Controller>,
    ) -> crate::AppResult<Self> {
        let (descriptors, generation) = descriptors(&controller)?;
        let token = handle
            .insert_source(
                ControlSource {
                    controller: controller.clone(),
                    descriptors,
                    generation,
                },
                |_, _, _| {},
            )
            .map_err(|error| error.error)?;
        Ok(Self {
            controller,
            token,
            generation,
        })
    }

    /// 【原生窗口】【控制期限】无参数，返回客户端或重复请求最近期限，没有必要期限时保持无限等待
    pub fn deadline(&self) -> Option<Instant> {
        self.controller.poll_guard().deadline()
    }

    /// 【原生窗口】【连接推进】按 handle 处理当前到达与超时连接，描述符变化后才更新登记，返回服务或事件循环错误
    pub fn progress(&mut self, handle: &LoopHandle<'static, NativeWindow>) -> crate::AppResult {
        self.controller.progress()?;
        let generation = self.controller.poll_guard().generation();
        if generation != self.generation {
            handle.update(&self.token)?;
            self.generation = generation;
        }
        Ok(())
    }
}

impl EventSource for ControlSource {
    type Event = ();
    type Metadata = ();
    type Ret = ();
    type Error = io::Error;

    /// 【原生窗口】【控制事件】按 readiness 和 token 处理已登记方向，callback 只接收完成通知，返回是否需要更新描述符
    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut((), &mut ()),
    {
        let mut ours = false;
        for descriptor in &mut self.descriptors {
            descriptor.process_events(readiness, token, |_, _| {
                ours = true;
                Ok(PostAction::Continue)
            })?;
        }
        if ours {
            self.controller
                .progress()
                .map_err(|error| io::Error::other(error.to_string()))?;
            callback((), &mut ());
        }
        if self.controller.poll_guard().generation() != self.generation {
            Ok(PostAction::Reregister)
        } else {
            Ok(PostAction::Continue)
        }
    }

    /// 【原生窗口】【首次登记】按 poll 和 token_factory 登记持有的全部独立描述符，返回登记错误
    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> smithay_client_toolkit::reexports::calloop::Result<()> {
        for descriptor in &mut self.descriptors {
            descriptor.register(poll, token_factory)?;
        }
        Ok(())
    }

    /// 【原生窗口】【登记更新】按 poll 和 token_factory 先注销旧副本再登记当前方向，未变化时保留原来源，返回错误
    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> smithay_client_toolkit::reexports::calloop::Result<()> {
        let (descriptors, generation) = descriptors(&self.controller)?;
        if generation == self.generation {
            return Ok(());
        }
        self.unregister(poll)?;
        self.descriptors = descriptors;
        self.generation = generation;
        self.register(poll, token_factory)
    }

    /// 【原生窗口】【来源注销】按 poll 注销全部仍由本来源持有的副本，返回注销错误，随后才允许关闭描述符
    fn unregister(
        &mut self,
        poll: &mut Poll,
    ) -> smithay_client_toolkit::reexports::calloop::Result<()> {
        for descriptor in &mut self.descriptors {
            descriptor.unregister(poll)?;
        }
        Ok(())
    }
}

/// 【原生窗口】【描述符快照】按 controller 返回同次借用中的描述符副本及变更序号，副本由标准事件源管理关闭
fn descriptors(controller: &Controller) -> io::Result<(Vec<Generic<OwnedFd>>, u64)> {
    let guard = controller.poll_guard();
    let descriptors = guard
        .duplicate_fds()?
        .into_iter()
        .map(|(fd, writing)| {
            Generic::new(
                fd,
                if writing {
                    Interest::WRITE
                } else {
                    Interest::READ
                },
                Mode::Level,
            )
        })
        .collect();
    Ok((descriptors, guard.generation()))
}
