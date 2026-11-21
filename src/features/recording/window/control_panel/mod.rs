//! 【录制控制面板】【窗口生命周期】可选常驻小窗口只持有界面状态，关闭后宿主继续录制

mod draw;
mod events;
mod hidden_notice;
mod layout;
mod state;
#[cfg(test)]
mod tests;

use super::{fonts, host};
use crate::{
    backend::window::{self, WindowSpec},
    features::recording::{Status, status_query::Query},
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

/// 【录制控制面板】【状态观察】命令异步交付，按短期限读取本地状态但不重复绘制静止内容
const POLL: Duration = Duration::from_millis(100);

struct Panel {
    snapshot: Box<dyn Fn() -> (bool, Status)>,
    fonts: fonts::Shared,
    model: state::Model,
    query: Query,
    pointer: (f64, f64),
    pressed: Option<layout::Control>,
    deadline: Option<Instant>,
    hidden: Rc<Cell<bool>>,
}

impl Panel {
    /// 【录制控制面板】【内容创建】按 fonts、snapshot 和 hidden 构造界面，返回不拥有录制线程的窗口内容
    fn new(
        fonts: fonts::Shared,
        snapshot: impl Fn() -> (bool, Status) + 'static,
        hidden: Rc<Cell<bool>>,
    ) -> Self {
        let (_, status) = snapshot();
        Self {
            snapshot: Box::new(snapshot),
            fonts,
            model: state::Model::new(status),
            query: Query::default(),
            pointer: (-1.0, -1.0),
            pressed: None,
            deadline: Some(Instant::now()),
            hidden,
        }
    }

    /// 【录制控制面板】【界面语言】无参数，返回是否使用中文
    fn chinese(&self) -> bool {
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese
    }
}

/// 【录制控制面板】【隐藏通知】按 host 与 fonts 显示短暂控制入口提示，原生窗口不可用时退回桌面通知，无返回值
pub(super) fn notify_hidden(host: &host::Shared, fonts: fonts::Shared) {
    if !host.borrow().active() {
        return;
    }
    // 1. 【录制控制面板】【采集隔离】Linux 无法将提示排除于屏幕流，录制期间不创建会进入视频的提示窗口
    if cfg!(target_os = "linux") {
        log::info!("【录制控制面板】【入口保留】面板已隐藏，录制继续，可通过截图界面或托盘控制");
        return;
    }
    if let Err(error) = hidden_notice::run(fonts) {
        log::error!("【录制控制面板】【隐藏提示】{error}");
        crate::backend::notify::send_blocking(
            crate::backend::notify::Notice::RecordingControlsHidden,
        );
    }
}

/// 【录制控制面板】【窗口运行】按 host 与 fonts 显示置顶小面板，返回窗口结果；隐藏和窗口错误不停止宿主
pub(super) fn run(host: host::Shared, fonts: fonts::Shared) -> crate::AppResult {
    let hidden = Rc::new(Cell::new(false));
    let shared = host.clone();
    let panel = Panel::new(
        fonts.clone(),
        move || {
            let host = shared.borrow();
            (host.active(), host.snapshot())
        },
        hidden.clone(),
    );
    let result = window::run(
        WindowSpec {
            title: "Mark Shot — Recording controls".into(),
            app_id: "mark-shot-recording-controls".into(),
            size: layout::SIZE,
            minimum: layout::SIZE,
        },
        panel,
    );
    // 1. 【录制控制面板】【界面隐藏】先销毁面板再发送提示，外层流程始终保留录制宿主
    if hidden.get() && result.is_ok() {
        notify_hidden(&host, fonts);
    }
    result
}
