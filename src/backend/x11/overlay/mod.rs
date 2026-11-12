//! 【X11覆盖层】【资源状态】每个显示器拥有独立截图窗口，像素交付前保持隐藏

mod events;
mod image_window;
mod popup;
mod presentation;
mod scroll;
mod text_input;

use super::{connection::Display, surface::Surface};
use crate::{
    backend::task_wake,
    types::{Output, OverlayEvent},
};
use std::collections::VecDeque;

pub struct Overlay {
    display: Display,
    outputs: Vec<Output>,
    surfaces: Vec<(usize, Surface)>,
    wake: task_wake::Source,
    pending: VecDeque<OverlayEvent>,
    image_size: Option<(u32, u32)>,
    image_minimum: (u32, u32),
    cursor_output: Option<usize>,
    suspended: bool,
    scrolling: Option<usize>,
    panel_rect: Option<tiny_skia::Rect>,
    panel_hidden: bool,
    repaint_after: Option<std::time::Instant>,
    previous_focus: Option<u32>,
    popup: Option<(u64, Surface)>,
    popup_grab: Option<super::popup_grab::Grab>,
    theme: Option<crate::theme::config_watch::Watch>,
    keyboard: super::input::Keyboard,
    ime: super::input_method::InputMethod,
    text_focus: Option<crate::types::text_input::Focus>,
    text_snapshot: Option<crate::types::text_input::Snapshot>,
}

impl Overlay {
    /// 【X11覆盖层】【未映射构造】按display发现显示器并创建通知源，返回不包含可见窗口的覆盖层
    pub fn new(display: Display) -> crate::AppResult<Self> {
        use x11rb::protocol::xproto::ConnectionExt;
        let previous_focus = display.active_window().ok().or_else(|| {
            display
                .connection
                .get_input_focus()
                .ok()?
                .reply()
                .ok()
                .map(|reply| reply.focus)
        });
        Ok(Self {
            keyboard: super::input::Keyboard::new(&display)?,
            ime: super::input_method::InputMethod::new(&display),
            text_focus: None,
            text_snapshot: None,
            outputs: display.outputs()?,
            display,
            surfaces: Vec::new(),
            wake: task_wake::Source::new()?,
            pending: VecDeque::new(),
            image_size: None,
            image_minimum: (320, 240),
            cursor_output: None,
            suspended: false,
            scrolling: None,
            panel_rect: None,
            panel_hidden: false,
            repaint_after: None,
            previous_focus,
            popup: None,
            popup_grab: None,
            theme: crate::theme::config_watch::Watch::current(),
        })
    }
}
