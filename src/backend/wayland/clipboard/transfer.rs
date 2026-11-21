//! 【Wayland剪贴板】【非阻塞传输】处理接收端背压和期限，窗口关闭后仍提供完整数据

use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use smithay_client_toolkit::data_device_manager::WritePipe;
use std::{
    io::{self, Write},
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) const MAX_TRANSFERS: usize = 8;

pub(super) struct Outgoing {
    pub pipe: WritePipe,
    bytes: Arc<[u8]>,
    offset: usize,
    pub deadline: Instant,
}

impl Outgoing {
    /// 【Wayland剪贴板】【传输创建】按 pipe 和 bytes 创建共享字节发送器，返回系统结果，不复制完整图像
    pub fn new(pipe: WritePipe, bytes: Arc<[u8]>) -> io::Result<Self> {
        fcntl_setfl(&pipe, fcntl_getfl(&pipe)? | OFlags::NONBLOCK)?;
        Ok(Self {
            pipe,
            bytes,
            offset: 0,
            deadline: Instant::now() + Duration::from_secs(30),
        })
    }

    /// 【Wayland剪贴板】【传输推进】无参数，发送当前可写字节，返回是否结束；管道背压等待下一次就绪事件
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
                Err(error) => {
                    log::error!("【Wayland剪贴板】【数据发送】{error}");
                    return true;
                }
            }
        }
        true
    }
}
