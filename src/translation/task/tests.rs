//! 【翻译任务测试】【真实资源】本地 TCP 请求验证取消、超时和一次性交付，拒绝依赖用户凭据或外部网络

use super::*;
use crate::translation::source_segments;
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Instant,
};

mod image_tests;
mod process_tests;
mod provider_tests;
mod transport_tests;

struct Server {
    endpoint: String,
    seen: mpsc::Receiver<String>,
    closed: mpsc::Receiver<()>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    /// 【翻译任务测试】【私有服务】按 response 在回环地址启动一次真实 HTTP 服务，空值等待连接取消，返回资源和请求通知
    fn start(response: Option<(u16, &str)>) -> Self {
        let response = response.map(|(status, body)| format!(
            "HTTP/1.1 {status} response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes());
        Self::start_bytes(response)
    }

    /// 【翻译任务测试】【原始 HTTP】按 response 在回环地址启动固定原始回复，返回资源，兼容分块和超限响应的主动断开
    fn start_bytes(response: Option<Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let (send, seen) = mpsc::channel();
        let (notify, closed) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let count = stream.read(&mut buffer).unwrap();
                assert!(
                    count > 0,
                    "request closed before sending its original source"
                );
                bytes.extend_from_slice(&buffer[..count]);
                let text = String::from_utf8_lossy(&bytes);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length: usize = text[..end]
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse().unwrap())
                        })
                        .unwrap_or_default();
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
                assert!(bytes.len() < 1024 * 1024);
            }
            send.send(String::from_utf8(bytes).unwrap()).unwrap();
            if let Some(bytes) = response {
                let _ = stream.write_all(&bytes);
            } else {
                let mut byte = [0];
                assert_eq!(
                    stream.read(&mut byte).unwrap(),
                    0,
                    "cancellation retained the real HTTP connection"
                );
            }
            let _ = notify.send(());
        });
        Self {
            endpoint,
            seen,
            closed,
            worker: Some(worker),
        }
    }
}

impl Drop for Server {
    /// 【翻译任务测试】【服务回收】连接已经处理后回收当前线程，无参数、无返回值，不操作其他进程
    fn drop(&mut self) {
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            self.worker.take().unwrap().join().unwrap();
        }
    }
}

/// 【翻译任务测试】【固定配置】按 server 返回仅含虚构密钥的内置服务配置，不读取进程环境或用户配置
fn config(server: &Server) -> Translation {
    serde_json::from_value(serde_json::json!({"provider":"builtin","apiKey":"private-test-key","apiBase":server.endpoint})).unwrap()
}

