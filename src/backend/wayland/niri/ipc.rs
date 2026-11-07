//! 【niri】【窗口信息】同一合成器内的有界 IPC 请求与输出身份

use std::{
    os::fd::AsFd,
    path::{Path, PathBuf},
};

use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};
use wayland_client::Connection;

use super::Result;

const MAX_REPLY: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Peer {
    pid: i32,
    uid: u32,
}

impl Peer {
    /// 【niri】【服务身份】读取 connection 的 Wayland 服务进程与用户，无其他参数；返回内核认证身份
    pub(crate) fn wayland(connection: &Connection) -> Result<Self> {
        Self::read(&connection.backend().poll_fd())
    }

    /// 【niri】【对端认证】读取 socket 的内核身份，不按套接字名称猜测；返回有效进程身份或错误
    pub(crate) fn read(socket: &impl AsFd) -> Result<Self> {
        let credentials = getsockopt(socket, PeerCredentials)?;
        if credentials.pid() <= 0 {
            return Err("the compositor peer has no process identity".into());
        }
        Ok(Self {
            pid: credentials.pid(),
            uid: credentials.uid(),
        })
    }
}

pub(crate) struct Client {
    reader: BufReader<UnixStream>,
    path: PathBuf,
    peer: Peer,
}

impl Client {
    /// 【niri】【连接校验】连接 path 并核对 expected 身份，返回独立 IPC 客户端；不执行截图或桌面动作
    pub(crate) async fn connect(path: &Path, expected: Peer) -> Result<Self> {
        let stream = UnixStream::connect(path).await?;
        if Peer::read(&stream)? != expected {
            return Err(
                "NIRI_SOCKET and the Wayland connection belong to different compositor processes"
                    .into(),
            );
        }
        Ok(Self {
            reader: BufReader::new(stream),
            path: path.to_owned(),
            peer: expected,
        })
    }

    /// 【niri】【事件连接】按本客户端已校验的路径与身份建立独立连接，无参数；不重新读取环境变量
    pub(crate) async fn separate(&self) -> Result<Self> {
        Self::connect(&self.path, self.peer).await
    }

    /// 【niri】【请求交付】发送 request 并解码 reply_variant，返回指定响应；回复尺寸有限且必须以换行结束
    pub(crate) async fn query<T: DeserializeOwned>(
        &mut self,
        request: &Value,
        reply_variant: &str,
    ) -> Result<T> {
        let mut encoded = serde_json::to_vec(request)?;
        encoded.push(b'\n');
        self.reader.get_mut().write_all(&encoded).await?;
        decode(&self.line().await?, reply_variant)
    }

    /// 【niri】【事件接收】读取一个有界换行 JSON 事件，无参数；返回事件值，不开启额外线程
    pub(crate) async fn event(&mut self) -> Result<Value> {
        Ok(serde_json::from_slice(&self.line().await?)?)
    }

    /// 【niri】【有界读取】读取至换行或限制，无参数；断连、过大或未完成回复均返回错误
    async fn line(&mut self) -> Result<Vec<u8>> {
        let mut line = Vec::new();
        loop {
            let buffer = self.reader.fill_buf().await?;
            if buffer.is_empty() {
                return Err("niri sent an incomplete IPC reply".into());
            }
            let end = buffer.iter().position(|byte| *byte == b'\n');
            let count = end.map_or(buffer.len(), |end| end + 1);
            if line.len() as u64 + count as u64 > MAX_REPLY {
                return Err("niri sent an oversized IPC reply".into());
            }
            line.extend_from_slice(&buffer[..count]);
            self.reader.consume(count);
            if end.is_some() {
                return Ok(line);
            }
        }
    }
}

/// 【niri】【响应解码】按 variant 解码 line 中成功响应，服务端错误保持原文；返回指定类型
fn decode<T: DeserializeOwned>(line: &[u8], variant: &str) -> Result<T> {
    let mut reply: Value = serde_json::from_slice(line)?;
    if let Some(error) = reply.get("Err") {
        return Err(format!("niri IPC: {error}").into());
    }
    let response = reply.get_mut("Ok").ok_or("niri IPC reply has no result")?;
    if variant == "Handled" && response == "Handled" {
        return Ok(serde_json::from_value(Value::Null)?);
    }
    let value = response
        .get_mut(variant)
        .ok_or_else(|| format!("unexpected niri IPC response, expected {variant}"))?;
    Ok(serde_json::from_value(value.take())?)
}

#[cfg(test)]
pub(crate) mod tests;
