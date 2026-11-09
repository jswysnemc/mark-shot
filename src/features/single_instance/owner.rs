//! 【单实例】【占用与转交】文件锁覆盖整个截图生命周期，重复启动只向同一合成器会话交付命令

use super::{
    Controller,
    request::{MAX_PACKET, Plan, Request, Response},
    server::Server,
    session_path::SessionPath,
};
use nix::{
    errno::Errno,
    fcntl::{Flock, FlockArg},
    sys::socket::{getsockopt, sockopt::PeerCredentials},
};
use std::{
    fs::File,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

pub(crate) enum Entry {
    Owner(Owner),
    Forwarded,
}

pub(crate) struct Owner {
    pub controller: Arc<Controller>,
    _guard: Flock<File>,
}

impl Owner {
    /// 【单实例】【截图入口】按 connection、request 与初始 plan 占用当前会话，返回所属实例或已完成转交，不创建表面
    pub async fn start(
        connection: &crate::backend::session::Session,
        request: Request,
        plan: Plan,
    ) -> crate::AppResult<Entry> {
        Self::start_at(SessionPath::for_connection(connection)?, request, plan).await
    }

    /// 【单实例】【私有占用】按 path、request 与 plan 取得文件锁或向已有实例转交，返回绑定及通信错误，整个转交限定一秒
    pub(super) async fn start_at(
        path: SessionPath,
        request: Request,
        plan: Plan,
    ) -> crate::AppResult<Entry> {
        request.plan(Instant::now()).map_err(str::to_owned)?;
        let mut file = path.lock_file()?;
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
                Ok(guard) => {
                    // 1. 【单实例】【旧路径恢复】只有已取得锁且内核拒绝连接的私有套接字才能删除
                    if let Some(identity) = path.socket_identity()? {
                        if let Some(stream) = connect(&path, deadline).await? {
                            exchange(stream, &request, path.uid, deadline).await?;
                            return Ok(Entry::Forwarded);
                        }
                        path.remove_socket(identity);
                    }
                    if !request.capture {
                        return Err("no running capture instance".into());
                    }
                    let server = Server::new(path)?;
                    return Ok(Entry::Owner(Self {
                        controller: Arc::new(Controller::new(server, plan)),
                        _guard: guard,
                    }));
                }
                Err((returned, Errno::EWOULDBLOCK)) => {
                    file = returned;
                    // 2. 【单实例】【启动竞争】已有实例可能尚未完成绑定，短暂等待后再次取得锁或交付，不另开覆盖窗口
                    if path.socket_identity()?.is_some()
                        && let Some(stream) = connect(&path, deadline).await?
                    {
                        exchange(stream, &request, path.uid, deadline).await?;
                        return Ok(Entry::Forwarded);
                    }
                    if Instant::now() >= deadline {
                        return Err("running capture instance did not become ready".into());
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err((_, error)) => return Err(error.into()),
            }
        }
    }
}

impl Drop for Owner {
    /// 【单实例】【占用释放】先关闭本次控制服务再由文件锁释放占用，无参数、无返回值，不保留后台进程
    fn drop(&mut self) {
        self.controller.close();
    }
}

/// 【单实例】【有限连接】按 path 与总 deadline 连接已有服务，失效路径返回空值，权限或期限错误返回失败
async fn connect(path: &SessionPath, deadline: Instant) -> crate::AppResult<Option<UnixStream>> {
    match tokio::time::timeout_at(deadline.into(), UnixStream::connect(&path.socket)).await {
        Ok(Ok(stream)) => Ok(Some(stream)),
        Ok(Err(error))
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) =>
        {
            Ok(None)
        }
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("single-instance connection timed out".into()),
    }
}

/// 【单实例】【命令往返】按 stream、request、uid 与 deadline 核对内核对端并交付一条请求，返回实际确认或有限通信错误
async fn exchange(
    mut stream: UnixStream,
    request: &Request,
    uid: u32,
    deadline: Instant,
) -> crate::AppResult {
    let credentials = getsockopt(&stream, PeerCredentials)?;
    if credentials.uid() != uid || credentials.pid() <= 0 {
        return Err("single-instance service does not belong to this user".into());
    }
    let work = async {
        stream.write_all(&request.encode()?).await?;
        let mut reader = BufReader::with_capacity(512, stream).take((MAX_PACKET + 1) as u64);
        let mut line = Vec::with_capacity(512);
        reader.read_until(b'\n', &mut line).await?;
        if line.len() > MAX_PACKET || !line.ends_with(b"\n") {
            return Err("single-instance service sent an incomplete or oversized reply".into());
        }
        let response: Response = serde_json::from_slice(&line)?;
        if !response.handled {
            return Err(response.message.into());
        }
        Ok(())
    };
    tokio::time::timeout_at(deadline.into(), work)
        .await
        .map_err(|_| "single-instance command acknowledgement timed out")?
}
