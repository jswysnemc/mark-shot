use crate::profile::{Signature, compute};
use crate::session::Stitcher;
use crate::types::{LineMatch, overhang};
use crate::{Axis, Edge, Status, StitchResult};
use shot_image::{Image, ImageError};

#[derive(Clone, Copy)]
struct Placement {
    position: i32,
    offset: i32,
    confidence: f32,
    line: Option<LineMatch>,
}

impl Stitcher {
    /// Pushes a frame into the stitcher, restoring detection state on allocation failure.
    pub fn push(&mut self, raw_frame: Image) -> Result<StitchResult, ImageError> {
        let fixed = self.fixed.clone();
        let motion = self.motion.clone();
        let result = self.push_inner(raw_frame);
        if result.is_err() {
            self.fixed = fixed;
            self.motion = motion;
        }
        result
    }

    fn recover(&self, frame: &Image, cols: &Signature, predicted: i32) -> Option<Placement> {
        let (position, confidence) = self.find_edge(cols, predicted);
        if confidence <= self.config.accept_diff {
            return Some(Placement {
                position,
                offset: position - self.anchor,
                confidence,
                line: None,
            });
        }
        let line = self.find_line_run(frame, predicted);
        if line.diff > self.config.accept_diff || line.rows < self.config.min_overlap {
            return None;
        }
        Some(Placement {
            position: line.position,
            offset: line.position - self.anchor,
            confidence: line.diff,
            line: Some(line),
        })
    }

    fn adopt(&mut self, frame: Image, cols: Signature, position: i32, offset: i32) -> StitchResult {
        let height = frame.height();
        self.anchor = position;
        self.last_offset = offset;
        self.pending_edge = Edge::None;
        self.remember(frame, cols);
        StitchResult {
            status: Status::NoProgress,
            position,
            frame_length: height,
            ..StitchResult::default()
        }
    }

    fn score_edge(
        &self,
        cols: &Signature,
        placement: Placement,
        trim: (i32, i32),
    ) -> (i32, Edge, f32, i32) {
        let (amount, edge) = overhang(
            placement.position,
            cols.len() as i32,
            self.long.height() - trim.1,
        );
        if amount <= 0 {
            return (amount, edge, 0.0, 0);
        }
        if let Some(line) = placement.line.filter(|line| line.edge == edge) {
            return (amount, edge, placement.confidence, line.rows);
        }
        let excluded = if edge == Edge::Start {
            (trim.0, 0)
        } else {
            (0, trim.1)
        };
        let (diff, length) = self.known_diff(cols, placement.position, excluded);
        (amount, edge, diff, length)
    }

