//! 【图片编辑】【原件驻留】连续绘制复用唯一文件映射，空闲后归还干净页，不建立完整图像副本

use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Residency {
    deadline: Option<Instant>,
}

impl Residency {
    /// 【图片编辑】【读取续期】按 now 延迟回收当前文件原件，无返回值；保留时间只覆盖连续输入
    pub(super) fn touched(&mut self, now: Instant) {
        self.deadline = Some(now + Duration::from_millis(250));
    }

    /// 【图片编辑】【空闲唤醒】按 now 返回最迟回收等待时间，无待回收原件时返回空值
    pub(super) fn timeout(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    /// 【图片编辑】【原件回收】按 now 判断空闲期限，到期时归还 source 的只读页，无返回值
    pub(super) fn reclaim_due(
        &mut self,
        source: Option<&crate::image_source::ImageSource>,
        now: Instant,
    ) {
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.deadline = None;
            if let Some(source) = source {
                source.release_pages();
            }
        }
    }
}
