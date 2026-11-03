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
        if offset.unsigned_abs() < 12
            || previous.width() != current.width()
            || previous.height() != current.height()
        {
            return Observation::default();
        }
        let header = scan(previous, current, false);
        let footer = scan(previous, current, true);
        self.header.update(header);
        self.footer.update(footer);
        Observation {
            header,
            footer,
            valid: true,
        }
    }
}

fn scan(previous: &Image, current: &Image, bottom: bool) -> i32 {
    let side = side_ignore(current.width());
    let width = current.width() - side * 2;
    let mut count = 0;
    while count < current.height() / 3 {
        let y = if bottom {
            current.height() - 1 - count
        } else {
            count
        };
        if row_diff(previous.row(y), current.row(y), side, width) > ROW_MAX_DIFF {
            break;
        }
        count += 1;
    }
    count
}
