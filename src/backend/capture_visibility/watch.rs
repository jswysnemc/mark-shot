//! 【截图窗口】【窗口监听】文件事件只唤醒窗口，所有图形操作仍在所属事件线程执行

use notify::Watcher;
use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct Watch {
    directory: PathBuf,
    window: tempfile::NamedTempFile,
    ack: PathBuf,
    _watcher: notify::RecommendedWatcher,
    requested: Vec<PathBuf>,
    confirmed: Vec<PathBuf>,
}

impl Watch {
    /// 【截图窗口】【监听创建】按 wake 登记当前窗口组，返回监听器；窗口关闭时自动注销
    pub fn new(wake: impl Fn() + Send + 'static) -> io::Result<Self> {
        Self::in_directory(&super::directory()?, wake)
    }

    /// 【截图窗口】【指定目录】按 directory 和 wake 建立监听与记录，供隔离测试及当前会话使用
    pub(super) fn in_directory(
        directory: &Path,
        wake: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.as_ref().is_ok_and(|event| {
                    event.paths.iter().any(|path| {
                        path.file_name()
                            .is_some_and(|name| name.to_string_lossy().starts_with("lease-"))
                    })
                }) {
                    wake();
                }
            })
            .map_err(io::Error::other)?;
        watcher
            .watch(directory, notify::RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
        let window = super::record(directory, "window-")?;
        let ack = window.path().with_extension("ack");
        Ok(Self {
            directory: directory.to_owned(),
            window,
            ack,
            _watcher: watcher,
            requested: Vec::new(),
            confirmed: Vec::new(),
        })
    }

    /// 【截图窗口】【隐藏请求】无参数，刷新活跃租约并返回期望显隐；错误向上传递，避免错误确认截图
    pub fn requested(&mut self) -> io::Result<bool> {
        self.requested = super::records(&self.directory, "lease-")?;
        Ok(!self.requested.is_empty())
    }

    /// 【截图窗口】【隐藏确认】无参数，在图形后端完成隐藏并刷新协议后写入确认，不重复写相同状态
    pub fn applied(&mut self) -> io::Result<()> {
        if self.confirmed != self.requested {
            let names: Vec<_> = self
                .requested
                .iter()
                .filter_map(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .collect();
            std::fs::write(&self.ack, serde_json::to_vec(&names)?)?;
            self.confirmed = self.requested.clone();
        }
        Ok(())
    }

    /// 【截图窗口】【异常恢复期限】无参数，仅隐藏期间定期检查租约锁，持有者崩溃也能恢复窗口
    pub fn deadline(&self) -> Option<Instant> {
        (!self.requested.is_empty()).then(|| Instant::now() + Duration::from_millis(100))
    }
}

impl Drop for Watch {
    /// 【截图窗口】【监听注销】无参数，删除确认文件；窗口锁与监听器随后随所有者释放
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ack);
        // 1. 【截图窗口】【关闭确认】记录持有至析构结束，不把已退出窗口作为等待目标
        let _ = self.window.path();
    }
}
