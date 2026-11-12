//! 【录制控制】【本地命令】配置级文件锁防重入，回环 TCP 与随机令牌隔离其他会话

use super::{Command, Status, backend::Shared};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::SyncSender,
    },
    time::Duration,
};

type Session = Arc<Mutex<Option<(SyncSender<Command>, Shared)>>>;

#[derive(Serialize, Deserialize)]
struct Endpoint {
    address: std::net::SocketAddr,
    token: String,
}

#[derive(Serialize, Deserialize)]
struct Request {
    token: String,
    action: String,
}

pub struct Server {
    endpoint: Endpoint,
    path: PathBuf,
    session: Session,
    stopped: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    _lock: File,
}

/// 【录制控制】【会话目录】无参数，返回当前配置专属 IPC 目录，沿用已有私有目录策略
fn root() -> crate::AppResult<PathBuf> {
    Ok(crate::features::desktop::status::directory(
        &crate::config::preferences::path()?,
    )?)
}

impl Server {
    /// 【录制控制】【建立服务】无参数，持有独占文件锁并启动阻塞命令监听，返回占用或资源错误
    pub fn new() -> crate::AppResult<Self> {
        let root = root()?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("screen-recording.lock"))?;
        lock.try_lock()
            .map_err(|_| "录制窗口已经打开 / Recording window is already open")?;
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let mut random = [0u8; 24];
        getrandom::fill(&mut random).map_err(|error| error.to_string())?;
        let token = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let endpoint = Endpoint {
            address: listener.local_addr()?,
            token: token.clone(),
        };
        let path = root.join("screen-recording.json");
        // 1. 【录制控制】【凭据保存】权限继承私有目录，原子替换旧端点
        let mut temporary = tempfile::NamedTempFile::new_in(&root)?;
        serde_json::to_writer(&mut temporary, &endpoint)?;
        temporary.persist(&path)?;
        let session: Session = Arc::new(Mutex::new(None));
        let current = session.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let shutdown = stopped.clone();
        let thread = std::thread::Builder::new()
            .name("recording-control".into())
            .spawn(move || {
                for connection in listener.incoming() {
                    if shutdown.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(mut stream) = connection else {
                        break;
                    };
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                    let mut bytes = Vec::new();
                    if Read::by_ref(&mut stream)
                        .take(2048)
                        .read_to_end(&mut bytes)
                        .is_err()
                    {
                        continue;
                    }
                    let Ok(request) = serde_json::from_slice::<Request>(&bytes) else {
                        continue;
                    };
                    if request.token != token {
                        continue;
                    }
                    let active = current.lock().unwrap_or_else(|error| error.into_inner());
                    let mut status = Status::default();
                    if let Some((sender, shared)) = active.as_ref() {
                        let command = match request.action.as_str() {
                            "pause" => Some(Command::Pause),
                            "resume" => Some(Command::Resume),
                            "stop" => Some(Command::Stop),
                            _ => None,
                        };
                        status = shared
                            .lock()
                            .unwrap_or_else(|error| error.into_inner())
                            .clone();
                        if let Some(command) = command
                            && let Err(error) = sender.try_send(command)
                        {
                            status.detail =
                                format!("控制请求失败 / Control request failed: {error}");
                        }
                    }
                    status.window_open = true;
                    let _ = serde_json::to_writer(&mut stream, &status);
                }
            })?;
        Ok(Self {
            endpoint,
            path,
            session,
            stopped,
            thread: Some(thread),
            _lock: lock,
        })
    }

    /// 【录制控制】【绑定会话】按 controller 更新唯一活动录制，返回空值
    pub fn attach(&self, controller: &super::controller::Controller) {
        *self.session.lock().unwrap() =
            Some((controller.sender.clone(), controller.status.clone()));
    }
}

impl Drop for Server {
    /// 【录制控制】【关闭服务】无参数，唤醒阻塞监听并回收线程，锁释放前删除端点
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ = TcpStream::connect_timeout(&self.endpoint.address, Duration::from_millis(200));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 【录制控制】【请求会话】按 action 读取本地端点并投递命令，返回当前状态或明确的连接错误
pub fn request(action: &str) -> crate::AppResult<Status> {
    let path = root()?.join("screen-recording.json");
    if !path.exists() && action == "status" {
        return Ok(Status::default());
    }
    let endpoint: Endpoint = serde_json::from_slice(&std::fs::read(path)?)?;
    if !endpoint.address.ip().is_loopback() {
        return Err("invalid recording endpoint".into());
    }
    let mut stream = match TcpStream::connect_timeout(&endpoint.address, Duration::from_secs(1)) {
        Ok(stream) => stream,
        Err(_) if action == "status" => return Ok(Status::default()),
        Err(error) => return Err(error.into()),
    };
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    serde_json::to_writer(
        &mut stream,
        &Request {
            token: endpoint.token,
            action: action.into(),
        },
    )?;
    stream.shutdown(std::net::Shutdown::Write)?;
    Ok(serde_json::from_reader(stream.take(65536))?)
}
