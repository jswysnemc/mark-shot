//! 【录制设置】【准备窗口】分段选项与开关组成的紧凑设置界面，确认开始后立即关闭

mod draw;
mod events;
mod layout;
mod options;

use super::{fonts, host::Shared};
use crate::{
    backend::window::{self, WindowSpec},
    config::recording::Recording,
    features::recording::Target,
};

pub(super) struct Setup {
    host: Shared,
    config: Recording,
    target: Target,
    failure: Option<String>,
    fonts: fonts::Shared,
    icons: crate::renderer::Icons,
    size: (u32, u32),
    pointer: (f64, f64),
    pressed: Option<layout::Control>,
    notice_scroll: usize,
    settings_opened: bool,
}

impl Setup {
    /// 【录制设置】【界面语言】无参数，返回是否使用中文
    fn chinese(&self) -> bool {
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese
    }

    /// 【录制设置】【选项保存】无参数，逐字段写入当前配置并保留其他设置，返回保存错误
    ///
    /// 本次由截图入口指定的格式只影响当前会话，不覆盖用户保存的默认格式
    fn save(&self) -> crate::AppResult {
        let values = serde_json::to_value(&self.config)?;
        let changes = values
            .as_object()
            .ok_or("invalid recording options")?
            .iter()
            .filter(|(key, _)| *key != "format" || self.target.format.is_none())
            .map(|(key, value)| (format!("recording.{key}"), value.clone()))
            .collect::<Vec<_>>();
        let path = crate::config::preferences::path()?;
        crate::config::settings_store::write(&path, &changes)?;
        crate::config::preferences::reload(&path)?;
        Ok(())
    }

    /// 【录制设置】【配置读取】无参数，从当前偏好重新生成选项，本次指定的格式继续生效
    fn reload(&mut self) {
        self.config = crate::config::preferences::current().recording.clone();
        if let Some(format) = self.target.format {
            self.config.format = format.resolve(&self.config.format);
        }
    }
}

/// 【录制设置】【窗口启动】按 host、fonts 与 failure 打开准备窗口并阻塞到关闭，返回平台窗口错误
///
/// 参数：host 录制会话宿主，窗口关闭后由调用方领取开始请求；fonts 与倒计时小组件共享的字库；
/// failure 上一轮录制的失败详情，存在时显示在提示区并可复制
pub(super) fn run(host: Shared, fonts: fonts::Shared, failure: Option<String>) -> crate::AppResult {
    let icons = [crate::ui::icons::CLOSE, crate::ui::icons::VIDEO]
        .into_iter()
        .filter_map(|svg| {
            usvg::Tree::from_str(svg, &usvg::Options::default())
                .ok()
                .map(|tree| (svg, tree))
        })
        .collect();
    let target = host.borrow().target().clone();
    let mut setup = Setup {
        host,
        config: Recording::default(),
        target,
        failure,
        fonts,
        icons,
        size: layout::SIZE,
        pointer: (-1.0, -1.0),
        pressed: None,
        notice_scroll: 0,
        settings_opened: false,
    };
    setup.reload();
    window::run(
        WindowSpec {
            title: "Mark Shot — Recording".into(),
            app_id: "mark-shot-recording".into(),
            size: layout::SIZE,
            minimum: layout::SIZE,
        },
        setup,
    )
}
