//! 【截图区域识别】【悬停流程】候选独立于正式选区，完成后台分析后按需唤醒

pub(crate) mod draw;
mod geometry;
pub(crate) mod input;
pub(crate) mod motion;
mod state;
pub(crate) mod task;

pub(crate) use state::State;

#[cfg(test)]
mod tests;
