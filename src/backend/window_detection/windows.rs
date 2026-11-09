//! 【Windows区域识别】【平台入口】冻结可见窗口与后台控件探测共享物理桌面坐标

mod automation;
mod snapshot;
#[cfg(test)]
#[path = "windows/tests.rs"]
mod tests;
mod worker;

pub(super) use snapshot::snapshot;
pub(crate) use worker::Worker;