    fn push_inner(&mut self, raw: Image) -> Result<StitchResult, ImageError> {
        let frame = if self.axis == Axis::Horizontal {
            raw.transpose()?
        } else {
            raw
        };
        let cols = compute(&frame)?;
        if self.long.height() == 0 {
            return self.initialize(frame, cols);
        }
        if frame.width() != self.long.width() {
            return Ok(StitchResult::default());
        }
        let height = frame.height();
        let full = self.long.height();
        // static frames skip anchor updates and do not report errors
        if cols == self.last_cols {
            self.pending_edge = Edge::None;
            return Ok(StitchResult {
                status: Status::NoProgress,
                position: self.anchor,
                frame_length: height,
                ..StitchResult::default()
            });
        }
        self.motion.observe(&self.last_cols, &cols);
        let (offset, confidence) = self.find_offset(&cols);
        let predicted = self.anchor + offset;
        let mut placement = Placement {
            position: predicted,
            offset,
            confidence,
            line: None,
        };
        let near_edge = self.anchor <= self.config.min_append
            || self.anchor + height >= full - self.config.min_append;
        let disagrees = confidence <= self.config.accept_diff
            && full >= height
            && overhang(predicted, height, full).0 <= 0
            && self.known_diff(&cols, predicted, (0, 0)).0 > self.config.accept_diff;
        // when adjacent frame matching fails, verify edge growth before searching full canvas
        if confidence > self.config.accept_diff || disagrees {
            if let Some(recovered) = near_edge
                .then(|| self.recover(&frame, &cols, predicted))
                .flatten()
            {
                placement = recovered;
            } else {
                if full >= height {
                    let known = self.find_known(&cols, predicted);
                    if known.1 <= self.config.accept_diff {
                        let applied = known.0 - self.anchor;
                        if let Some(last) = &self.last_frame {
                            self.fixed.observe(last, &frame, applied);
                        }
                        return Ok(self.adopt(frame, cols, known.0, applied));
                    }
                }
                let Some(recovered) = self.recover(&frame, &cols, predicted) else {
                    return Ok(StitchResult::default());
                };
                placement = recovered;
            }
        }
        // observe fixed regions only with trusted displacement
        let observation = self
            .last_frame
            .as_ref()
            .map(|last| self.fixed.observe(last, &frame, placement.offset))
            .unwrap_or_default();
        let mut top_trim = 0;
        let mut bottom_trim = 0;
        if let Some(line) = placement.line {
            if line.edge == Edge::Start {
                top_trim = line.trim;
            } else {
                bottom_trim = line.trim;
            }
        } else {
            let (amount, edge) = overhang(placement.position, height, full);
            if amount > 0 && edge == Edge::End {
                let footer = if observation.valid {
                    observation.footer
                } else {
                    self.fixed.bottom()
                };
                let maximum = (full - 1)
                    .min(height / 3)
                    .min(full / 3)
                    .min(full - placement.position)
                    .max(0);
                bottom_trim = footer.clamp(0, maximum);
            } else if amount > 0 && edge == Edge::Start {
                let header = if observation.valid {
                    observation.header
                } else {
                    self.fixed.top()
                };
                top_trim = header.clamp(0, (height - amount).min(full - 1).max(0));
            }
        }
        let (mut amount, mut edge, mut edge_diff, mut overlap) =
            self.score_edge(&cols, placement, (top_trim, bottom_trim));
        let switching =
            self.growth_edge != Edge::None && edge != Edge::None && edge != self.growth_edge;
        let boundary = (edge == Edge::Start
            && self.anchor <= self.config.min_append
            && placement.position <= 0)
            || (edge == Edge::End
                && self.anchor + height >= full - self.config.min_append
                && placement.position + height >= full - bottom_trim);
        let trusted_switch = switching
            && boundary
            && placement.confidence <= self.config.accept_diff
            && placement.offset.abs() >= self.config.min_append
            && overlap >= self.config.min_overlap;
        if amount > 0
            && full >= height
            && !trusted_switch
            && (edge_diff > self.config.accept_diff || overlap < self.config.min_overlap)
        {
            let known = self.find_known(&cols, predicted);
            if known.1 <= self.config.accept_diff {
                placement.position = known.0;
                placement.offset = known.0 - self.anchor;
                placement.line = None;
                top_trim = 0;
                bottom_trim = 0;
                (amount, edge) = overhang(placement.position, height, full);
                edge_diff = 0.0;
                overlap = height;
            }
        }
        let delta = if edge == Edge::End {
            amount - bottom_trim
        } else {
            amount
        };
        // reject jitter and growth with insufficient overlap evidence
        if delta >= self.config.min_append {
            if self.pending_edge != Edge::None && edge != Edge::None && edge != self.pending_edge {
                self.pending_edge = Edge::None;
                return Ok(StitchResult::default());
            }
            let changing =
                self.growth_edge != Edge::None && edge != Edge::None && edge != self.growth_edge;
            let edge_valid =
                edge_diff <= self.config.accept_diff && overlap >= self.config.min_overlap;
            let at_end = if edge == Edge::Start {
                placement.position <= 0
            } else {
                placement.position + height >= full - bottom_trim
            };
            if (changing && !trusted_switch && (!edge_valid || !at_end))
                || (!edge_valid && !trusted_switch)
            {
                return Ok(StitchResult::default());
            }
            if edge == Edge::End {
                self.long.append(&frame, &cols, amount, bottom_trim)?;
                self.anchor = placement.position;
                self.best_bottom_trim = self.best_bottom_trim.max(bottom_trim);
            } else {
                top_trim = top_trim.clamp(0, (height - amount).min(full - 1).max(0));
                self.long
                    .prepend(&frame, &cols, amount + top_trim, top_trim)?;
                self.anchor = placement.position + amount;
                self.best_top_trim = self.best_top_trim.max(top_trim);
            }
            self.axis_locked = true;
            self.pending_edge = Edge::None;
            if edge != Edge::None {
                self.growth_edge = edge;
            }
            self.remember(frame, cols);
            self.last_offset = placement.offset;
            self.stats.frame_count += 1;
            self.stats.total_height = self.long.height();
            self.stats.last_append = delta;
            return Ok(StitchResult {
                status: Status::Appended,
                added: delta,
                edge,
                position: self.anchor,
                frame_length: height,
            });
        }
        if amount == 0 {
            return Ok(self.adopt(frame, cols, placement.position, placement.offset));
        }
        self.pending_edge = edge;
        Ok(StitchResult {
            status: Status::NoProgress,
            edge,
            position: placement.position,
            frame_length: height,
            ..StitchResult::default()
        })
    }
}
