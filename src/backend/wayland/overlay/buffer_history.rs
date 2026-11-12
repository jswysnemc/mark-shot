//! 【截图覆盖层】【缓冲差异】累计备用帧尚未获得的损坏，空闲回收后必须完整恢复

use crate::types::DamageRect;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct History {
    pub(crate) missing: Option<DamageRect>,
    deadline: Option<Instant>,
}

impl History {
    /// 【截图覆盖层】【变化登记】按 damage、swapped 和 now 更新备用帧缺少的区域，无返回值；交换后旧当前帧仅缺少本次变化
    pub(crate) fn changed(&mut self, damage: Option<DamageRect>, swapped: bool, now: Instant) {
        self.missing = if swapped {
            damage
        } else {
            self.missing.zip(damage).map(|(first, second)| {
                let x = first.0.min(second.0);
                let y = first.1.min(second.1);
                let right = first
                    .0
                    .saturating_add(first.2)
                    .max(second.0.saturating_add(second.2));
                let bottom = first
                    .1
                    .saturating_add(first.3)
                    .max(second.1.saturating_add(second.3));
                (x, y, right - x, bottom - y)
            })
        };
        self.deadline = Some(now + Duration::from_millis(250));
    }

    /// 【截图覆盖层】【空闲期限】按 now 返回等待备用帧回收的剩余时间，没有待处理期限时返回空值
    pub(crate) fn timeout(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    /// 【截图覆盖层】【回收判定】按 now 判断是否到期，无返回副作用，不把帧回调视为缓冲释放
    pub(crate) fn due(&self, now: Instant) -> bool {
        self.deadline.is_some_and(|deadline| now >= deadline)
    }

    /// 【截图覆盖层】【回收结束】按 reclaimed 记录回收结果，返回值为空；失败不重复唤醒，成功后丢弃旧差异范围
    pub(crate) fn reclaimed(&mut self, reclaimed: bool) {
        self.deadline = None;
        if reclaimed {
            self.missing = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【缓冲差异测试】【连续变化】无参数，验证交换后只复制一次变化、原地更新累计范围及完整帧失效
    #[test]
    fn changes_accumulate_until_swapping_and_full_frames_invalidate_the_whole_spare() {
        let now = Instant::now();
        let mut history = History::default();
        history.changed(Some((10, 20, 30, 40)), false, now);
        assert_eq!(history.missing, None);
        history.changed(Some((10, 20, 30, 40)), true, now);
        assert_eq!(history.missing, Some((10, 20, 30, 40)));
        history.changed(Some((50, 5, 10, 20)), false, now);
        assert_eq!(history.missing, Some((10, 5, 50, 55)));
        history.changed(Some((90, 80, 3, 5)), true, now);
        assert_eq!(history.missing, Some((90, 80, 3, 5)));
        history.changed(None, false, now);
        assert_eq!(history.missing, None);
    }

    /// 【缓冲差异测试】【空闲续期】无参数，验证连续输入延后回收，失败停止重复唤醒，成功使备用帧完整失效
    #[test]
    fn reclaiming_invalidates_pixels_and_waits_until_input_is_idle() {
        let now = Instant::now();
        let mut history = History::default();
        history.changed(Some((10, 20, 30, 40)), true, now);
        assert!(!history.due(now + Duration::from_millis(249)));
        history.changed(
            Some((20, 30, 30, 40)),
            false,
            now + Duration::from_millis(200),
        );
        assert!(!history.due(now + Duration::from_millis(300)));
        assert!(history.due(now + Duration::from_millis(450)));
        history.reclaimed(false);
        assert!(history.timeout(now).is_none());
        assert!(history.missing.is_some());
        history.reclaimed(true);
        assert_eq!(history.missing, None);
    }
}
