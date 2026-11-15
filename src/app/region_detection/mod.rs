//! 【截图区域识别】【悬停流程】候选独立于正式选区，完成后台分析后按需唤醒

pub(crate) mod draw;
mod geometry;
#[cfg(any(target_os = "windows", test))]
mod hit_path;
pub(crate) mod input;
pub(crate) mod keyboard;
pub(crate) mod motion;
#[cfg(any(target_os = "windows", test))]
mod native;
mod state;
pub(crate) mod task;

pub(crate) use state::State;

#[cfg(test)]
mod tests;
