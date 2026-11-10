//! 【翻译测试回归】【本地接口】只监听回环地址，提供固定响应或等待取消，不读取用户凭据

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread::JoinHandle,
    time::Duration,
};

pub(super) struct Server {
    pub endpoint: String,
    pub seen: mpsc::Receiver<String>,
    pub closed: mpsc::Receiver<()>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    /// 【翻译测试回归】【响应启动】按 response 创建一次 HTTP 服务，None 保持连接直到取消或超时，返回测试资源
    pub fn start(response: Option<(u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let (send, seen) = mpsc::channel();
        let (notify, closed) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let mut data = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let length = stream.read(&mut buffer).unwrap();
                assert!(length > 0);
                data.extend_from_slice(&buffer[..length]);
                let text = String::from_utf8_lossy(&data);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length: usize = text[..end]
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse().unwrap())
                        })
                        .unwrap_or(0);
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let _ = send.send(String::from_utf8(data).unwrap());
            if let Some((status, body)) = response {
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            } else {
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).unwrap(), 0);
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
    /// 【翻译测试回归】【服务回收】无参数，仅连接已结束线程，未接入请求不阻塞失败测试退出
    fn drop(&mut self) {
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            self.worker.take().unwrap().join().unwrap();
        }
    }
}
