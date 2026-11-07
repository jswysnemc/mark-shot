//! 【Wayland覆盖层】【共同等待】图形连接、主选择传输、后台通知和单实例请求共用一次阻塞等待

use super::state::OverlayRunTime;
use crate::{backend::task_wake::Source, features::single_instance::Controller};
use rustix::{
    event::{PollFd, PollFlags, poll},
    time::Timespec,
};
use std::{
    os::fd::AsFd,
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) enum Wake {
    Wayland,
    Tick,
    Progress,
}

/// 【Wayland覆盖层】【事件等待】按 runtime、task、controller 与 timeout 等待实际来源，返回输入、任务或传输推进结果
pub(super) fn wait(
    runtime: &mut OverlayRunTime,
    task: Option<&Source>,
    theme: Option<&crate::theme::config_watch::Watch>,
    controller: Option<&Arc<Controller>>,
    timeout: Option<Duration>,
) -> crate::AppResult<Wake> {
    if let Some(controller) = controller {
        controller.progress()?;
    }
    let control = controller.map(|controller| controller.poll_guard());
    let control_deadline = control.as_ref().and_then(|guard| guard.deadline());
    let primary_timeout = runtime.state.primary_selection.timeout();
    let timeout = [
        timeout,
        primary_timeout,
        control_deadline.map(|at| at.saturating_duration_since(Instant::now())),
    ]
    .into_iter()
    .flatten()
    .min();
    let deadline = timeout.map(|duration| Timespec {
        tv_sec: duration.as_secs() as i64,
        tv_nsec: duration.subsec_nanos() as i64,
    });
    let wayland = runtime.event_queue.as_fd();
    let mut descriptors = vec![PollFd::new(&wayland, PollFlags::IN)];
    let task_index = task.map(|source| {
        let index = descriptors.len();
        descriptors.push(PollFd::new(source, PollFlags::IN));
        index
    });
    let theme_index = theme.map(|source| {
        let index = descriptors.len();
        descriptors.push(PollFd::new(source, PollFlags::IN));
        index
    });
    let primary_start = descriptors.len();
    if primary_timeout.is_some() {
        runtime.state.primary_selection.poll_fds(&mut descriptors);
    }
    let control_start = descriptors.len();
    if let Some(control) = &control {
        control.poll_fds(&mut descriptors);
    }
    poll(&mut descriptors, deadline.as_ref())?;
    let wayland_events = descriptors[0].revents();
    let task_events = task_index.map_or(PollFlags::empty(), |index| descriptors[index].revents());
    let theme_ready =
        theme_index.is_some_and(|index| descriptors[index].revents().contains(PollFlags::IN));
    let primary_ready = descriptors[primary_start..control_start]
        .iter()
        .any(|fd| !fd.revents().is_empty());
    let control_ready = descriptors[control_start..]
        .iter()
        .any(|fd| !fd.revents().is_empty());
    // 1. 【Wayland覆盖层】【借用释放】处理来源前释放全部描述符借用，控制服务可以回收已结束连接
    drop(descriptors);
    drop(control);
    if wayland_events.intersects(PollFlags::ERR | PollFlags::HUP) {
        return Err("Wayland connection closed".into());
    }
    if let Some(controller) = controller
        && (control_ready || control_deadline.is_some_and(|at| at <= Instant::now()))
    {
        controller.progress()?;
    }
    // 2. 【Wayland覆盖层】【外部主题】只处理真实配置事件，重复保存不会新增重绘，也不修改标注和导出颜色
    if theme_ready {
        match theme.unwrap().changed() {
            Ok(Some(mode)) => {
                crate::theme::apply_saved(mode);
                runtime
                    .state
                    .events
                    .push_back(crate::types::OverlayEvent::ThemeChanged);
                return Ok(Wake::Progress);
            }
            Ok(_) => {}
            Err(error) => log::error!("【Wayland覆盖层】【主题读取】无法读取保存主题: {error}"),
        }
    }
    // 3. 【Wayland覆盖层】【既有优先级】主选择传输先推进事件队列，后台通知仍由下一次有效等待消费
    if primary_ready {
        runtime
            .state
            .primary_selection
            .progress(&mut runtime.state.events);
        return Ok(Wake::Progress);
    }
    if task_events.contains(PollFlags::IN) {
        task.unwrap().drain()?;
        return Ok(Wake::Tick);
    }
    if wayland_events.contains(PollFlags::IN) {
        return Ok(Wake::Wayland);
    }
    if control_ready {
        return Ok(Wake::Progress);
    }
    Ok(Wake::Tick)
}
