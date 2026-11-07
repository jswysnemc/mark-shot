//! 【单实例】【非阻塞服务】有限连接同时参与异步启动等待和原生窗口等待

use super::{peer::Peer, request::Request, session_path::SessionPath};
use rustix::event::{PollFd, PollFlags};
use std::{
    fs,
    os::fd::{AsFd, OwnedFd},
    os::unix::{fs::PermissionsExt, net::UnixListener},
    task::{Context, Poll},
    time::Instant,
};
use tokio::io::unix::AsyncFd;

const MAX_PEERS: usize = 8;

pub(super) struct Server {
    listener: Option<AsyncFd<UnixListener>>,
    peers: Vec<Peer>,
    path: SessionPath,
    identity: Option<(u64, u64)>,
    pub generation: u64,
}

impl Server {
    /// 【单实例】【服务绑定】按 path 绑定已取得占用权的私有地址，返回服务或系统错误，不读取其他会话
    pub fn new(path: SessionPath) -> crate::AppResult<Self> {
        let listener = UnixListener::bind(&path.socket)?;
        let identity = path
            .socket_identity()?
            .ok_or("single-instance socket was not created")?;
        let registered = (|| -> crate::AppResult<_> {
            fs::set_permissions(&path.socket, fs::Permissions::from_mode(0o600))?;
            listener.set_nonblocking(true)?;
            // 1. 【单实例】【监听拥有】服务独占监听描述符，注销完成后才释放原始监听连接
            Ok(unsafe { AsyncFd::register(listener) }.map_err(|error| error.into_parts().1)?)
        })();
        let listener = match registered {
            Ok(listener) => listener,
            Err(error) => {
                path.remove_socket(identity);
                return Err(error);
            }
        };
        Ok(Self {
            listener: Some(listener),
            peers: Vec::with_capacity(MAX_PEERS),
            path,
            identity: Some(identity),
            generation: 1,
        })
    }

    /// 【单实例】【服务推进】按 now 和 handler 接收及处理有限请求，无返回值；客户端错误仅释放对应连接
    pub fn progress(
        &mut self,
        now: Instant,
        mut handler: impl FnMut(Request) -> Vec<u8>,
    ) -> crate::AppResult {
        let Some(listener) = &self.listener else {
            return Ok(());
        };
        let mut changed = false;
        // 1. 【单实例】【接入范围】清空已经到达的接入队列，超出八个连接的客户端立即关闭，不造成常驻忙等待
        loop {
            match listener.get_ref().accept() {
                Ok((stream, _)) if self.peers.len() < MAX_PEERS => {
                    match Peer::new(stream, self.path.uid, now) {
                        Ok(peer) => {
                            self.peers.push(peer);
                            changed = true;
                        }
                        Err(error) => log::info!("【单实例】【客户端接入】连接被拒绝：{error}"),
                    }
                }
                Ok((stream, _)) => drop(stream),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        // 2. 【单实例】【连接回收】完成、超时和断开的连接立即释放，不保留空闲客户端
        self.peers.retain_mut(|peer| {
            let writing = peer.writing();
            match peer.progress(now, &mut handler) {
                Ok(keep) => {
                    changed |= !keep || peer.writing() != writing;
                    keep
                }
                Err(error) => {
                    log::info!("【单实例】【客户端收发】连接结束：{error}");
                    changed = true;
                    false
                }
            }
        });
        if changed {
            self.generation = self.generation.wrapping_add(1);
        }
        Ok(())
    }

    /// 【单实例】【事件源句柄】无参数，返回当前描述符的独立副本和读写方向；副本在注销前保持内核连接有效
    pub fn duplicate_fds(&self) -> std::io::Result<Vec<(OwnedFd, bool)>> {
        let mut descriptors = Vec::with_capacity(self.peers.len() + 1);
        if let Some(listener) = &self.listener {
            descriptors.push((listener.get_ref().as_fd().try_clone_to_owned()?, false));
        }
        for peer in &self.peers {
            descriptors.push((peer.duplicate_fd()?, peer.writing()));
        }
        Ok(descriptors)
    }

    /// 【单实例】【原生描述符】按 fds 追加本服务当前读写兴趣，无返回值，调用方必须在等待结束后释放借用
    pub fn poll_fds<'a>(&'a self, fds: &mut Vec<PollFd<'a>>) {
        if let Some(listener) = &self.listener {
            fds.push(PollFd::new(listener.get_ref(), PollFlags::IN));
        }
        fds.extend(self.peers.iter().map(Peer::poll_fd));
    }

    /// 【单实例】【异步就绪】按 cx 注册全部接入与客户端兴趣，任一方向到达时返回就绪，不创建计时轮询
    pub fn ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        let mut ready = false;
        if let Some(listener) = &self.listener {
            match listener.poll_read_ready(cx) {
                Poll::Ready(Ok(mut guard)) => {
                    guard.clear_ready();
                    ready = true;
                }
                Poll::Ready(Err(_)) => ready = true,
                Poll::Pending => {}
            }
        }
        for peer in &self.peers {
            ready |= peer.ready(cx).is_ready();
        }
        if ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    /// 【单实例】【连接期限】无参数，返回最近客户端回收期限，没有客户端时不启用计时器
    pub fn deadline(&self) -> Option<Instant> {
        self.peers.iter().map(|peer| peer.deadline).min()
    }

    /// 【单实例】【服务关闭】断开本次全部连接并删除对应 inode 的套接字，无参数、无返回值，锁由占用者释放
    pub fn close(&mut self) {
        if self.listener.is_some() {
            self.generation = self.generation.wrapping_add(1);
        }
        self.peers.clear();
        self.listener = None;
        if let Some(identity) = self.identity.take() {
            self.path.remove_socket(identity);
        }
    }
}

impl Drop for Server {
    /// 【单实例】【服务析构】回收本次文件描述符和套接字路径，无参数、无返回值
    fn drop(&mut self) {
        self.close();
    }
}
