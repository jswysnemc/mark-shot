use crate::comparison::{Window, overlap_diff};
use crate::fixed_regions::FixedRegions;
use crate::long_image::LongImage;
use crate::motion_mask::MotionMask;
use crate::profile::{Signature, compute};
use crate::{Axis, Config, Edge, Stats, Status, StitchResult};
use shot_image::{Image, ImageError};

pub struct Stitcher {
    pub(crate) config: Config,
    pub(crate) axis: Axis,
    pub(crate) axis_locked: bool,
    pub(crate) long: LongImage,
    pub(crate) last_frame: Option<Image>,
    pub(crate) last_cols: Signature,
    pub(crate) fixed: FixedRegions,
    pub(crate) motion: MotionMask,
    pub(crate) last_offset: i32,
    pub(crate) best_bottom_trim: i32,
    pub(crate) best_top_trim: i32,
    pub(crate) pending_edge: Edge,
    pub(crate) growth_edge: Edge,
    pub(crate) anchor: i32,
    pub(crate) stats: Stats,
    cache: Option<(u64, Image)>,
}

impl Stitcher {
    pub fn new(config: Config) -> Result<Self, ImageError> {
        if !config.valid() {
            return Err(ImageError::Parameter);
        }
        Ok(Self {
            config,
            axis: Axis::Vertical,
            axis_locked: false,
            long: LongImage::default(),
            last_frame: None,
            last_cols: Vec::new(),
            fixed: FixedRegions::default(),
            motion: MotionMask::default(),
            last_offset: 0,
            best_bottom_trim: 0,
            best_top_trim: 0,
            pending_edge: Edge::None,
            growth_edge: Edge::None,
            anchor: 0,
            stats: Stats::default(),
            cache: None,
        })
    }

    pub fn stats(&self) -> Stats {
        self.stats
    }

    pub fn axis(&self) -> Axis {
        self.axis
    }

    pub fn axis_locked(&self) -> bool {
        self.axis_locked
    }

    pub fn revision(&self) -> u64 {
        self.long.revision
    }

    pub fn dimensions(&self) -> (i32, i32) {
        if self.axis == Axis::Horizontal {
            (self.long.height(), self.long.width())
        } else {
            (self.long.width(), self.long.height())
        }
    }

    /// Switches scrolling axis before initial growth has locked it.
    pub fn set_axis(&mut self, axis: Axis) -> Result<bool, ImageError> {
        if self.axis == axis {
            return Ok(true);
        }
        if self.axis_locked {
            return Ok(false);
        }
        if self.long.height() > 0 {
            let seed = self.long.image()?.transpose()?;
            let cols = compute(&seed)?;
            self.long.reset(&seed, cols.clone())?;
            self.last_frame = Some(seed);
            self.last_cols = cols;
            self.stats.total_height = self.long.height();
            self.stats.last_append = self.long.height();
        }
        self.axis = axis;
        self.pending_edge = Edge::None;
        self.growth_edge = Edge::None;
        self.fixed.reset();
        self.motion = MotionMask::default();
        self.best_bottom_trim = 0;
        self.best_top_trim = 0;
        self.cache = None;
        Ok(true)
    }

    /// Returns the stitched long image, cached by revision number.
    pub fn full_image(&mut self) -> Result<Option<Image>, ImageError> {
        if self.long.height() == 0 {
            return Ok(None);
        }
        if self
            .cache
            .as_ref()
            .is_none_or(|(revision, _)| *revision != self.long.revision)
        {
            let image = self.long.image()?;
            let image = if self.axis == Axis::Horizontal {
                image.transpose()?
            } else {
                image
            };
            self.cache = Some((self.long.revision, image));
        }
        Ok(self.cache.as_ref().map(|(_, image)| image.clone()))
    }

    pub(crate) fn initialize(
        &mut self,
        frame: Image,
        cols: Signature,
    ) -> Result<StitchResult, ImageError> {
        self.long.reset(&frame, cols.clone())?;
        self.fixed.reset();
        self.motion = MotionMask::default();
        self.anchor = 0;
        self.best_bottom_trim = 0;
        self.best_top_trim = 0;
        self.pending_edge = Edge::None;
        self.growth_edge = Edge::None;
        let height = frame.height();
        self.stats = Stats {
            frame_count: 1,
            total_height: height,
            last_append: height,
        };
        self.remember(frame, cols);
        Ok(StitchResult {
            status: Status::FirstFrame,
            added: height,
            edge: Edge::None,
            position: 0,
            frame_length: height,
        })
    }

    pub(crate) fn remember(&mut self, frame: Image, cols: Signature) {
        self.last_frame = Some(frame);
        self.last_cols = cols;
    }

    pub(crate) fn window(&self) -> Window {
        Window {
            minimum: self.config.min_overlap,
            top: self.fixed.top(),
            bottom: self.fixed.bottom(),
        }
    }

    pub(crate) fn known_diff(
        &self,
        cols: &Signature,
        pos: i32,
        excluded: (i32, i32),
    ) -> (f32, i32) {
        overlap_diff(
            &self.long.cols,
            cols,
            pos,
            self.window(),
            excluded,
            &self.motion.weights,
        )
    }

    pub(crate) fn reachable(&self, pos: i32, height: i32) -> bool {
        (pos - self.anchor).abs() <= height - self.config.min_overlap
    }
}
