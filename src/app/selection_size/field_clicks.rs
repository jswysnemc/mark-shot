//! 【选区尺寸】【点击序列】平台双击使用原始按钮时间与全局位移，字段三击使用独立墙钟及局部整数坐标

use crate::{features::selection_size::ratio::Field, types::MouseButton};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum Time {
    Native(u32),
    Local(Instant),
}

impl Time {
    /// 【选区尺寸】【点击时间】按 now 和 timestamp 返回原生时间或明确的本地输入时间，不把处理延迟加到原生时间
    fn new(now: Instant, timestamp: Option<u32>) -> Self {
        timestamp.map_or(Self::Local(now), Self::Native)
    }

    /// 【选区尺寸】【按钮间隔】按 previous 返回同一时间来源的间隔，倒退或时间来源变化时不组成双击
    fn since(self, previous: Self) -> Option<Duration> {
        match (self, previous) {
            (Self::Native(now), Self::Native(previous)) => now
                .checked_sub(previous)
                .map(|value| Duration::from_millis(u64::from(value))),
            (Self::Local(now), Self::Local(previous)) => now.checked_duration_since(previous),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct Press {
    time: Time,
    point: (f64, f64),
    button: MouseButton,
}

#[derive(Clone, Copy)]
struct Triple {
    started: Instant,
    point: (f64, f64),
}

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Single,
    Double,
    Triple,
}

#[derive(Default)]
pub(in crate::app) struct Clicks {
    last: Option<Press>,
    double: Option<MouseButton>,
    triples: [Option<Triple>; 2],
}

/// 【选区尺寸】【点击间隔】返回当前配置的毫秒间隔，无参数，平台双击和字段三击使用相同长度及不同时间来源
fn interval() -> Duration {
    Duration::from_millis(crate::config::get().input.double_click_ms)
}

impl Clicks {
    /// 【选区尺寸】【指针位移】按 point 清理超出平台五像素范围的双击候选，返回空值，移动回来不恢复旧候选
    pub fn moved(&mut self, point: (f64, f64)) {
        if self.last.is_some_and(|previous| {
            (point.0 - previous.point.0).abs() > 5.0 || (point.1 - previous.point.1).abs() > 5.0
        }) {
            self.last = None;
        }
    }

    /// 【选区尺寸】【平台按钮】按 button、pressed、point、timestamp 和 now 记录实际按下；返回空值，释放不建立新候选
    pub fn button(
        &mut self,
        button: MouseButton,
        pressed: bool,
        point: (f64, f64),
        timestamp: Option<u32>,
        now: Instant,
    ) {
        if !pressed {
            return;
        }
        let time = Time::new(now, timestamp);
        // 1. 【选区尺寸】【平台双击】时间严格小于间隔，横纵位移允许等于五像素，双击后平台重新开始计数
        let double = self.last.is_some_and(|previous| {
            previous.button == button
                && time
                    .since(previous.time)
                    .is_some_and(|duration| duration < interval())
                && (point.0 - previous.point.0).abs() <= 5.0
                && (point.1 - previous.point.1).abs() <= 5.0
        });
        self.last = (!double).then_some(Press {
            time,
            point,
            button,
        });
        self.double = double.then_some(button);
    }

    /// 【选区尺寸】【字段选择】按 field、point 与 now 返回单击、双击或三击，原字段计时器跨越右键和重新编辑
    pub(super) fn field(
        &mut self,
        field: Field,
        point: (f64, f64),
        now: Instant,
        button: MouseButton,
    ) -> Kind {
        let point = (point.0.round(), point.1.round());
        let triple = &mut self.triples[usize::from(field == Field::Height)];
        // 2. 【选区尺寸】【字段三击】双击启动本字段墙钟计时器，普通按下只比较局部整数坐标的曼哈顿距离
        if self.double == Some(button) {
            if button == MouseButton::Left {
                *triple = Some(Triple {
                    started: now,
                    point,
                });
            }
            return Kind::Double;
        }
        if triple.is_some_and(|previous| {
            now.saturating_duration_since(previous.started) < interval()
                && (point.0 - previous.point.0).abs() + (point.1 - previous.point.1).abs() < 10.0
        }) {
            Kind::Triple
        } else {
            Kind::Single
        }
    }
}
