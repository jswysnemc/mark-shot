use crate::profile::{BAND_COUNT, Signature, Weights, bottom_ignore, top_ignore};
use crate::types::NO_MATCH;

#[derive(Clone, Copy)]
pub(crate) struct Window {
    pub minimum: i32,
    pub top: i32,
    pub bottom: i32,
}

impl Window {
    fn crop(self, length: i32, height: i32) -> bool {
        length >= self.minimum + top_ignore(height) + bottom_ignore(height)
    }

    fn usable(self, y: i32, height: i32, crop: bool) -> bool {
        y >= self.top
            && y < height - self.bottom
            && (!crop || (y >= top_ignore(height) && y < height - bottom_ignore(height)))
    }
}

fn textured(cols: &Signature, y: usize, weights: &Weights) -> bool {
    y > 0
        && weights
            .iter()
            .enumerate()
            .any(|(g, &weight)| weight > 0.0 && (cols[y][g] - cols[y - 1][g]).abs() > 1.0)
}

fn trimmed_mean(sums: &[f32; BAND_COUNT], rows: i32, weights: &Weights) -> f32 {
    let (mut sum, mut total, mut worst, mut worst_weight, mut active) = (0.0, 0.0, 0.0, 0.0, 0);
    for g in 0..BAND_COUNT {
        if weights[g] <= 0.0 {
            continue;
        }
        let value = weights[g] * sums[g];
        sum += value;
        total += weights[g];
        active += 1;
        if value > worst {
            worst = value;
            worst_weight = weights[g];
        }
    }
    if active >= 4 {
        sum -= worst;
        total -= worst_weight;
    }
    if rows <= 0 || total <= 0.0 {
        NO_MATCH
    } else {
        sum / (rows as f32 * total)
    }
}

pub(crate) fn frame_diff(
    first: &Signature,
    second: &Signature,
    offset: i32,
    window: Window,
    weights: &Weights,
) -> f32 {
    let (h1, h2) = (first.len() as i32, second.len() as i32);
    let length = if offset >= 0 {
        (h1 - offset).min(h2)
    } else {
        h1.min(h2 + offset)
    };
    if h1 == 0 || h2 == 0 || length < window.minimum {
        return NO_MATCH;
    }
    let crop = window.crop(length, h1.min(h2));
    let (s1, s2) = if offset >= 0 {
        (offset, 0)
    } else {
        (0, -offset)
    };
    let (mut rows, mut textured_rows) = (0, 0);
    let mut sums = [0.0; BAND_COUNT];
    for i in 0..length {
        let (y1, y2) = (s1 + i, s2 + i);
        if !window.usable(y1, h1, crop) || !window.usable(y2, h2, crop) {
            continue;
        }
        rows += 1;
        for (g, sum) in sums.iter_mut().enumerate() {
            *sum += (first[y1 as usize][g] - second[y2 as usize][g]).abs();
        }
        if textured_rows < 8 && textured(second, y2 as usize, weights) {
            textured_rows += 1;
        }
    }
    let minimum = if crop {
        24.max(window.minimum / 2)
    } else {
        window.minimum
    };
    if rows < minimum || textured_rows < 8 {
        NO_MATCH
    } else {
        trimmed_mean(&sums, rows, weights)
    }
}

pub(crate) fn overlap_diff(
    full: &Signature,
    frame: &Signature,
    position: i32,
    window: Window,
    excluded: (i32, i32),
    weights: &Weights,
) -> (f32, i32) {
    let (full_h, frame_h) = (full.len() as i32, frame.len() as i32);
    let top = excluded.0.clamp(0, full_h);
    let bottom = excluded.1.clamp(0, full_h - top);
    let start = top.max(position);
    let end = (full_h - bottom).min(position + frame_h);
    let length = (end - start).max(0);
    if length < window.minimum {
        return (NO_MATCH, length);
    }
    let crop = window.crop(length, frame_h);
    let (mut rows, mut textured_rows) = (0, 0);
    let mut sums = [0.0; BAND_COUNT];
    for row in 0..length {
        let y = start - position + row;
        if !window.usable(y, frame_h, crop) {
            continue;
        }
        rows += 1;
        for (g, sum) in sums.iter_mut().enumerate() {
            *sum += (full[(start + row) as usize][g] - frame[y as usize][g]).abs();
        }
        if textured_rows < 8 && textured(frame, y as usize, weights) {
            textured_rows += 1;
        }
    }
    let minimum = if crop {
        24.max(window.minimum / 2)
    } else {
        window.minimum
    };
    (
        if rows < minimum || textured_rows < 8 {
            NO_MATCH
        } else {
            trimmed_mean(&sums, rows, weights)
        },
        length,
    )
}
