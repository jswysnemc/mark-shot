//! 【区域识别】【功能入口】冻结图像候选、窗口注入和会话悬停层级

mod analysis;
pub mod candidate;
pub mod cursor;
pub mod detector;
mod edges;
mod image;
mod proposals;
mod refinement;
mod scoring;
pub mod window;

pub use candidate::{Candidate, PixelRect, Source};
pub use cursor::Cursor;
pub use detector::Detector;

#[cfg(test)]
mod tests;
