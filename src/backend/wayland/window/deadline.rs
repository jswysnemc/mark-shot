//! 【原生窗口】【单次期限】合并父子内容期限，仅在确实到期时唤醒，空闲窗口继续无限阻塞

use std::time::Instant;

use super::runtime::NativeWindow;

impl NativeWindow {
    /// 【原生窗口】【当前期限】读取当前父子内容的最近期限，无参数；输入更新或关闭子窗口后不沿用旧等待期限
    pub(super) fn next_deadline(&self) -> Option<Instant> {
        earliest(
            self.content.next_deadline(),
            self.dialog
                .as_ref()
                .and_then(|dialog| dialog.content.next_deadline()),
        )
    }
}

/// 【原生窗口】【期限合并】返回 main、child 中最早的存在期限，两者均无期限时返回空值，不创建计时线程
pub(super) fn earliest(main: Option<Instant>, child: Option<Instant>) -> Option<Instant> {
    main.into_iter().chain(child).min()
}

/// 【原生窗口】【到期判定】返回 deadline 是否不晚于 now；没有期限的窗口不会因外部输入获得额外唤醒
pub(super) fn due(deadline: Option<Instant>, now: Instant) -> bool {
    deadline.is_some_and(|deadline| deadline <= now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 【窗口期限测试】【父子合并】缺少一侧期限不隐藏另一侧，父子并存时选择最早期限，无参数、无返回值
    #[test]
    fn earliest_ignores_missing_deadlines_and_keeps_the_earliest_present_one() {
        let now = Instant::now();
        let later = now + Duration::from_secs(2);
        assert_eq!(earliest(None, None), None);
        assert_eq!(earliest(Some(now), None), Some(now));
        assert_eq!(earliest(None, Some(now)), Some(now));
        assert_eq!(earliest(Some(later), Some(now)), Some(now));
        assert_eq!(earliest(Some(now), Some(later)), Some(now));
    }

    /// 【窗口期限测试】【无周期唤醒】未到期或无期限不触发，等于期限时只提供一次到期条件，无参数、无返回值
    #[test]
    fn only_an_existing_expired_deadline_is_due() {
        let now = Instant::now();
        assert!(!due(None, now));
        assert!(!due(Some(now + Duration::from_secs(1)), now));
        assert!(due(Some(now), now));
    }
}
