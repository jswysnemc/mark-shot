//! 【托盘延时】【单次计划】常驻事件循环持有唯一期限，新计划替换旧计划，取消和退出不会留下后台任务

use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Schedule {
    deadline: Option<Instant>,
}

impl Schedule {
    /// 【托盘延时】【计划替换】按 seconds 和 now 保存新的期限，菜单预设以外的值不创建计划
    pub fn replace(&mut self, seconds: u64, now: Instant) {
        if matches!(seconds, 3 | 5 | 10) {
            self.deadline = now.checked_add(Duration::from_secs(seconds));
        }
    }

    /// 【托盘延时】【计划取消】无参数，移除唯一待执行计划，不影响正在编辑的截图
    pub fn cancel(&mut self) {
        self.deadline = None;
    }

    /// 【托盘延时】【待执行状态】无参数，返回是否仍持有计划，供取消菜单启用状态使用
    pub fn pending(&self) -> bool {
        self.deadline.is_some()
    }

    /// 【托盘延时】【一次性取出】按 now 取出已到期计划，返回是否应启动截图，多次调用不会重复触发
    pub fn take_due(&mut self, now: Instant) -> bool {
        if self.deadline.is_some_and(|at| at <= now) {
            self.deadline = None;
            true
        } else {
            false
        }
    }

    /// 【托盘延时】【事件等待】无参数，有计划时等待期限，无计划时保持挂起，取消等待不创建额外任务
    pub async fn wait(&self) {
        match self.deadline {
            Some(at) => tokio::time::sleep_until(at.into()).await,
            None => std::future::pending().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【托盘延时测试】【替换与取消】无参数，旧计划不触发、取消后不触发、正常期限仅触发一次
    #[test]
    fn only_latest_uncancelled_capture_runs_once() {
        let now = Instant::now();
        let mut schedule = Schedule::default();
        schedule.replace(3, now);
        schedule.replace(10, now + Duration::from_secs(1));
        assert!(!schedule.take_due(now + Duration::from_secs(3)));
        assert!(schedule.pending());
        schedule.cancel();
        assert!(!schedule.pending());
        assert!(!schedule.take_due(now + Duration::from_secs(11)));
        schedule.replace(5, now);
        assert!(schedule.take_due(now + Duration::from_secs(5)));
        assert!(!schedule.pending());
        assert!(!schedule.take_due(now + Duration::from_secs(6)));
    }
}
