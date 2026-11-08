use crate::comparison::{Window, frame_diff};
use crate::profile::{Signature, Weights};
use crate::session::Stitcher;
use crate::types::NO_MATCH;

struct Search<'a> {
    previous: &'a Signature,
    current: &'a Signature,
    weights: &'a Weights,
    window: Window,
    threshold: f32,
    best: (i32, f32),
    approaches: i32,
}

impl Search<'_> {
    fn probe(&mut self, offset: i32) -> (bool, f32) {
        let diff = frame_diff(
            self.previous,
            self.current,
            offset,
            self.window,
            self.weights,
        );
        if diff < self.best.1 {
            self.best = (offset, diff);
        }
        if self.best.1 < self.threshold {
            self.approaches += 1;
            if self.approaches > 10 || diff < self.threshold / 4.0 {
                return (true, diff);
            }
        }
        (false, diff)
    }
}

fn offsets(maximum: i32, prediction: i32) -> impl Iterator<Item = i32> {
    let prediction = prediction.clamp(-maximum, maximum);
    std::iter::once(prediction).chain((1..=maximum * 2).flat_map(move |delta| {
        [prediction + delta, prediction - delta]
            .into_iter()
            .filter(move |&x| (-maximum..=maximum).contains(&x))
    }))
}

impl Stitcher {
    /// 【滚动截图】【相邻位移】按 cols 与上一帧签名估计位移，返回候选位移和平均差异
    pub(crate) fn find_offset(&self, cols: &Signature) -> (i32, f32) {
        self.offset_in(cols, self.window())
    }

    /// 【滚动截图】【正文位移】按 cols 排除视口两端后重试匹配，返回待原始像素校验的候选
    pub(crate) fn find_inner_offset(&self, cols: &Signature) -> (i32, f32) {
        let mut window = self.window();
        let margin = cols.len() as i32 / 4;
        window.top = window.top.max(margin);
        window.bottom = window.bottom.max(margin);
        self.offset_in(cols, window)
    }

    /// 【滚动截图】【限定匹配】按 cols 和 window 搜索相邻帧位移，返回最佳位置及差异
    fn offset_in(&self, cols: &Signature, window: Window) -> (i32, f32) {
        if self.last_cols.is_empty() || cols.is_empty() {
            return (0, NO_MATCH);
        }
        let maximum = (self.last_cols.len() as i32 - self.config.min_overlap).max(0);
        let zero = frame_diff(&self.last_cols, cols, 0, window, &self.motion.weights);
        let threshold = self.config.approx_diff.min(zero * 0.03);
        let mut search = Search {
            previous: &self.last_cols,
            current: cols,
            weights: &self.motion.weights,
            window,
            threshold,
            best: (0, NO_MATCH),
            approaches: 0,
        };
        let predicted = self.last_offset.clamp(-maximum, maximum);
        // search pixel-by-pixel around predicted offset
        for offset in offsets(maximum, predicted) {
            if (offset - predicted).abs() > 128 {
                break;
            }
            if search.probe(offset).0 {
                return search.best;
            }
        }
        if search.best.1 < threshold {
            return search.best;
        }
        // coarse search with 3-pixel step keeping 3 distinct candidate basins
        let mut basins = [(0, NO_MATCH); 3];
        for offset in (-maximum..=maximum).step_by(3) {
            if (offset - predicted).abs() <= 128 {
                continue;
            }
            let (done, diff) = search.probe(offset);
            if done {
                return search.best;
            }
            if let Some(basin) = basins
                .iter_mut()
                .find(|b| b.1 < NO_MATCH && (b.0 - offset).abs() <= 6)
            {
                if diff < basin.1 {
                    *basin = (offset, diff);
                }
            } else {
                let worst = basins
                    .iter_mut()
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .expect("three basins");
                if diff < worst.1 {
                    *worst = (offset, diff);
                }
            }
        }
        // refine candidate basins pixel-by-pixel
        for &(offset, diff) in &basins {
            if diff >= NO_MATCH {
                continue;
            }
            for candidate in (-maximum).max(offset - 3)..=maximum.min(offset + 3) {
                if search.probe(candidate).0 {
                    return search.best;
                }
            }
        }
        let rival = basins
            .iter()
            .filter(|b| (b.0 - search.best.0).abs() > 6)
            .map(|b| b.1)
            .fold(NO_MATCH, f32::min);
        if search.best.1 >= threshold && search.best.1 > rival * 0.25 {
            for offset in -maximum..=maximum {
                if (offset - predicted).abs() <= 128 || (offset + maximum) % 3 == 0 {
                    continue;
                }
                if search.probe(offset).0 {
                    break;
                }
            }
        }
        search.best
    }
}

#[cfg(test)]
mod tests {
    use super::offsets;

    #[test]
    fn prediction_covers_opposite_edge() {
        let mut values: Vec<_> = offsets(5, 4).collect();
        assert_eq!(values[0], 4);
        values.sort_unstable();
        assert_eq!(values, (-5..=5).collect::<Vec<_>>());
    }
}
