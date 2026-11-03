//! Integer ARGB pixel buffer operations without GUI framework dependencies.

#![forbid(unsafe_code)]

pub mod blur;
pub mod buffer;
pub mod filters;
pub mod pixelate;

pub use buffer::{Image, ImageError, ImageView, MAX_PIXELS};