/// 【翻译任务测试】【一次结果】在 deadline 前等待 task 当前一次性结果，返回输出或错误，超时直接失败
fn result(task: &mut Task, deadline: Instant) -> Result<Output> {
    while Instant::now() < deadline {
        if let Some(result) = task.take_result() {
            return result;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("translation task did not finish within its actual deadline")
}

/// 【翻译任务测试】【完成线程】等待 task 的工作线程真实退出，无参数之外的任务状态，超过一秒失败
fn finished(task: &Task) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !task.worker.as_ref().unwrap().is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(task.worker.as_ref().unwrap().is_finished());
}

/// 【翻译任务测试】【网络完成】实际 HTTP 收到确认原文，完成只交付一次并退出线程，原文字符串未改变，无参数、无返回值
#[test]
fn actual_http_snapshot_finishes_once_without_a_retained_runtime_thread() {
    let server = Server::start(Some((
        200,
        r#"{"choices":[{"message":{"content":"{\"translations\":[{\"id\":0,\"text\":\"translated\"}]}"}}]}"#,
    )));
    let source = " original ";
    let mut task = Task::start_with(
        &config(&server),
        source,
        "English",
        Path::new("/private/test-config"),
        WindowWake::detached().unwrap(),
        &|_| None,
    )
    .unwrap();
    let request = server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
    assert!(
        request
            .to_lowercase()
            .contains("authorization: bearer private-test-key")
    );
    assert!(request.contains("original"));
    assert_eq!(
        result(&mut task, Instant::now() + Duration::from_secs(2))
            .unwrap()
            .text(),
        "translated"
    );
    assert!(task.take_result().is_none());
    finished(&task);
    assert_eq!(source, " original ");
}

/// 【翻译任务测试】【实际取消】服务已收到请求后取消任务，真实连接关闭且线程退出，重复取消不重复交付，无参数、无返回值
#[test]
fn cancellation_closes_the_real_socket_and_exits_the_worker_promptly() {
    let server = Server::start(None);
    let mut task = Task::start_with(
        &config(&server),
        "source",
        "English",
        Path::new("/private/test-config"),
        WindowWake::detached().unwrap(),
        &|_| None,
    )
    .unwrap();
    server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
    let started = Instant::now();
    task.cancel();
    task.cancel();
    assert_eq!(
        result(&mut task, started + Duration::from_secs(1))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    server.closed.recv_timeout(Duration::from_secs(1)).unwrap();
    finished(&task);
    assert!(started.elapsed() < Duration::from_millis(700));
}

/// 【翻译任务测试】【实际超时】整个请求超过配置期限后关闭连接，错误是超时而非取消，没有晚到成功结果，无参数、无返回值
#[test]
fn timeout_closes_the_real_request_without_late_delivery() {
    let server = Server::start(None);
    let mut config = config(&server);
    config.timeout_ms = 1000;
    let mut task = Task::start_with(
        &config,
        "source",
        "English",
        Path::new("/private/test-config"),
        WindowWake::detached().unwrap(),
        &|_| None,
    )
    .unwrap();
    server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        result(&mut task, Instant::now() + Duration::from_secs(2))
            .unwrap_err()
            .kind,
        ErrorKind::Timeout
    );
    server.closed.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(task.take_result().is_none());
    finished(&task);
}

/// 【翻译任务测试】【上游脱敏】真实 HTTP 错误回显的密钥不会进入任务错误，无参数、无返回值
#[test]
fn actual_http_errors_cannot_echo_configured_credentials() {
    let server = Server::start(Some((401, "echo private-test-key")));
    let mut task = Task::start_with(
        &config(&server),
        "source",
        "English",
        Path::new("/private/test-config"),
        WindowWake::detached().unwrap(),
        &|_| None,
    )
    .unwrap();
    let error = result(&mut task, Instant::now() + Duration::from_secs(2)).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Http);
    assert!(error.detail.contains("[redacted]"));
    assert!(!error.detail.contains("private-test-key"));
    finished(&task);
}

/// 【翻译任务测试】【命令取消】自定义命令优先于缺失服务，取消同时终止独立进程组中的后代，不遗留睡眠工作，无参数、无返回值
#[test]
fn command_priority_and_cancel_reclaim_the_owned_process_group() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("child.pid");
    let config = Translation {
        provider: "plugin:missing".into(),
        command: "sleep 60 & echo $$:$! > {config}; wait".into(),
        ..Default::default()
    };
    let mut task = Task::start_with(
        &config,
        "source",
        "English",
        &pid_file,
        WindowWake::detached().unwrap(),
        &|_| None,
    )
    .unwrap();
    assert_eq!(task.provider, Provider::Command);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !pid_file.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let data = std::fs::read_to_string(&pid_file).unwrap();
    let (leader, child) = data.trim().split_once(':').unwrap();
    let leader = leader.parse::<u32>().unwrap();
    let pid = child.parse::<u32>().unwrap();
    assert!(pid > 1);
    task.cancel();
    assert_eq!(
        result(&mut task, Instant::now() + Duration::from_secs(1))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    finished(&task);
    let deadline = Instant::now() + Duration::from_secs(1);
    let live = || {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .is_some_and(|stat| {
                stat.rsplit(") ")
                    .next()
                    .is_some_and(|tail| !tail.starts_with('Z'))
            })
    };
    while live() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!live(), "canceled command retained its owned child process");
    let deadline = Instant::now() + Duration::from_secs(1);
    while Path::new(&format!("/proc/{leader}")).exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !Path::new(&format!("/proc/{leader}")).exists(),
        "canceled command retained its direct child as a zombie"
    );
}
