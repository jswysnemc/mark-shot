//! 【全局快捷键】【录入互斥】设置录入期间暂停注册，文件锁随窗口退出自动释放

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
};

pub(super) const FILE: &str = "shortcut-recording.lock";

pub struct Pause(File);

impl Pause {
    /// 【全局快捷键】【暂停申请】按config定位当前服务，返回持有录入锁的句柄或占用错误
    pub fn acquire(config: &Path) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(super::status::directory(config)?.join(FILE))?;
        file.try_lock().map_err(io::Error::other)?;
        file.write_all(&[1])?;
        file.flush()?;
        Ok(Self(file))
    }
}

impl Drop for Pause {
    /// 【全局快捷键】【录入结束】无参数，释放锁并通知服务恢复注册，异常退出由系统释放锁
    fn drop(&mut self) {
        let _ = self.0.unlock();
        let _ = self.0.write_all(&[0]);
        let _ = self.0.flush();
    }
}

/// 【全局快捷键】【暂停查询】按root检测录入锁是否仍被窗口持有，返回暂停状态
pub(super) fn active(root: &Path) -> bool {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(FILE))
        .is_ok_and(|file| matches!(file.try_lock(), Err(std::fs::TryLockError::WouldBlock)))
}
