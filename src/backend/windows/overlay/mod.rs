//! 【Windows覆盖层】【资源状态】原生窗口固定在创建线程，冻结来源由共享业务拥有

mod events;
mod image_window;
mod presentation;

use super::{input::Decoder, native};
use crate::{
    backend::task_wake,
    theme::config_watch,
    types::{Output, OverlayEvent},
};
use std::collections::VecDeque;

pub struct Overlay {
    outputs: Vec<Output>,
    surfaces: Vec<(usize, native::Window)>,
    queue: native::EventQueue,
    wake: task_wake::Source,
    theme: Option<config_watch::Watch>,
    pending: VecDeque<OverlayEvent>,
    decoder: Decoder,
    cursor_output: Option<usize>,
    composited: bool,
    staged: Vec<bool>,
    text: Option<crate::types::text_input::Snapshot>,
    focus: Option<crate::types::text_input::Focus>,
    popup: Option<(u64, usize, native::Window)>,
    image_size: Option<(u32, u32)>,
    image_minimum: (u32, u32),
}

impl Overlay {
    /// 【Windows覆盖层】【未映射构造】无参数，发现显示器并创建等待来源，返回不包含屏幕缓冲或映射表面的覆盖层
    pub fn new() -> crate::AppResult<Self> {
        Ok(Self {
            outputs: native::monitors()?.into_iter().map(Output::new).collect(),
            surfaces: Vec::new(),
            queue: Default::default(),
            wake: task_wake::Source::new()?,
            theme: config_watch::Watch::current(),
            pending: VecDeque::new(),
            decoder: Default::default(),
            cursor_output: None,
            composited: true,
            staged: Vec::new(),
            popup: None,
            text: None,
            focus: None,
            image_size: None,
            image_minimum: (320, 240),
        })
    }

    /// 【Windows覆盖层】【表面查询】按index返回当前选定输出窗口，超界返回明确错误，不回退到其他显示器
    fn surface(&mut self, index: usize) -> crate::AppResult<&mut native::Window> {
        self.surfaces
            .get_mut(index)
            .map(|entry| &mut entry.1)
            .ok_or_else(|| "Windows output surface is unavailable".into())
    }
}
