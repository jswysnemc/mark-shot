//! 【设置窗口】【独立入口】复用原生窗口与控件，配置修改逐项保存，下次截图读取最新值

mod catalog;
mod colors;
mod draw;
mod dropdown;
mod editing;
mod events;
mod frame_preview;
mod layout;
mod menus;
mod navigation;
mod recovery;
pub(crate) mod runtime;
mod scrolling;
mod sections;
mod shell;
mod shortcuts;
#[cfg(test)]
mod tests;
mod toolbar;
mod translation;
mod widgets;
mod window;

pub const READY_ARG: &str = "--settings-ready";

/// 【设置窗口】【窗口启动】无参数，启动同构建的独立设置窗口，返回首帧确认结果
pub fn spawn() -> crate::AppResult {
    crate::utils::spawn_self_ready(&[READY_ARG], &[], crate::backend::window::READY_TIMEOUT)
}

/// 【设置窗口】【原生运行】按 ready 决定是否确认首帧，返回窗口运行结果；不初始化截图或识别任务
pub fn run(ready: bool) -> crate::AppResult {
    window::run(ready)
}
