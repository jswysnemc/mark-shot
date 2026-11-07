//! 【单实例】【连接收发】单个客户端只接收一条有限 JSON 请求，非阻塞回复后立即释放

use super::request::{MAX_PACKET, Request, response};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use rustix::event::{PollFd, PollFlags};
use std::{
    io::{self, Read, Write},
    os::fd::{AsFd, OwnedFd},
    os::unix::net::UnixStream,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::io::unix::AsyncFd;

pub(super) struct Peer {
    stream: AsyncFd<UnixStream>,
    input: Vec<u8>,
    output: Option<Vec<u8>>,
    written: usize,
    pub deadline: Instant,
}

impl Peer {
    /// 【单实例】【当前方向】无参数，返回连接是否正在等待写入回复，供注册描述符时选择正确兴趣
    pub fn writing(&self) -> bool {
        self.output.is_some()
    }

    /// 【单实例】【独立登记】无参数，返回保持同一连接有效的复制描述符，事件源必须注销后再关闭复制句柄
    pub fn duplicate_fd(&self) -> io::Result<OwnedFd> {
        self.stream.get_ref().as_fd().try_clone_to_owned()
    }

    /// 【单实例】【客户端认证】按 stream、uid 和 now 创建有限连接，返回内核身份或注册错误，不创建工作线程
    pub fn new(stream: UnixStream, uid: u32, now: Instant) -> crate::AppResult<Self> {
        let credentials = getsockopt(&stream, PeerCredentials)?;
        if credentials.uid() != uid || credentials.pid() <= 0 {
            return Err("single-instance client does not belong to this user".into());
        }
        stream.set_nonblocking(true)?;
        Ok(Self {
            // 1. 【单实例】【异步登记】本对象独占原始描述符，登记期间不替换资源，复制句柄不会关闭原始连接
            stream: unsafe { AsyncFd::register(stream) }.map_err(|error| error.into_parts().1)?,
            input: Vec::with_capacity(MAX_PACKET + 1),
            output: None,
            written: 0,
            deadline: now + Duration::from_secs(1),
        })
    }

    /// 【单实例】【原生等待】无额外参数，返回当前需要读或写的描述符，连接仍由 self 持有
    pub fn poll_fd(&self) -> PollFd<'_> {
        PollFd::new(
            self.stream.get_ref(),
            if self.output.is_some() {
                PollFlags::OUT
            } else {
                PollFlags::IN
            },
        )
    }

    /// 【单实例】【异步等待】按 cx 注册当前读写兴趣，返回就绪或等待；随后必须将对应方向处理至完成或 WouldBlock
    pub fn ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        let ready = if self.output.is_some() {
            self.stream.poll_write_ready(cx)
        } else {
            self.stream.poll_read_ready(cx)
        };
        match ready {
            Poll::Ready(Ok(mut guard)) => {
                guard.clear_ready();
                Poll::Ready(())
            }
            Poll::Ready(Err(_)) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }

    /// 【单实例】【连接推进】按 now 和 handler 读取一条命令并发送回复，返回是否继续持有；超时、完成或断连后释放
    pub fn progress(
        &mut self,
        now: Instant,
        handler: &mut impl FnMut(Request) -> Vec<u8>,
    ) -> io::Result<bool> {
        if now >= self.deadline {
            return Ok(false);
        }
        // 1. 【单实例】【输入上限】一次连接只接收一个换行 JSON，分配与正文均有固定上限
        if self.output.is_none() {
            let mut buffer = [0; 1024];
            loop {
                let remaining = MAX_PACKET + 1 - self.input.len();
                match self
                    .stream
                    .get_ref()
                    .read(&mut buffer[..remaining.min(1024)])
                {
                    Ok(0) => return Ok(false),
                    Ok(count) => {
                        self.input.extend_from_slice(&buffer[..count]);
                        if self.input.len() > MAX_PACKET {
                            self.output = Some(response(
                                false,
                                "single-instance request exceeds its size limit",
                            ));
                            break;
                        }
                        if let Some(end) = self.input.iter().position(|byte| *byte == b'\n') {
                            let reply = match serde_json::from_slice::<Request>(&self.input[..end])
                            {
                                Ok(request) => handler(request),
                                Err(_) => response(false, "invalid single-instance command"),
                            };
                            self.output = Some(reply);
                            self.input.clear();
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(true),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }
        }
        // 2. 【单实例】【有限回复】只有完整请求才触发业务，慢接收方不会阻塞截图输入
        let output = self.output.as_ref().unwrap();
        while self.written < output.len() {
            match self.stream.get_ref().write(&output[self.written..]) {
                Ok(0) => return Ok(false),
                Ok(count) => self.written += count,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(true),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(false)
    }
}
