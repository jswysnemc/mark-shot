//! 【录制倒计时】【刷新计划】只按剩余时间计算显示数字、唤醒与结束时刻，不依赖窗口或录制会话

use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(super) struct Schedule {
    until: Instant,
}

impl Schedule {
    /// 【录制倒计时】【计划创建】按 until 记录倒计时结束的时刻，返回刷新计划
    pub(super) fn new(until: Instant) -> Self {
        Self { until }
    }

    /// 【录制倒计时】【剩余秒数】按 now 返回向上取整的剩余秒数，最小为一
    pub(super) fn seconds(self, now: Instant) -> u64 {
        let remaining = self.until.saturating_duration_since(now);
        remaining.as_millis().div_ceil(1000).max(1) as u64
    }

    /// 【录制倒计时】【到期判断】按 now 返回倒计时是否已经结束
    pub(super) fn expired(self, now: Instant) -> bool {
        now >= self.until
    }

    /// 【录制倒计时】【下次唤醒】按 now 返回下一次数字变化或结束的时刻，不做周期轮询
    pub(super) fn next_wake(self, now: Instant) -> Instant {
        let remaining = self.until.saturating_duration_since(now);
        let partial = Duration::from_millis((remaining.as_millis() % 1000) as u64 + 5);
        (now + partial).min(self.until)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【录制倒计时测试】【刷新节奏】剩余秒数向上取整，唤醒对齐到数字变化且不晚于结束时刻
    #[test]
    fn wakes_follow_second_boundaries_until_the_end() {
        let now = Instant::now();
        let schedule = Schedule::new(now + Duration::from_millis(2400));
        assert_eq!(schedule.seconds(now), 3);
        assert_eq!(schedule.seconds(now + Duration::from_millis(500)), 2);
        assert_eq!(schedule.seconds(now + Duration::from_secs(9)), 1);
        let wake = schedule.next_wake(now);
        assert!(wake > now && wake <= now + Duration::from_millis(410));
        assert!(!schedule.expired(now + Duration::from_millis(2399)));
        assert!(schedule.expired(now + Duration::from_millis(2400)));
        assert_eq!(
            schedule.next_wake(now + Duration::from_millis(2399)),
            now + Duration::from_millis(2400)
        );
    }
}
