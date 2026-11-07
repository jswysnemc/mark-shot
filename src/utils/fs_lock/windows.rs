//! 【Windows文件】【进程锁】系统独占文件锁自动回收，等待有明确截止时间

use std::{
    fs::File,
    io,
    ops::{Deref, DerefMut},
    time::{Duration, Instant},
};

pub(crate) struct Lock(File);
impl Deref for Lock {
    type Target = File;
    /// 【Windows文件】【只读借用】无参数，返回持锁文件引用，不转移关闭责任
    fn deref(&self) -> &File {
        &self.0
    }
}
impl DerefMut for Lock {
    /// 【Windows文件】【写入借用】无参数，返回持锁文件可变引用，不提前释放系统锁
    fn deref_mut(&mut self) -> &mut File {
        &mut self.0
    }
}
impl Drop for Lock {
    /// 【Windows文件】【锁释放】无参数，先解除本次独占锁再关闭文件，无返回值
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

/// 【Windows文件】【获取锁】在timeout内独占file，返回自动解除的文件锁，不创建长期工作线程
pub(crate) fn acquire(file: File, timeout: Duration) -> io::Result<Lock> {
    let deadline = Instant::now() + timeout;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Lock(file)),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(std::fs::TryLockError::Error(error)) => return Err(error),
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "file is busy"));
        }
        std::thread::sleep(
            Duration::from_millis(8).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}
