//! 【录制窗口】【紧凑控制】复用原生窗口与统一控件，不保留视频预览帧

mod draw;
mod events;
mod layout;

use super::{Status, Target, controller::Controller, ipc::Server};
use crate::{
    backend::window::{self, WindowSpec},
    config::recording::Recording,
    ui::fonts::UiFonts,
};

pub(super) struct Recorder {
    config: Recording,
    target: Target,
    controller: Option<Controller>,
    server: Server,
    fonts: UiFonts,
    cache: cosmic_text::SwashCache,
    icons: crate::renderer::Icons,
    size: (u32, u32),
    pointer: (f64, f64),
    pressed: Option<layout::Control>,
    status: Status,
    close_after_stop: bool,
    error_scroll: usize,
    settings_opened: bool,
    deadline: Option<std::time::Instant>,
}

impl Recorder {
    /// 【录制窗口】【界面语言】无参数，返回是否使用中文
    fn chinese(&self) -> bool {
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese
    }

    /// 【录制窗口】【活动判断】无参数，返回设备或导出线程是否仍在工作
    fn active(&self) -> bool {
        self.controller
            .as_ref()
            .is_some_and(|controller| !controller.finished())
    }

    /// 【录制窗口】【选项保存】无参数，逐字段写入当前配置，保留其他设置，返回保存错误
    fn save(&self) -> crate::AppResult {
        let values = serde_json::to_value(&self.config)?;
        let changes = values
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (format!("recording.{key}"), value.clone()))
            .collect::<Vec<_>>();
        let path = crate::config::preferences::path()?;
        crate::config::settings_store::write(&path, &changes)?;
        crate::config::preferences::reload(&path)?;
        Ok(())
    }
}

/// 【录制窗口】【启动】按 target 创建独立窗口，返回配置、独占锁或平台窗口错误
pub(super) fn run(target: Target) -> crate::AppResult {
    let icons = [crate::ui::icons::CLOSE, crate::ui::icons::VIDEO]
        .into_iter()
        .filter_map(|svg| {
            usvg::Tree::from_str(svg, &usvg::Options::default())
                .ok()
                .map(|tree| (svg, tree))
        })
        .collect();
    let recorder = Recorder {
        config: crate::config::preferences::current().recording.clone(),
        target,
        controller: None,
        server: Server::new()?,
        fonts: UiFonts::with_system_family("Sans Serif"),
        cache: cosmic_text::SwashCache::new(),
        icons,
        size: (560, 420),
        pointer: (-1.0, -1.0),
        pressed: None,
        status: Status::default(),
        close_after_stop: false,
        error_scroll: 0,
        settings_opened: false,
        deadline: None,
    };
    window::run(
        WindowSpec {
            title: "Mark Shot — Recording".into(),
            app_id: "mark-shot-recording".into(),
            size: recorder.size,
            minimum: (480, 420),
        },
        recorder,
    )
}
