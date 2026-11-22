use crate::profile::{ROW_MAX_DIFF, row_diff, side_ignore};
use shot_image::Image;

#[derive(Clone, Copy, Debug, Default)]
pub struct Observation {
    pub header: i32,
    pub footer: i32,
    pub valid: bool,
}

#[derive(Clone, Default)]
struct Band {
    committed: i32,
    pending: Option<i32>,
    sampled: bool,
}

impl Band {
    fn update(&mut self, candidate: i32) {
        if !self.sampled || candidate <= self.committed {
            self.committed = candidate;
            self.sampled = true;
            self.pending = None;
        } else if let Some(pending) = self.pending {
            self.committed = pending.min(candidate);
            self.pending = None;
        } else {
            self.pending = Some(candidate);
        }
    }
}

#[derive(Clone, Default)]
pub struct FixedRegions {
    header: Band,
    footer: Band,
}

impl FixedRegions {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn top(&self) -> i32 {
        self.header.committed
    }

    pub fn bottom(&self) -> i32 {
        self.footer.committed
    }

    pub fn observe(&mut self, previous: &Image, current: &Image, offset: i32) -> Observation {
        let side = side_ignore(current.width());
        self.observe_span(
            previous,
            current,
            offset,
            (side, current.width() - side * 2),
        )
    }

    /// 【长图拼接】【固定区域观察】按 previous、current、offset 和运动正文 span 返回固定顶底栏，更新检测状态
    pub(crate) fn observe_span(
        &mut self,
        previous: &Image,
        current: &Image,
        offset: i32,
        span: (i32, i32),
    ) -> Observation {
        if offset.unsigned_abs() < 12
            || previous.width() != current.width()
            || previous.height() != current.height()
        {
            return Observation::default();
        }
        // 1. 【长图拼接】【正文范围】不能裁掉窄正文后只比较空白，否则整幅视口会被误认成固定栏
        let header = scan(previous, current, false, span);
        let footer = scan(previous, current, true, span);
        self.header.update(header);
        self.footer.update(footer);
        Observation {
            header,
            footer,
            valid: true,
        }
    }
}

/// 【长图拼接】【固定边缘扫描】按 previous、current、bottom 和 span 返回从指定边缘连续不变的行数
fn scan(previous: &Image, current: &Image, bottom: bool, span: (i32, i32)) -> i32 {
    let mut count = 0;
    while count < current.height() / 3 {
        let y = if bottom {
            current.height() - 1 - count
        } else {
            count
        };
        if row_diff(previous.row(y), current.row(y), span.0, span.1) > ROW_MAX_DIFF {
            break;
        }
        count += 1;
    }
    count
}
