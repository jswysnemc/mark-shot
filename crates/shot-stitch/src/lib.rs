//! Bidirectional scrolling capture stitcher.

#![forbid(unsafe_code)]

mod adjacent;
mod algorithm;
mod antialiasing;
mod comparison;
mod content_span;
mod edge_occlusion;
pub mod fixed_regions;
mod hover_overlap;
mod image_view;
mod line_matching;
mod long_image;
mod long_rows;
mod matching;
mod motion_mask;
mod overlap_evidence;
mod owned_image;
mod pixel_block;
pub mod profile;
mod recovery_search;
mod row_anchors;
mod seam;
mod session;
mod texture_consistency;
mod types;

pub use image_view::StitchedView;
pub use long_rows::PixelStorage;
pub use owned_image::StitchedImage;
pub use session::Stitcher;
pub use types::{Axis, Config, Edge, Stats, Status, StitchResult};
