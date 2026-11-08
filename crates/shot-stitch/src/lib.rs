//! Bidirectional scrolling capture stitcher.

#![forbid(unsafe_code)]

mod adjacent;
mod algorithm;
mod comparison;
mod edge_occlusion;
pub mod fixed_regions;
mod image_view;
mod line_matching;
mod long_image;
mod long_rows;
mod matching;
mod motion_mask;
mod owned_image;
mod pixel_block;
pub mod profile;
mod seam;
mod session;
mod types;

pub use image_view::StitchedView;
pub use long_rows::PixelStorage;
pub use owned_image::StitchedImage;
pub use session::Stitcher;
pub use types::{Axis, Config, Edge, Stats, Status, StitchResult};
