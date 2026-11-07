//! 【单实例】【截图控制】异步捕获与原生窗口共用有限服务，延迟替换和活动窗口保护不增加常驻线程

pub(crate) mod context;
mod owner;
mod peer;
pub(crate) mod request;
mod server;
mod session_path;
mod state;

pub(crate) use owner::{Entry, Owner};
use request::{Plan, Request};
use rustix::event::PollFd;
use server::Server;
use std::{
    future::poll_fn,
    sync::{Mutex, MutexGuard},
    time::Instant,
};
use tokio::sync::Notify;

pub(crate) struct Controller {
    server: Mutex<Server>,
    state: Mutex<state::State>,
    changed: Notify,
}

pub(crate) struct PollGuard<'a> {
    controller: &'a Controller,
    server: MutexGuard<'a, Server>,
}

impl Controller {
    /// 【单实例】【控制构造】按 server 和初始 plan 建立独立控制器，返回可由启动与窗口共同持有的状态，不创建线程
    fn new(server: Server, plan: Plan) -> Self {
        Self {
            server: Mutex::new(server),
            state: Mutex::new(state::State::new(plan, Instant::now())),
            changed: Notify::new(),
        }
    }

    /// 【单实例】【请求交付】按 request 和 now 更新截图调度，返回有限 JSON 回复，延迟发生变化时只通知等待者
    fn request(&self, request: Request, now: Instant) -> Vec<u8> {
        let (reply, changed) = lock(&self.state).request(request, now);
        if changed {
            self.changed.notify_one();
        }
        reply
    }

    /// 【单实例】【服务处理】无参数，处理当前已到达或超时的连接，返回监听错误，不等待客户端补全输入
    pub fn progress(&self) -> crate::AppResult {
        let now = Instant::now();
        lock(&self.state).expire(now);
        lock(&self.server).progress(now, |request| self.request(request, now))
    }

    /// 【单实例】【等待借用】无参数，返回本次描述符范围；调用方等待结束后必须先释放范围，再处理到达请求
    pub fn poll_guard(&self) -> PollGuard<'_> {
        PollGuard {
            controller: self,
            server: lock(&self.server),
        }
    }

    /// 【单实例】【准备期服务】无参数，在捕获与延迟期间等待真实连接或必要期限，返回监听错误；原生窗口期间复用同一描述符
    pub async fn serve(&self) -> crate::AppResult {
        loop {
            self.progress()?;
            let deadline = self.poll_guard().deadline();
            let ready = poll_fn(|cx| lock(&self.server).ready(cx));
            if let Some(deadline) = deadline {
                tokio::select! {
                    _ = ready => {}
                    _ = tokio::time::sleep_until(deadline.into()) => {}
                }
            } else {
                ready.await;
            }
        }
    }

    /// 【单实例】【当前计划】无参数，返回尚未开始的最新延迟计划，已开始或关闭返回空值
    pub fn pending(&self) -> Option<Plan> {
        lock(&self.state).pending()
    }

    /// 【单实例】【开始确认】无参数，返回已经到期的最新截图参数；仍在等待或已关闭时返回空值
    pub fn begin(&self) -> Option<Plan> {
        lock(&self.state).begin(Instant::now())
    }

    /// 【单实例】【首帧确认】无参数、无返回值，截图就绪后启用活动保护，不触发其他截图
    pub fn active(&self) {
        lock(&self.state).active();
    }

    /// 【单实例】【调度通知】无参数，等待下一次有效延迟替换，不进行周期检查，无返回值
    pub async fn changed(&self) {
        self.changed.notified().await;
    }

    /// 【单实例】【占用结束】先释放连接再标记关闭，无参数、无返回值；文件锁由所属 Owner 最后释放
    fn close(&self) {
        lock(&self.server).close();
        lock(&self.state).close();
        self.changed.notify_one();
    }
}

impl PollGuard<'_> {
    /// 【单实例】【描述符序号】无参数，返回接入、释放或方向切换后的变更序号，未变化时事件源可以保留登记
    pub fn generation(&self) -> u64 {
        self.server.generation
    }

    /// 【单实例】【登记副本】无参数，返回可由事件源独立持有的描述符和写方向，注销后必须释放副本
    pub fn duplicate_fds(&self) -> std::io::Result<Vec<(std::os::fd::OwnedFd, bool)>> {
        self.server.duplicate_fds()
    }

    /// 【单实例】【等待期限】无参数，返回客户端回收或活动重复请求的最近期限，普通空闲截图保持无限等待
    pub fn deadline(&self) -> Option<Instant> {
        earliest(
            self.server.deadline(),
            lock(&self.controller.state).deadline(),
        )
    }

    /// 【单实例】【附加描述符】按 fds 追加当前接入与客户端读写兴趣，无返回值，整个等待必须持有本范围
    pub fn poll_fds<'a>(&'a self, fds: &mut Vec<PollFd<'a>>) {
        self.server.poll_fds(fds);
    }
}

/// 【单实例】【状态借用】按 value 返回互斥范围，异常退出后的清理仍能取得资源，不把中毒状态永久保留
fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// 【单实例】【最早期限】按 left 与 right 返回非空期限的较早值，两者为空时保持无限等待
fn earliest(left: Option<Instant>, right: Option<Instant>) -> Option<Instant> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (left, right) => left.or(right),
    }
}

#[cfg(test)]
mod tests;
