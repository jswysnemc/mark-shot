//! 【普通标注】【旋转接口】保存原始几何，通过独立角度统一控制点、命中与正文变换

pub mod drag;
pub mod geometry;
pub(crate) mod resize;

#[cfg(test)]
mod reference_tests;
