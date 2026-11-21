//! 【原生窗口剪贴板】【关闭交接】释放业务内容和窗口缓冲，保留核心选择直到接管

use super::{WindowAction, WindowContent, WindowEvent, runtime::NativeWindow};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use wayland_client::{Connection, protocol::wl_seat::WlSeat};

struct ClosedContent;

impl WindowContent for ClosedContent {
    /// 【原生窗口剪贴板】【关闭绘制】参数为 canvas 和 scale，关闭状态不绘制，无返回值
    fn draw(&mut self, _: &mut tiny_skia::Pixmap, _: f32) {}

    /// 【原生窗口剪贴板】【关闭输入】参数为 event，关闭状态忽略迟到输入，返回无动作
    fn event(&mut self, _: WindowEvent) -> WindowAction {
        WindowAction::None
    }
}

impl NativeWindow {
    /// 【原生窗口剪贴板】【输入授权】按 seat 和 serial 同步登记本次真实输入，无返回值，事件直接交付无需排队
    pub(super) fn clipboard_input(&mut self, seat: Option<WlSeat>, serial: u32) {
        if let (Some(core), Some(seat)) = (&self.clipboard, seat) {
            let mut core = core.borrow_mut();
            core.input(seat, serial, false);
            core.advance(None);
        }
    }
}

/// 【原生窗口剪贴板】【后台保留】按 state、connection 和 event_loop 释放业务资源并延迟驱动数据传输，返回交接结果
pub(super) fn retain(
    mut state: NativeWindow,
    connection: Connection,
    mut event_loop: EventLoop<'static, NativeWindow>,
) -> crate::AppResult {
    // 1. 【原生窗口剪贴板】【业务释放】替换内容触发任务析构，清空像素和缓冲，主表面提交空缓冲解除映射
    state.content = Box::new(ClosedContent);
    state.main.release_pixels();
    state.pool = crate::backend::wayland::utils::shm_pool::SlotPool::new(4096, &state.shm)?;
    state.main.role.wl_surface().attach(None, 0, 0);
    state.main.role.commit();
    state.text_input.sync(None);
    connection.flush()?;
    let core = state
        .clipboard
        .clone()
        .ok_or("clipboard state is unavailable")?;
    // 2. 【原生窗口剪贴板】【传输驱动】先推进并检查终止，再等待来源事件或有限管道传输
    crate::backend::wayland::clipboard::background::defer(move || {
        while core.borrow().active() {
            core.borrow_mut().progress();
            if !core.borrow().active() {
                break;
            }
            let timeout = core
                .borrow()
                .timeout()
                .map(|_| std::time::Duration::from_millis(5));
            event_loop.dispatch(timeout, &mut state)?;
        }
        drop(state);
        connection.flush()?;
        Ok(())
    });
    Ok(())
}

crate::backend::wayland::clipboard::core_dispatch::clipboard_dispatch!(NativeWindow);
