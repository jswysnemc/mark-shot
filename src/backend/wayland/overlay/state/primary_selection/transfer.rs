//! 【主选择区】【管道传输】在覆盖层事件循环中读取或发送有界正文，禁止等待自身来源造成死锁

use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use smithay_client_toolkit::data_device_manager::{ReadPipe, WritePipe};
use std::{
    io::{self, Read, Write},
    os::fd::AsFd,
    sync::Arc,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(3);
pub(super) const MAX_TRANSFERS: usize = 8;

/// 【主选择区】【非阻塞设置】为 pipe 保留原有标志并设置非阻塞，返回系统调用结果
fn nonblocking(pipe: &impl AsFd) -> io::Result<()> {
    fcntl_setfl(pipe, fcntl_getfl(pipe)? | OFlags::NONBLOCK)?;
    Ok(())
}

pub(super) struct Incoming {
    pub id: u64,
    pub pipe: ReadPipe,
    bytes: Vec<u8>,
    limit: usize,
    pub deadline: Instant,
}

impl Incoming {
    /// 【主选择区】【接收构造】按 id、pipe 与 limit 返回有界接收器，不立即读取来源
    pub fn new(id: u64, pipe: ReadPipe, limit: usize) -> io::Result<Self> {
        nonblocking(&pipe)?;
        Ok(Self {
            id,
            pipe,
            bytes: Vec::new(),
            limit,
            deadline: Instant::now() + TIMEOUT,
        })
    }

    /// 【主选择区】【接收进度】读取当前可用字节，返回未完成或完成正文，错误及超时明确返回空正文
    pub fn progress(&mut self) -> Option<Option<String>> {
        if Instant::now() >= self.deadline {
            return Some(None);
        }
        let mut buffer = [0; 4096];
        loop {
            let remaining = self.limit.saturating_sub(self.bytes.len());
            if remaining == 0 {
                return Some(Self::text(&self.bytes));
            }
            let count = remaining.min(buffer.len());
            match self.pipe.read(&mut buffer[..count]) {
                Ok(0) => return Some(std::str::from_utf8(&self.bytes).ok().map(str::to_owned)),
                Ok(count) => self.bytes.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return None,
                Err(_) => return Some(None),
            }
        }
    }

    /// 【主选择区】【正文解码】按 bytes 返回 UTF-8 正文，截断末尾未完成字符，拒绝中间非法编码
    fn text(bytes: &[u8]) -> Option<String> {
        match std::str::from_utf8(bytes) {
            Ok(text) => Some(text.to_owned()),
            Err(error) if error.error_len().is_none() => {
                std::str::from_utf8(&bytes[..error.valid_up_to()])
                    .ok()
                    .map(str::to_owned)
            }
            Err(_) => None,
        }
    }
}

pub(super) struct Outgoing {
    pub pipe: WritePipe,
    bytes: Arc<[u8]>,
    offset: usize,
    pub deadline: Instant,
}

impl Outgoing {
    /// 【主选择区】【发送构造】按 pipe 与 bytes 返回共享正文的发送器，不新增线程或复制整份数据
    pub fn new(pipe: WritePipe, bytes: Arc<[u8]>) -> io::Result<Self> {
        nonblocking(&pipe)?;
        Ok(Self {
            pipe,
            bytes,
            offset: 0,
            deadline: Instant::now() + TIMEOUT,
        })
    }

    /// 【主选择区】【发送进度】发送当前管道可接收的字节，返回是否完成或终止，阻塞时等待下一次就绪通知
    pub fn progress(&mut self) -> bool {
        if Instant::now() >= self.deadline {
            return true;
        }
        while self.offset < self.bytes.len() {
            match self.pipe.write(&self.bytes[self.offset..]) {
                Ok(0) => return true,
                Ok(count) => self.offset += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return false,
                Err(_) => return true,
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustix::pipe::{PipeFlags, pipe_with};

    /// 【主选择区测试】【分段接收】空管道立即返回未完成，分段写入后等待 EOF，结果与原文一致
    #[test]
    fn incoming_waits_without_blocking_and_finishes_on_eof() {
        let (read, write) = pipe_with(PipeFlags::CLOEXEC).unwrap();
        let mut writer = std::fs::File::from(write);
        let mut incoming = Incoming::new(7, read.into(), 64).unwrap();
        assert_eq!(incoming.progress(), None);
        writer.write_all(b"12").unwrap();
        assert_eq!(incoming.progress(), None);
        writer.write_all(b"34").unwrap();
        drop(writer);
        assert_eq!(incoming.progress(), Some(Some("1234".into())));
    }

    /// 【主选择区测试】【有界正文】过大来源只保留允许字节，超时和非法编码明确失败，末尾部分字符安全截断
    #[test]
    fn incoming_limits_data_and_rejects_invalid_or_expired_sources() {
        let (read, write) = pipe_with(PipeFlags::CLOEXEC).unwrap();
        let mut writer = std::fs::File::from(write);
        writer.write_all("12中34".as_bytes()).unwrap();
        let mut incoming = Incoming::new(1, read.into(), 4).unwrap();
        assert_eq!(incoming.progress(), Some(Some("12".into())));
        incoming.deadline = Instant::now();
        assert_eq!(incoming.progress(), Some(None));
        assert_eq!(Incoming::text(b"1\xff2"), None);
    }

    /// 【主选择区测试】【非法末尾】来源自身包含不完整 UTF-8 字符时整批拒绝，不把非法字符删除后插入数字前缀
    #[test]
    fn incoming_rejects_incomplete_utf8_at_source_eof() {
        let (read, write) = pipe_with(PipeFlags::CLOEXEC).unwrap();
        let mut writer = std::fs::File::from(write);
        writer.write_all(b"12\xf0").unwrap();
        drop(writer);
        let mut incoming = Incoming::new(2, read.into(), 64).unwrap();
        assert_eq!(incoming.progress(), Some(None));
    }

    /// 【主选择区测试】【管道背压】大正文发送遇满管道立即返回，读取后继续传输，EOF 与完整字节一致
    #[test]
    fn outgoing_resumes_under_backpressure_and_closes_at_completion() {
        let (read, write) = pipe_with(PipeFlags::CLOEXEC).unwrap();
        let mut reader = std::fs::File::from(read);
        nonblocking(&reader).unwrap();
        let expected: Arc<[u8]> = vec![b'9'; 131072].into();
        let mut outgoing = Outgoing::new(write.into(), expected.clone()).unwrap();
        assert!(!outgoing.progress());
        let mut actual = Vec::new();
        loop {
            let mut buffer = [0; 8192];
            while let Ok(count) = reader.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                actual.extend_from_slice(&buffer[..count]);
            }
            if outgoing.progress() {
                break;
            }
        }
        drop(outgoing);
        // 1. 【主选择区测试】【结束等待】并行创建的进程可能短暂持有写入描述符，非阻塞读取必须在期限内等待实际EOF
        let deadline = Instant::now() + TIMEOUT;
        let mut buffer = [0; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => actual.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "sender pipe did not reach EOF");
                    std::thread::yield_now();
                }
                Err(error) => panic!("sender pipe read failed: {error}"),
            }
        }
        assert_eq!(actual, &*expected);
    }
}
