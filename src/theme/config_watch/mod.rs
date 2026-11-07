//! 【界面主题】【配置通知】合并原生目录事件，只在已保存主题变化时通知存活界面

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;
#[cfg(test)]
mod tests;

use super::Mode;
use std::{
    cell::Cell,
    io,
    path::{Path, PathBuf},
};

pub(crate) struct Watch {
    source: platform::Source,
    path: PathBuf,
    seen: Cell<Mode>,
    initial: Cell<bool>,
}

impl Watch {
    /// 【界面主题】【来源创建】按 path 和 initial 创建目录通知，返回来源或文件系统错误，不创建线程
    pub(crate) fn new(path: &Path, initial: Mode) -> io::Result<Self> {
        Ok(Self {
            source: platform::Source::new(path)?,
            path: path.to_owned(),
            seen: Cell::new(initial),
            initial: Cell::new(true),
        })
    }

    /// 【界面主题】【当前监听】按当前配置路径创建来源，返回可选监听器；不可用时记录错误并保留手动切换
    pub(crate) fn current() -> Option<Self> {
        let path = crate::config::interface_theme::path()?;
        match Self::new(&path, Mode::current()) {
            Ok(watch) => Some(watch),
            Err(error) => {
                log::error!("【界面主题】【来源创建】无法监听配置目录: {error}");
                None
            }
        }
    }

    /// 【界面主题】【事件推进】合并本来源事件并有界读取配置，返回新的保存主题；重复、无关和没有事件时不通知
    pub(crate) fn changed(&self) -> io::Result<Option<Mode>> {
        let event = self.source.drain()?;
        if !self.initial.replace(false) && !event {
            return Ok(None);
        }
        let mode = crate::config::interface_theme::read_mode(&self.path)?;
        if self.seen.replace(mode) == mode {
            Ok(None)
        } else {
            Ok(Some(mode))
        }
    }
}

#[cfg(target_os = "linux")]
impl std::os::fd::AsFd for Watch {
    /// 【界面主题】【描述符等待】返回原生来源的借用描述符，无参数，来源持有关闭责任
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.source.as_fd()
    }
}

#[cfg(target_os = "windows")]
impl Watch {
    /// 【Windows主题】【等待句柄】无参数，返回原生目录事件值，调用方不得关闭或重新登记
    pub(crate) fn handle(&self) -> isize {
        self.source.handle()
    }
}
