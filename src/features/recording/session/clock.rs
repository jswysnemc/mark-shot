//! 【录制时钟】【单调时间】视频与音频共用同一基准，暂停时间从两条轨道扣除

use std::time::{Duration, Instant};

pub(super) struct Clock {
    epoch: Instant,
    active_since: Instant,
    paused: Option<Instant>,
    offset: Duration,
}
impl Clock {
    /// 【录制时钟】【基准建立】按 epoch 创建录制时间轴，首帧时刻为零
    pub fn new(epoch: Instant) -> Self {
        Self {
            epoch,
            active_since: epoch,
            paused: None,
            offset: Duration::ZERO,
        }
    }
    /// 【录制时钟】【暂停】按 now 冻结时间，重复暂停不覆盖原时刻
    pub fn pause(&mut self, now: Instant) {
        self.paused.get_or_insert(now);
    }
    /// 【录制时钟】【继续】按 now 扣除暂停区间，重复继续不改变时间
    pub fn resume(&mut self, now: Instant) {
        if let Some(paused) = self.paused.take() {
            self.offset += now.saturating_duration_since(paused);
            self.active_since = now;
        }
    }
    /// 【录制时钟】【状态读取】无参数，返回当前是否暂停
    pub fn paused(&self) -> bool {
        self.paused.is_some()
    }
    /// 【录制时钟】【有效起点】无参数，返回最近恢复时刻，跨边界音频需裁掉此时刻以前的样本
    pub fn active_since(&self) -> Instant {
        self.active_since
    }
    /// 【录制时钟】【媒体时间】按 now 返回扣除暂停后的微秒，启动前的样本返回空值
    pub fn pts(&self, now: Instant) -> Option<i64> {
        if now < self.active_since {
            return None;
        }
        self.paused
            .unwrap_or(now)
            .checked_duration_since(self.epoch)?
            .checked_sub(self.offset)
            .map(|value| value.as_micros().min(i64::MAX as u128) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【录制时钟回归】【暂停连续性】无参数，多次暂停恢复保持音视频同一时间，旧样本不可跨越恢复点
    #[test]
    fn pauses_are_removed_from_both_tracks() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        clock.pause(start + Duration::from_secs(2));
        assert_eq!(clock.pts(start + Duration::from_secs(10)), Some(2_000_000));
        clock.resume(start + Duration::from_secs(5));
        assert_eq!(clock.pts(start + Duration::from_secs(6)), Some(3_000_000));
        assert_eq!(clock.pts(start + Duration::from_secs(1)), None);
        assert_eq!(clock.pts(start + Duration::from_secs(4)), None);
    }
}
