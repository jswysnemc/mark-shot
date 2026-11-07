//! 【文件操作】【进程锁】有截止时间的文件锁，不建立常驻线程

use std::{
    fs::File,
    io,
    time::{Duration, Instant},
};

use nix::{
    errno::Errno,
    fcntl::{Flock, FlockArg},
};

/// 【文件操作】【获取锁】在 timeout 内独占 file；返回持有锁的文件，释放对象时自动解锁
pub(crate) fn acquire(mut file: File, timeout: Duration) -> io::Result<Flock<File>> {
    let deadline = Instant::now() + timeout;
    loop {
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(lock) => return Ok(lock),
            Err((returned, Errno::EAGAIN | Errno::EINTR)) => file = returned,
            Err((_, error)) => return Err(error.into()),
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "file is busy"));
        }
        std::thread::sleep(
            Duration::from_millis(8).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}
