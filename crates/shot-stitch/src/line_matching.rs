use crate::profile::{ROW_MAX_DIFF, row_diff, side_ignore};
use crate::session::Stitcher;
use crate::types::{Edge, LineMatch, overhang};
use shot_image::Image;

fn trims(remembered: i32, detector: i32, detected: i32, floor: i32, maximum: i32) -> Vec<i32> {
    let mut values = vec![0];
    if remembered > 0 {
        values.push(remembered.clamp(0, maximum));
    }
    if detector > 0 {
        values.push(detector.clamp(0, maximum));
    }
    if detected >= floor {
        values.push(detected.clamp(0, maximum));
    }
    values.sort_unstable();
    values.dedup();
    values
}

fn better(candidate: LineMatch, current: LineMatch, prediction: i32) -> bool {
    if candidate.rows != current.rows {
        return candidate.rows > current.rows;
    }
    if (candidate.diff - current.diff).abs() >= 0.001 {
        return candidate.diff < current.diff;
    }
    (candidate.position - prediction).abs() < (current.position - prediction).abs()
}

impl Stitcher {
    pub(crate) fn find_line_run(&self, frame: &Image, prediction: i32) -> LineMatch {
        let mut best = LineMatch::default();
        let (full, height) = (self.long.height(), frame.height());
        if full <= 0 || self.long.width() != frame.width() {
            return best;
        }
        let side = side_ignore(frame.width());
        let (span, span_width) = self.motion.moving_span(frame.width());
        let start = side.max(span);
        let width = (frame.width() - side).min(span + span_width) - start;
        if width <= 0 {
            return best;
        }
        let diff = |full_y: i32, frame_y: i32| {
            row_diff(self.long.row(full_y), frame.row(frame_y), start, width)
        };
        let maximum = (full - 1).min(height / 3).min(full / 3);
        let minimum = self.config.min_overlap;
        let limit = minimum.max(height / 2);
        let floor = 8.max(self.config.min_append / 2);

        // search upward from bottom for consecutive matching rows
        let mut detected_bottom = 0;
        while detected_bottom < maximum
            && diff(full - 1 - detected_bottom, height - 1 - detected_bottom) <= ROW_MAX_DIFF
        {
            detected_bottom += 1;
        }
        for trim in trims(
            self.best_bottom_trim,
            self.fixed.bottom(),
            detected_bottom,
            floor,
            maximum,
        ) {
            let edge_y = full - trim - 1;
            if edge_y < minimum - 1 {
                continue;
            }
            for current_y in (0..height).rev() {
                if best.rows >= limit {
                    break;
                }
                let (mut rows, mut sum) = (0, 0.0);
                while rows < limit && edge_y - rows >= 0 && current_y - rows >= 0 {
                    let value = diff(edge_y - rows, current_y - rows);
                    if value > ROW_MAX_DIFF {
                        break;
                    }
                    sum += value;
                    rows += 1;
                }
                if rows < minimum {
                    continue;
                }
                let position = edge_y - current_y;
                let (amount, edge) = overhang(position, height, full - trim);
                if edge != Edge::End
                    || amount - trim < self.config.min_append
                    || !self.reachable(position, height)
                {
                    continue;
                }
                let candidate = LineMatch {
                    position,
                    diff: sum / rows as f32,
                    trim,
                    rows,
                    edge,
                };
                if better(candidate, best, prediction) {
                    best = candidate;
                }
            }
        }

        // search downward from top for reverse scrolling
        let mut detected_top = 0;
        while detected_top < maximum && diff(detected_top, detected_top) <= ROW_MAX_DIFF {
            detected_top += 1;
        }
        for trim in trims(
            self.best_top_trim,
            self.fixed.top(),
            detected_top,
            floor,
            maximum,
        ) {
            if full - trim < minimum {
                continue;
            }
            for current_y in 0..height {
                if best.rows >= limit {
                    break;
                }
                let (mut rows, mut sum) = (0, 0.0);
                while rows < limit && trim + rows < full && current_y + rows < height {
                    let value = diff(trim + rows, current_y + rows);
                    if value > ROW_MAX_DIFF {
                        break;
                    }
                    sum += value;
                    rows += 1;
                }
                if rows < minimum {
                    continue;
                }
                let position = trim - current_y;
                if position >= 0 || -position < self.config.min_append {
                    continue;
                }
                let (_, edge) = overhang(position, height, full);
                if edge != Edge::Start || !self.reachable(position, height) {
                    continue;
                }
                let candidate = LineMatch {
                    position,
                    diff: sum / rows as f32,
                    trim,
                    rows,
                    edge,
                };
                if better(candidate, best, prediction) {
                    best = candidate;
                }
            }
        }
        best
    }
}
