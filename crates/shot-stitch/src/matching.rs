use crate::profile::Signature;
use crate::session::Stitcher;
use crate::types::{Edge, NO_MATCH};
use shot_image::Image;

struct Candidates {
    best: (i32, f32),
    good: (i32, f32),
    distance: i32,
}

impl Candidates {
    fn visit(&mut self, pos: i32, diff: f32, prediction: i32, accept: f32) {
        if diff < self.best.1 {
            self.best = (pos, diff);
        }
        let distance = (pos - prediction).abs();
        if diff <= accept
            && (distance < self.distance || (distance == self.distance && diff < self.good.1))
        {
            self.good = (pos, diff);
            self.distance = distance;
        }
    }
}

impl Stitcher {
    /// 【长图拼接】【历史复核】按 frame、cols 和 prediction 搜索正文位置，返回通过完整评分与像素校验的结果
    pub(crate) fn find_known(
        &self,
        frame: &Image,
        cols: &Signature,
        prediction: i32,
    ) -> (i32, f32) {
        let maximum = self.long.height() - cols.len() as i32;
        if maximum < 0 || cols.is_empty() {
            return (0, NO_MATCH);
        }
        let inside = prediction.clamp(0, maximum);
        let mut candidates = Candidates {
            best: (inside, NO_MATCH),
            good: (inside, NO_MATCH),
            distance: i32::MAX,
        };
        // 1. 【长图拼接】【直接位置】优先核验预测和原锚点，评分通过仍必须具备连续原始像素证据
        let valid = |pos| {
            crate::seam::find(self, frame, pos, Edge::None)
                .is_some_and(|seam| seam.matching_rows >= self.config.min_overlap)
        };
        for pos in [prediction, self.anchor, 0, maximum] {
            let pos = pos.clamp(0, maximum);
            let diff = self.known_diff(cols, pos, (0, 0)).0;
            if diff <= self.config.accept_diff && valid(pos) {
                return (pos, diff);
            }
        }
        // 2. 【长图拼接】【分层搜索】用分散行筛选任意位置，仅对有限候选执行完整重叠校验
        for pos in crate::recovery_search::candidates(
            &self.long.cols,
            cols,
            inside,
            &self.motion.weights,
            (self.fixed.top(), self.fixed.bottom()),
            self.config.accept_diff,
            None,
        ) {
            let diff = self.known_diff(cols, pos, (0, 0)).0;
            if diff <= self.config.accept_diff && valid(pos) {
                candidates.visit(pos, diff, inside, self.config.accept_diff);
            }
        }
        if candidates.good.1 <= self.config.accept_diff {
            candidates.good
        } else {
            candidates.best
        }
    }

    /// 【长图拼接】【边缘恢复】按 frame、cols 和 prediction 筛选可达的边缘位置，返回通过连续原始像素核验的候选
    pub(crate) fn find_edge(&self, frame: &Image, cols: &Signature, prediction: i32) -> (i32, f32) {
        // 1. 【长图拼接】【可达边缘】恢复仅允许揭露新内容且具有最小几何重叠的位置
        let (full, height) = (self.long.height(), cols.len() as i32);
        if full <= 0 || height <= 0 {
            return (0, NO_MATCH);
        }
        let distance = height - self.config.min_overlap;
        let minimum = (-distance).max(self.anchor - distance);
        let maximum = (full - self.config.min_overlap).min(self.anchor + distance);
        if minimum > maximum {
            return (0, NO_MATCH);
        }
        let ranges = [
            (minimum, maximum.min(-1)),
            (minimum.max(full - height + 1), maximum),
        ];
        let mut best = (prediction.clamp(minimum, maximum), NO_MATCH);
        // 2. 【长图拼接】【完整复核】分别筛选首尾候选，以原始像素证据确认真实位置
        for (start, end) in ranges {
            if start > end {
                continue;
            }
            for pos in crate::recovery_search::candidates(
                &self.long.cols,
                cols,
                prediction,
                &self.motion.weights,
                (self.fixed.top(), self.fixed.bottom()),
                self.config.accept_diff,
                Some((start, end)),
            ) {
                if crate::seam::find(self, frame, pos, Edge::None)
                    .is_some_and(|seam| seam.matching_rows >= self.config.min_overlap)
                    && (best.1 == NO_MATCH
                        || (pos - prediction).abs() < (best.0 - prediction).abs())
                {
                    best = (pos, 0.0);
                }
            }
        }
        best
    }
}
