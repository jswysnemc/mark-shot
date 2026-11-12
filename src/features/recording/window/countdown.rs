//! 【录制倒计时】【置顶小组件】仅在用户设置倒计时时显示，倒计时结束并关闭后才开始采集，不进入录制画面

mod draw;
mod events;
mod schedule;

use super::fonts;
use crate::backend::window::{self, WindowSpec};
use schedule::Schedule;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
use tiny_skia::Rect;

/// 小组件的固定逻辑尺寸
const SIZE: (u32, u32) = (272, 56);

pub(super) struct Countdown {
    fonts: fonts::Shared,
    duration: Duration,
    schedule: Option<Schedule>,
    summary: String,
    pointer: (f64, f64),
    pressed: bool,
    deadline: Option<Instant>,
    completed: Rc<Cell<bool>>,
}

impl Countdown {
    /// 【录制倒计时】【界面语言】无参数，返回是否使用中文
    fn chinese(&self) -> bool {
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese
    }

    /// 【录制倒计时】【取消按钮】无参数，返回取消按钮的范围，绘制与命中共用
    fn cancel_bounds() -> Rect {
        Rect::from_xywh(SIZE.0 as f32 - 72.0, 14.0, 60.0, 28.0).unwrap()
    }

    /// 【录制倒计时】【按钮命中】按 point 返回指针是否位于取消按钮内
    fn over_cancel(point: (f64, f64)) -> bool {
        let area = Self::cancel_bounds();
        point.0 >= f64::from(area.left())
            && point.0 < f64::from(area.right())
            && point.1 >= f64::from(area.top())
            && point.1 < f64::from(area.bottom())
    }

    /// 【录制倒计时】【开始计时】按 now 在首帧绘制后启动倒计时，窗口创建耗时不占用用户看到的秒数
    fn begin(&mut self, now: Instant) {
        if self.schedule.is_none() {
            let schedule = Schedule::new(now + self.duration);
            self.deadline = Some(schedule.next_wake(now));
            self.schedule = Some(schedule);
        }
    }
}

/// 【录制倒计时】【窗口启动】按 fonts、duration 和 summary 显示小组件并阻塞到倒计时结束或取消
///
/// 参数：fonts 与准备窗口共享的字库、duration 倒计时时长、summary 目标与格式摘要；
/// 返回倒计时是否完整结束，用户取消返回 false，平台窗口故障返回错误
pub(super) fn run(
    fonts: fonts::Shared,
    duration: Duration,
    summary: String,
) -> crate::AppResult<bool> {
    let completed = Rc::new(Cell::new(false));
    window::run(
        WindowSpec {
            title: "Mark Shot — Recording countdown".into(),
            app_id: "mark-shot-recording-countdown".into(),
            size: SIZE,
            minimum: SIZE,
        },
        Countdown {
            fonts,
            duration,
            schedule: None,
            summary,
            pointer: (-1.0, -1.0),
            pressed: false,
            deadline: None,
            completed: completed.clone(),
        },
    )?;
    Ok(completed.get())
}
