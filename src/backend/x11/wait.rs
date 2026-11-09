//! 【X11事件】【共同等待】显示连接、后台任务及单实例控制共用阻塞等待

use super::connection::Display;
use crate::backend::task_wake::Source;
use rustix::{
    event::{PollFd, PollFlags, poll},
    time::Timespec,
};
use std::time::{Duration, Instant};

/// 【X11事件】【基础等待】按display、wake与timeout等待真实输入，返回后台任务是否就绪
pub fn wait(display: &Display, wake: &Source, timeout: Option<Duration>) -> crate::AppResult<bool> {
    wait_with_theme(display, wake, timeout, None)
}

/// 【X11事件】【主题通知】按watch消费目录事件，返回保存主题是否改变，失败保留当前模式
pub(crate) fn theme_changed(watch: Option<&crate::theme::config_watch::Watch>) -> bool {
    match watch.map(|watch| watch.changed()) {
        Some(Ok(Some(mode))) => crate::theme::apply_saved(mode),
        Some(Err(error)) => {
            log::error!("【X11主题】【配置读取】{error}");
            false
        }
        _ => false,
    }
}

/// 【X11事件】【主题联合等待】按display、wake、timeout及theme等待输入，不添加空闲轮询
pub(crate) fn wait_with_theme(
    display: &Display,
    wake: &Source,
    timeout: Option<Duration>,
    theme: Option<&crate::theme::config_watch::Watch>,
) -> crate::AppResult<bool> {
    let controller = crate::features::single_instance::context::current();
    if let Some(controller) = &controller {
        controller.progress()?;
    }
    let control = controller
        .as_ref()
        .map(|controller| controller.poll_guard());
    let deadline = control.as_ref().and_then(|control| control.deadline());
    let timeout = [
        timeout,
        deadline.map(|deadline| deadline.saturating_duration_since(Instant::now())),
    ]
    .into_iter()
    .flatten()
    .min();
    let deadline = timeout.map(|timeout| Timespec {
        tv_sec: timeout.as_secs() as i64,
        tv_nsec: timeout.subsec_nanos() as i64,
    });
    let mut descriptors = vec![
        PollFd::new(display.connection.as_ref(), PollFlags::IN),
        PollFd::new(wake, PollFlags::IN),
    ];
    if let Some(control) = &control {
        control.poll_fds(&mut descriptors);
    }
    if let Some(theme) = theme {
        descriptors.push(PollFd::new(theme, PollFlags::IN));
    }
    poll(&mut descriptors, deadline.as_ref())?;
    let connection_closed = descriptors[0]
        .revents()
        .intersects(PollFlags::ERR | PollFlags::HUP);
    let notified = descriptors[1].revents().contains(PollFlags::IN);
    drop(descriptors);
    drop(control);
    if connection_closed {
        return Err("X11 display connection closed".into());
    }
    if notified {
        wake.drain()?;
    }
    if let Some(controller) = controller {
        controller.progress()?;
    }
    Ok(notified)
}
