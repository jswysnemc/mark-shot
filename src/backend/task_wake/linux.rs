//! 【后台任务】【覆盖层唤醒】按需创建非阻塞通知管道，完成事件与 Wayland 连接共同等待

use std::{
    io::{self, Read, Write},
    os::fd::{AsFd, BorrowedFd},
    os::unix::net::UnixStream,
    sync::Arc,
};

#[derive(Clone)]
pub struct Wake(Arc<UnixStream>);
pub struct Source {
    reader: UnixStream,
    wake: Wake,
}

impl Wake {
    /// 【后台任务】【完成通知】发送一次合并通知，无参数、无返回值；通知缓冲已满表示事件循环已有未消费通知
    pub fn notify(&self) {
        let _ = (&*self.0).write(&[1]);
    }
}

impl Source {
    /// 【后台任务】【按需构造】创建通知源与可复制句柄，无参数，返回系统创建错误，不启动任何线程
    pub fn new() -> io::Result<Self> {
        let (reader, writer) = UnixStream::pair()?;
        reader.set_nonblocking(true)?;
        writer.set_nonblocking(true)?;
        Ok(Self {
            reader,
            wake: Wake(Arc::new(writer)),
        })
    }

    /// 【后台任务】【通知句柄】返回可供工作线程持有的完成通知，无参数，不复制读端
    pub fn wake(&self) -> Wake {
        self.wake.clone()
    }

    /// 【后台任务】【通知消费】清空已到达通知，无参数，返回读取错误；重复通知不会产生持续重绘
    pub fn drain(&self) -> io::Result<()> {
        let mut buffer = [0; 256];
        loop {
            match (&self.reader).read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }
}

impl AsFd for Source {
    /// 【后台任务】【等待接口】返回通知读端的借用描述符，无参数，保持通知源拥有关闭责任
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.reader.as_fd()
    }
}
