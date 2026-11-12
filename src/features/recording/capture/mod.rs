//! 【录制采集】【持续来源】每个会话固定一个输出，像素仅在录制工作线程消费

pub(super) mod pipewire;
mod portal;
mod wayland;
mod x11;

use crate::{
    backend::{self, session::Session},
    features::recording::Target,
    types::Output,
};
use std::time::Duration;

pub(super) struct Frame<'a> {
    pub gpu: Option<&'a shot_media::Surface>,
    pub pixels: &'a [u8],
    pub size: (u32, u32),
    pub stride: usize,
}

pub(super) trait Source {
    /// 【录制采集】【下一帧】等待不超过 timeout，返回是否更新像素，不将静态画面等待视为采集失败
    fn next(&mut self, timeout: Duration) -> Result<bool, String>;
    /// 【录制采集】【当前画面】无参数，借用最近完成的画面，不读取正在写入的捕获缓冲
    fn frame(&self) -> Option<Frame<'_>>;
    /// 【录制采集】【暂停同步】按 paused 停用或恢复采集，恢复时不交付暂停前缓存
    fn pause(&mut self, paused: bool) -> Result<(), String>;
    /// 【录制采集】【路径诊断】无参数，返回当前真实采集方式
    fn name(&self) -> &'static str;
    /// 【录制采集】【兼容切换】无参数，尝试从显存交付切换到 CPU 帧，返回是否支持重建来源
    fn prefer_cpu(&mut self) -> bool {
        false
    }
    /// 【录制采集】【异步等待】无参数，返回是否需在编码期限前推进协议，以免一次帧请求被拆成多个帧周期
    fn asynchronous(&self) -> bool {
        false
    }
}

/// 【录制采集】【来源打开】按 target 与 cursor 建立固定目标的持续采集，返回来源与物理裁剪区域
pub(super) fn open(
    target: &Target,
    cursor: bool,
    commands: &std::sync::mpsc::Receiver<super::Command>,
    media: &shot_media::Media,
    hardware: bool,
) -> Result<Box<dyn Source>, String> {
    let session = Session::connect_to_env().map_err(|e| e.to_string())?;
    match session {
        Session::X11(display) => Ok(Box::new(x11::Capture::new(display, target, cursor)?)),
        Session::Wayland(connection) => {
            let overlay = backend::initialize_overlay(Session::Wayland(connection.clone()))
                .map_err(|e| e.to_string())?;
            let outputs = overlay.discovered_outputs();
            let output = choose(outputs, target)?.clone();
            let force_portal =
                std::env::var("MARK_SHOT_RECORDING_CAPTURE").as_deref() == Ok("portal");
            if !force_portal && wayland::Capture::supported(&connection) {
                Ok(Box::new(wayland::Capture::new(
                    connection, output, cursor, media, hardware,
                )?))
            } else {
                Ok(Box::new(portal::Capture::new(
                    target, output, cursor, commands,
                )?))
            }
        }
    }
}

/// 【录制采集】【输出绑定】按 target 名称在 outputs 中定位来源，默认使用第一个输出，不接受名称失配
fn choose<'a>(outputs: &'a [Output], target: &Target) -> Result<&'a Output, String> {
    outputs
        .iter()
        .enumerate()
        .find(|(index, output)| {
            target.display.is_empty()
                || backend::output_selection::name(output, *index) == target.display
        })
        .map(|(_, output)| output)
        .ok_or_else(|| "录制显示器已移除 / Display removed".into())
}
