//! 【Wayland剪贴板】【快捷授权】无界面捕获缺少数据控制时，在捕获完成后取得短期窗口焦点

use super::WaylandOverlay;
use crate::{backend::ScreenOverlay, types::OverlayEvent};
use std::time::{Duration, Instant};

impl WaylandOverlay {
    /// 【Wayland剪贴板】【截图交接】无参数，关闭截图资源并延迟驱动选择服务，返回关闭结果；传输结束后退出
    pub(super) fn retain_clipboard(mut self: Box<Self>) -> crate::AppResult {
        let Some(core) = self
            .runtime
            .state
            .clipboard
            .clone()
            .filter(|core| core.borrow().active())
        else {
            return Ok(());
        };
        // 1. 【Wayland剪贴板】【资源释放】销毁截图表面及通知源，释放单实例控制器，保留数据连接
        self.suspend_surfaces()?;
        self.controller = None;
        self.theme_source = None;
        self.task_source = None;
        // 2. 【Wayland剪贴板】【服务延迟】外层先释放任务运行时与单实例锁，再继续处理来源事件
        crate::backend::wayland::clipboard::background::defer(move || {
            while core.borrow().active() {
                self.next_event(None)?;
            }
            Ok(())
        });
        Ok(())
    }
    /// 【Wayland剪贴板】【窗口准备】无参数，映射透明授权表面并等待真实焦点，返回结果；不支持数据控制的会话才调用
    pub(super) fn prepare_clipboard_window(&mut self) -> crate::AppResult {
        if self.runtime.state.clipboard.is_none() {
            return Ok(());
        }
        if !self.runtime.state.surfaces.is_empty() {
            return Ok(());
        }
        // 1. 【Wayland剪贴板】【捕获后授权】截图已经完成，短期表面只用于合法焦点，不进入截图内容
        self.configure_image_window((1, 1), (1, 1));
        self.present(&[0])?;
        let surface = self
            .runtime
            .state
            .surfaces
            .get(&0)
            .ok_or("clipboard authorization surface is unavailable")?;
        let (width, height) = crate::types::px_size((surface.width, surface.height), surface.scale);
        let pixels = tiny_skia::Pixmap::new(width, height)
            .ok_or("cannot allocate clipboard authorization surface")?;
        self.stage_frame(0, pixels.data(), None)?;
        self.flush()?;
        // 2. 【Wayland剪贴板】【焦点确认】使用合成器产生的进入序列，焦点请求超时必须报告失败
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if matches!(
                self.next_event(Some(deadline.saturating_duration_since(Instant::now())))?,
                OverlayEvent::Focus { .. }
            ) {
                return Ok(());
            }
        }
        Err("compositor did not grant focus for clipboard copy".into())
    }
}
