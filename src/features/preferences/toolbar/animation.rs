//! 【工具栏设置】【避让动画】以短时插值移动按钮，静止时不安排周期唤醒

use crate::config::toolbar_layout::Profile;
use std::time::{Duration, Instant};
use tiny_skia::Rect;

const DURATION: Duration = Duration::from_millis(150);
type Position = (Profile, usize, Rect);

#[derive(Default)]
pub(in crate::features::preferences) struct Animation {
    start: Option<Instant>,
    from: Vec<Position>,
    to: Vec<Position>,
}

impl Animation {
    /// 【工具栏设置】【动画续接】从 old 当前画面过渡到 next，返回空值，目标未变时保持原时间线
    pub fn retarget(&mut self, old: &[Position], next: Vec<Position>) {
        if self.to == next {
            return;
        }
        self.from = old
            .iter()
            .map(|(profile, index, area)| {
                (*profile, *index, self.position(*profile, *index, *area))
            })
            .collect();
        self.to = next;
        self.start = Some(Instant::now());
    }

    /// 【工具栏设置】【动画位置】按 profile、index 查找插值矩形，无记录时返回 fallback
    pub fn position(&self, profile: Profile, index: usize, fallback: Rect) -> Rect {
        let Some(start) = self.start else {
            return fallback;
        };
        let Some((_, _, target)) = self
            .to
            .iter()
            .find(|entry| entry.0 == profile && entry.1 == index)
        else {
            return fallback;
        };
        let origin = self
            .from
            .iter()
            .find(|entry| entry.0 == profile && entry.1 == index)
            .map_or(*target, |entry| entry.2);
        let elapsed = (start.elapsed().as_secs_f32() / DURATION.as_secs_f32()).min(1.0);
        let fraction = 1.0 - (1.0 - elapsed).powi(3);
        Rect::from_xywh(
            origin.left() + (target.left() - origin.left()) * fraction,
            origin.top() + (target.top() - origin.top()) * fraction,
            target.width(),
            target.height(),
        )
        .unwrap()
    }

    /// 【工具栏设置】【动画期限】无参数，返回下一帧期限；动画结束后取消定时唤醒
    pub fn deadline(&self) -> Option<Instant> {
        self.start
            .filter(|start| start.elapsed() < DURATION)
            .map(|_| Instant::now() + Duration::from_millis(16))
    }
}
