use crate::profile::{
    ALL_BANDS, BAND_COUNT, Signature, Weights, band_columns, bottom_ignore, top_ignore,
};

#[derive(Clone)]
pub(crate) struct MotionMask {
    motion: Weights,
    sampled: bool,
    pub weights: Weights,
    partial: bool,
}

impl Default for MotionMask {
    fn default() -> Self {
        Self {
            motion: [0.0; BAND_COUNT],
            sampled: false,
            weights: ALL_BANDS,
            partial: false,
        }
    }
}

impl MotionMask {
    pub fn observe(&mut self, previous: &Signature, current: &Signature) {
        let height = current.len() as i32;
        if height == 0 || previous.len() != current.len() {
            return;
        }
        let start = top_ignore(height);
        let end = height - bottom_ignore(height);
        if end <= start {
            return;
        }
        let mut motion = [0.0_f32; BAND_COUNT];
        for y in start as usize..end as usize {
            for (g, value) in motion.iter_mut().enumerate() {
                *value += (previous[y][g] - current[y][g]).abs();
            }
        }
        for value in &mut motion {
            *value /= (end - start) as f32;
        }
        let strongest = motion.iter().copied().fold(0.0_f32, f32::max);
        if strongest < 1.0 {
            return;
        }
        for (g, &value) in motion.iter().enumerate() {
            let relative = if value < 1.0 { 0.0 } else { value / strongest };
            self.motion[g] = if self.sampled {
                self.motion[g] * 0.5 + relative * 0.5
            } else {
                relative
            };
        }
        self.sampled = true;
        let weights = self
            .motion
            .map(|value| if value >= 0.10 { 1.0 } else { 0.0 });
        let moving = weights.iter().filter(|&&w| w > 0.0).count();
        self.partial = (3..BAND_COUNT).contains(&moving);
        self.weights = if self.partial { weights } else { ALL_BANDS };
    }

    pub fn moving_span(&self, width: i32) -> (i32, i32) {
        if !self.partial || width <= 0 {
            return (0, width);
        }
        let first = self.weights.iter().position(|&w| w > 0.0).unwrap_or(0);
        let last = self
            .weights
            .iter()
            .rposition(|&w| w > 0.0)
            .unwrap_or(BAND_COUNT - 1);
        let start = band_columns(width, first).0;
        let end = band_columns(width, last).1;
        (start, (end - start + 1).max(1))
    }
}
