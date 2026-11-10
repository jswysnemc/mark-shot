//! 【截图窗口】【捕获租约】等待自身窗口完成隐藏，成功、失败和取消均释放租约恢复窗口

use std::{
    io::{self, Write},
    path::Path,
    time::{Duration, Instant},
};

pub struct Guard {
    _lease: Option<tempfile::NamedTempFile>,
}

impl Guard {
    /// 【截图窗口】【捕获开始】按 enabled 建立短期隐藏请求，返回守卫；关闭策略时不访问运行目录
    pub fn begin(enabled: bool) -> io::Result<Self> {
        if !enabled {
            return Ok(Self { _lease: None });
        }
        Self::in_directory(&super::directory()?, Duration::from_millis(1500))
    }

    /// 【截图窗口】【确认等待】按 directory 和 timeout 等待存活窗口确认，超时释放租约并返回错误
    pub(super) fn in_directory(directory: &Path, timeout: Duration) -> io::Result<Self> {
        let windows = super::records(directory, "window-")?;
        if windows.is_empty() {
            return Ok(Self { _lease: None });
        }
        let lease = super::record(directory, "lease-")?;
        let name = lease
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        // 1. 【截图窗口】【锁就绪通知】创建事件可能早于文件锁，写入一次以确保窗口看到完整租约
        lease.as_file().write_all(b"capture")?;
        let guard = Self {
            _lease: Some(lease),
        };
        let deadline = Instant::now() + timeout;
        let mut settled = None;
        loop {
            let windows = super::records(directory, "window-")?;
            let ready = windows.iter().all(|window| {
                if !super::active(window) {
                    return true;
                }
                std::fs::read(window.with_extension("ack"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Vec<String>>(&bytes).ok())
                    .is_some_and(|names| names.contains(&name))
            });
            if ready {
                // 2. 【截图窗口】【动画退场】确认来自已 flush 的窗口，等待桌面收起动画结束，避免冻结残影
                let since = settled.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_millis(350) {
                    return Ok(guard);
                }
            } else {
                settled = None;
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "自身窗口未及时完成隐藏，已取消本次截图并恢复窗口",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
